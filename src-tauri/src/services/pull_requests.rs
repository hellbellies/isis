//! Local cache of GitHub pull requests and the sync that fills it.
//!
//! The cache holds open pull requests the user authored or was asked to review,
//! the other open pull requests in the watched repositories (the configured ones,
//! or else those of the user's own pull requests), plus pull requests linked to a
//! workstream (kept, and refreshed by id, after they are merged, closed or no
//! longer listed). Anything else is pruned.
//!
//! Every sync is a full one: the user's open pull requests are few, so two or
//! three GraphQL requests replace the need for delta tracking.

use std::collections::{HashMap, HashSet};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::db::Db;
use crate::error::AppResult;
use crate::models::github::{
    GitHubSyncSummary, PullRequest, PullRequestList, PullRequestState, ReviewDecision,
};
use crate::services::github::{scope_qualifiers, search_scope};
use crate::services::github_client::GitHubClient;

/// Columns read by [`pull_request_from_row`]; `p` is `github_pull_requests`, `w` is `workstream_items`.
const COLUMNS: &str = "p.id, p.number, p.title, p.url, p.repository, p.author, p.state,
     p.is_draft, p.review_decision, p.updated_at, p.authored_by_me, p.review_requested,
     p.other_open, w.workstream_id";
const FROM: &str = "github_pull_requests p LEFT JOIN workstream_items w
     ON w.item_type = 'github_pr' AND w.item_id = p.id";
const INVOLVES_ME: &str = "(p.authored_by_me = 1 OR p.review_requested = 1)";
/// GitHub rejects search queries longer than this.
const MAX_QUERY_LENGTH: usize = 256;

/// Pull requests involving the user and other open ones in the watched
/// repositories, most recently updated first.
pub fn list(conn: &Connection) -> AppResult<PullRequestList> {
    let pull_requests = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM {FROM} WHERE {INVOLVES_ME} OR p.other_open = 1
             ORDER BY p.updated_at DESC"
        ))?
        .query_map([], pull_request_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    let last_synced_at = conn
        .query_row(
            "SELECT last_synced_at FROM github_sync_state WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(PullRequestList {
        pull_requests,
        last_synced_at,
    })
}

/// Pull requests linked to a workstream, whatever their state.
pub fn list_for_workstream(conn: &Connection, workstream_id: i64) -> AppResult<Vec<PullRequest>> {
    Ok(conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM {FROM} WHERE w.workstream_id = ?1 ORDER BY p.updated_at DESC"
        ))?
        .query_map([workstream_id], pull_request_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

/// Cached pull requests whose title or `owner/name#number` contains `term`
/// (ASCII case-insensitive), whatever their state, most recently updated first.
pub fn search(conn: &Connection, term: &str, limit: i64) -> AppResult<Vec<PullRequest>> {
    Ok(conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM {FROM}
             WHERE instr(lower(p.repository || '#' || p.number || ' ' || p.title), lower(?1)) > 0
             ORDER BY p.updated_at DESC LIMIT ?2"
        ))?
        .query_map(params![term, limit], pull_request_from_row)?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn exists(conn: &Connection, id: &str) -> AppResult<bool> {
    Ok(url(conn, id)?.is_some())
}

/// Web URL of a cached pull request.
pub fn url(conn: &Connection, id: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT url FROM github_pull_requests WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Deletes pull requests that are neither listed nor linked to a workstream.
pub fn prune_unlinked(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute(
        "DELETE FROM github_pull_requests
         WHERE authored_by_me = 0 AND review_requested = 0 AND other_open = 0 AND id NOT IN
             (SELECT item_id FROM workstream_items WHERE item_type = 'github_pr')",
        [],
    )?)
}

/// Fetches open pull requests the user authored or must review (optionally limited
/// to `repositories`), the other open ones in the watched repositories, and
/// refreshes workstream-linked ones.
pub async fn sync(
    db: &Db,
    client: &GitHubClient,
    repositories: &[String],
    now_ms: i64,
) -> AppResult<GitHubSyncSummary> {
    let scope = search_scope(repositories);
    let base = format!("is:pr is:open archived:false {scope}");
    let authored = client
        .search_pull_requests(&format!("{base} author:@me"))
        .await?;
    let requested = client
        .search_pull_requests(&format!("{base} review-requested:@me"))
        .await?;

    // One entry per pull request, flagged with every reason it involves the user.
    let mut mine: HashMap<String, PullRequest> = HashMap::new();
    for (pr, is_authored) in authored
        .into_iter()
        .map(|pr| (pr, true))
        .chain(requested.into_iter().map(|pr| (pr, false)))
    {
        let entry = mine.entry(pr.id.clone()).or_insert(pr);
        if is_authored {
            entry.authored_by_me = true;
        } else {
            entry.review_requested = true;
        }
    }

    // Watch the configured repositories, or else the ones the user works in.
    let watched = if repositories.is_empty() {
        let mut repos: Vec<String> = mine
            .values()
            .map(|pr| format!("repo:{}", pr.repository))
            .collect();
        repos.sort();
        repos.dedup();
        repos
    } else {
        scope_qualifiers(repositories)
    };
    let mut other_open: HashMap<String, PullRequest> = HashMap::new();
    let others_base = "is:pr is:open archived:false -author:@me -review-requested:@me";
    for query in batched_queries(others_base, &watched) {
        for mut pr in client.search_pull_requests(&query).await? {
            if !mine.contains_key(&pr.id) {
                pr.other_open = true;
                other_open.insert(pr.id.clone(), pr);
            }
        }
    }

    let linked: Vec<String> = linked_ids(&db.lock())?
        .into_iter()
        .filter(|id| !mine.contains_key(id) && !other_open.contains_key(id))
        .collect();
    let linked = client.pull_requests_by_id(&linked).await?;

    let mine: Vec<PullRequest> = mine.into_values().collect();
    let other_open: Vec<PullRequest> = other_open.into_values().collect();
    apply(&mut db.lock(), &mine, &other_open, &linked, now_ms)
}

/// `base` plus as many `qualifiers` per query as fit in [`MAX_QUERY_LENGTH`]
/// (multiple `repo:`/`user:` qualifiers are ORed). None without qualifiers.
fn batched_queries(base: &str, qualifiers: &[String]) -> Vec<String> {
    let mut queries = Vec::new();
    let mut query = base.to_owned();
    for qualifier in qualifiers {
        if query.len() > base.len() && query.len() + 1 + qualifier.len() > MAX_QUERY_LENGTH {
            queries.push(std::mem::replace(&mut query, base.to_owned()));
        }
        query.push(' ');
        query.push_str(qualifier);
    }
    if query.len() > base.len() {
        queries.push(query);
    }
    queries
}

/// Writes a sync in one transaction, so a failure leaves the previous data intact.
fn apply(
    conn: &mut Connection,
    mine: &[PullRequest],
    other_open: &[PullRequest],
    linked: &[PullRequest],
    now_ms: i64,
) -> AppResult<GitHubSyncSummary> {
    let tx = conn.transaction()?;
    let before = involved_ids(&tx)?;
    // Whatever this sync doesn't list is no longer listed.
    tx.execute(
        "UPDATE github_pull_requests
         SET authored_by_me = 0, review_requested = 0, other_open = 0",
        [],
    )?;
    {
        let mut upsert = tx.prepare(
            "INSERT INTO github_pull_requests
                 (id, number, title, url, repository, author, state, is_draft,
                  review_decision, updated_at, authored_by_me, review_requested, other_open)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                 number = excluded.number, title = excluded.title, url = excluded.url,
                 repository = excluded.repository, author = excluded.author,
                 state = excluded.state, is_draft = excluded.is_draft,
                 review_decision = excluded.review_decision, updated_at = excluded.updated_at,
                 authored_by_me = excluded.authored_by_me,
                 review_requested = excluded.review_requested,
                 other_open = excluded.other_open",
        )?;
        for pr in mine.iter().chain(other_open).chain(linked) {
            upsert.execute(params![
                pr.id,
                pr.number,
                pr.title,
                pr.url,
                pr.repository,
                pr.author,
                state_to_db(pr.state),
                pr.is_draft,
                pr.review_decision.map(review_decision_to_db),
                pr.updated_at,
                pr.authored_by_me,
                pr.review_requested,
                pr.other_open
            ])?;
        }
    }
    prune_unlinked(&tx)?;
    let after = involved_ids(&tx)?;
    tx.execute(
        "INSERT INTO github_sync_state (id, last_synced_at) VALUES (1, ?1)
         ON CONFLICT(id) DO UPDATE SET last_synced_at = excluded.last_synced_at",
        [now_ms],
    )?;
    tx.commit()?;
    Ok(GitHubSyncSummary {
        fetched: mine.len(),
        removed: before.difference(&after).count(),
        others: other_open.len(),
    })
}

fn linked_ids(conn: &Connection) -> AppResult<Vec<String>> {
    Ok(conn
        .prepare("SELECT item_id FROM workstream_items WHERE item_type = 'github_pr'")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?)
}

fn involved_ids(conn: &Connection) -> AppResult<HashSet<String>> {
    Ok(conn
        .prepare(&format!(
            "SELECT id FROM github_pull_requests p WHERE {INVOLVES_ME}"
        ))?
        .query_map([], |row| row.get(0))?
        .collect::<Result<HashSet<_>, _>>()?)
}

fn pull_request_from_row(row: &Row<'_>) -> rusqlite::Result<PullRequest> {
    let state: String = row.get(6)?;
    let review_decision: Option<String> = row.get(8)?;
    Ok(PullRequest {
        id: row.get(0)?,
        number: row.get(1)?,
        title: row.get(2)?,
        url: row.get(3)?,
        repository: row.get(4)?,
        author: row.get(5)?,
        state: state_from_db(&state),
        is_draft: row.get(7)?,
        review_decision: review_decision.as_deref().and_then(review_decision_from_db),
        updated_at: row.get(9)?,
        authored_by_me: row.get(10)?,
        review_requested: row.get(11)?,
        other_open: row.get(12)?,
        workstream_id: row.get(13)?,
    })
}

fn state_to_db(state: PullRequestState) -> &'static str {
    match state {
        PullRequestState::Open => "open",
        PullRequestState::Closed => "closed",
        PullRequestState::Merged => "merged",
    }
}

fn state_from_db(value: &str) -> PullRequestState {
    match value {
        "merged" => PullRequestState::Merged,
        "closed" => PullRequestState::Closed,
        _ => PullRequestState::Open,
    }
}

fn review_decision_to_db(decision: ReviewDecision) -> &'static str {
    match decision {
        ReviewDecision::Approved => "approved",
        ReviewDecision::ChangesRequested => "changes_requested",
        ReviewDecision::ReviewRequired => "review_required",
    }
}

fn review_decision_from_db(value: &str) -> Option<ReviewDecision> {
    match value {
        "approved" => Some(ReviewDecision::Approved),
        "changes_requested" => Some(ReviewDecision::ChangesRequested),
        "review_required" => Some(ReviewDecision::ReviewRequired),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use mockito::{Matcher, Server};

    use super::*;
    use crate::db;
    use crate::models::workstreams::{WorkstreamItemRef, WorkstreamItemType};
    use crate::services::github_client::tests::{client, pr_json, search_page};
    use crate::services::workstreams;

    const NOW: i64 = 1_800_000_000_000;

    /// Mocks the two searches (matched by their `@me` qualifier) and the by-id lookup.
    async fn mock_github(
        server: &mut Server,
        authored: &[String],
        requested: &[String],
        by_id: &[String],
    ) {
        mock_github_with_others(server, authored, requested, &[], by_id).await;
    }

    /// Like [`mock_github`], plus the search for other open pull requests, which
    /// must be scoped to `acme/app` (the repository of every `pr_json`).
    async fn mock_github_with_others(
        server: &mut Server,
        authored: &[String],
        requested: &[String],
        others: &[String],
        by_id: &[String],
    ) {
        server.reset();
        // The user's own searches end with their `@me` qualifier; the other search
        // excludes both (`-author:@me -review-requested:@me`).
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex(r#"[^-]author:@me""#.into()))
            .with_body(search_page(authored, None))
            .create_async()
            .await;
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex(r#"[^-]review-requested:@me""#.into()))
            .with_body(search_page(requested, None))
            .create_async()
            .await;
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex(
                r#"-review-requested:@me repo:acme/app""#.into(),
            ))
            .with_body(search_page(others, None))
            .create_async()
            .await;
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex("nodes\\(ids".into()))
            .with_body(format!(r#"{{"data":{{"nodes":[{}]}}}}"#, by_id.join(",")))
            .create_async()
            .await;
    }

    fn listed_ids(db: &Db) -> Vec<String> {
        let mut ids: Vec<_> = list(&db.lock())
            .unwrap()
            .pull_requests
            .into_iter()
            .map(|pr| pr.id)
            .collect();
        ids.sort();
        ids
    }

    #[tokio::test]
    async fn sync_merges_authored_and_review_requested() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        mock_github(
            &mut server,
            &[pr_json("PR_1", 1, "OPEN"), pr_json("PR_2", 2, "OPEN")],
            &[pr_json("PR_2", 2, "OPEN"), pr_json("PR_3", 3, "OPEN")],
            &[],
        )
        .await;

        let summary = sync(&db, &client(&server.url()), &[], NOW).await.unwrap();
        assert_eq!(summary.fetched, 3);

        let list = list(&db.lock()).unwrap();
        assert_eq!(list.last_synced_at, Some(NOW));
        let pr2 = list
            .pull_requests
            .iter()
            .find(|pr| pr.id == "PR_2")
            .unwrap();
        assert!(pr2.authored_by_me && pr2.review_requested);
        let pr3 = list
            .pull_requests
            .iter()
            .find(|pr| pr.id == "PR_3")
            .unwrap();
        assert!(!pr3.authored_by_me && pr3.review_requested);
    }

    #[tokio::test]
    async fn merged_pull_request_stays_in_its_workstream() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let github = client(&server.url());
        mock_github(
            &mut server,
            &[pr_json("PR_1", 1, "OPEN"), pr_json("PR_2", 2, "OPEN")],
            &[],
            &[],
        )
        .await;
        sync(&db, &github, &[], NOW).await.unwrap();

        let workstream_id = {
            let mut conn = db.lock();
            let workstream = workstreams::create(&conn, "W", NOW).unwrap();
            let item = WorkstreamItemRef {
                item_type: WorkstreamItemType::PullRequest,
                item_id: "PR_1".into(),
            };
            workstreams::set_item_workstream(&mut conn, &item, Some(workstream.id), NOW).unwrap();
            workstream.id
        };

        // Both get merged: no longer in the open-PR searches; PR_1 is refreshed by id.
        mock_github(&mut server, &[], &[], &[pr_json("PR_1", 1, "MERGED")]).await;
        let summary = sync(&db, &github, &[], NOW + 1).await.unwrap();
        assert_eq!(summary.removed, 2);

        assert!(listed_ids(&db).is_empty());
        assert!(!exists(&db.lock(), "PR_2").unwrap());
        let linked = list_for_workstream(&db.lock(), workstream_id).unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].state, PullRequestState::Merged);
    }

    #[tokio::test]
    async fn sync_uses_repository_scope_in_queries() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let scoped = server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex("repo:acme/app user:octo".into()))
            .with_body(search_page(&[], None))
            // Authored, review requested, and the other open pull requests.
            .expect(3)
            .create_async()
            .await;
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex("nodes\\(ids".into()))
            .with_body(r#"{"data":{"nodes":[]}}"#)
            .create_async()
            .await;
        let repos = ["acme/app".to_string(), "octo".to_string()];
        sync(&db, &client(&server.url()), &repos, NOW)
            .await
            .unwrap();
        scoped.assert_async().await;
    }

    #[tokio::test]
    async fn sync_lists_other_open_pull_requests_in_the_users_repositories() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let github = client(&server.url());
        // The other search may still return the user's own pull request; it stays "mine".
        mock_github_with_others(
            &mut server,
            &[pr_json("PR_1", 1, "OPEN")],
            &[],
            &[pr_json("PR_1", 1, "OPEN"), pr_json("PR_9", 9, "OPEN")],
            &[],
        )
        .await;
        let summary = sync(&db, &github, &[], NOW).await.unwrap();
        assert_eq!((summary.fetched, summary.others), (1, 1));

        let list = list(&db.lock()).unwrap().pull_requests;
        let flags = |id: &str| {
            let pr = list.iter().find(|pr| pr.id == id).unwrap();
            (pr.authored_by_me, pr.other_open)
        };
        assert_eq!(flags("PR_1"), (true, false));
        assert_eq!(flags("PR_9"), (false, true));

        // Once it is no longer open (or listed), it is pruned.
        mock_github(&mut server, &[pr_json("PR_1", 1, "OPEN")], &[], &[]).await;
        let summary = sync(&db, &github, &[], NOW + 1).await.unwrap();
        assert_eq!((summary.removed, summary.others), (0, 0));
        assert_eq!(listed_ids(&db), ["PR_1"]);
        assert!(!exists(&db.lock(), "PR_9").unwrap());
    }

    #[test]
    fn batched_queries_keep_each_query_short_enough() {
        let base = "is:pr is:open";
        assert!(batched_queries(base, &[]).is_empty());
        assert_eq!(
            batched_queries(base, &["repo:a/b".into(), "user:c".into()]),
            ["is:pr is:open repo:a/b user:c"]
        );

        let qualifiers: Vec<String> = (0..40)
            .map(|i| format!("repo:acme/repository-{i}"))
            .collect();
        let queries = batched_queries(base, &qualifiers);
        assert!(queries.len() > 1);
        assert!(
            queries
                .iter()
                .all(|q| q.len() <= MAX_QUERY_LENGTH && q.starts_with(base))
        );
        let joined = queries.join(" ");
        assert!(qualifiers.iter().all(|q| joined.contains(q.as_str())));
    }

    #[tokio::test]
    async fn failed_sync_keeps_existing_data() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let github = client(&server.url());
        mock_github(&mut server, &[pr_json("PR_1", 1, "OPEN")], &[], &[]).await;
        sync(&db, &github, &[], NOW).await.unwrap();

        server.reset();
        server
            .mock("POST", "/graphql")
            .with_status(502)
            .create_async()
            .await;
        assert!(sync(&db, &github, &[], NOW + 1).await.is_err());
        assert_eq!(listed_ids(&db), ["PR_1"]);
        assert_eq!(list(&db.lock()).unwrap().last_synced_at, Some(NOW));
    }
}
