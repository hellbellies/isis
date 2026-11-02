//! Local cache of GitHub pull requests and the sync that fills it.
//!
//! The cache holds open pull requests the user authored or was asked to review,
//! plus pull requests linked to a workstream (kept, and refreshed by id, after they
//! are merged, closed or no longer involve the user). Anything else is pruned.
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
use crate::services::github::search_scope;
use crate::services::github_client::GitHubClient;

/// Columns read by [`pull_request_from_row`]; `p` is `github_pull_requests`, `w` is `workstream_items`.
const COLUMNS: &str = "p.id, p.number, p.title, p.url, p.repository, p.author, p.state,
     p.is_draft, p.review_decision, p.updated_at, p.authored_by_me, p.review_requested,
     w.workstream_id";
const FROM: &str = "github_pull_requests p LEFT JOIN workstream_items w
     ON w.item_type = 'github_pr' AND w.item_id = p.id";
const INVOLVES_ME: &str = "(p.authored_by_me = 1 OR p.review_requested = 1)";

/// Pull requests involving the user, most recently updated first.
pub fn list(conn: &Connection) -> AppResult<PullRequestList> {
    let pull_requests = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM {FROM} WHERE {INVOLVES_ME} ORDER BY p.updated_at DESC"
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

/// Deletes pull requests that neither involve the user nor belong to a workstream.
pub fn prune_unlinked(conn: &Connection) -> AppResult<usize> {
    Ok(conn.execute(
        "DELETE FROM github_pull_requests
         WHERE authored_by_me = 0 AND review_requested = 0 AND id NOT IN
             (SELECT item_id FROM workstream_items WHERE item_type = 'github_pr')",
        [],
    )?)
}

/// Fetches open pull requests the user authored or must review (optionally limited
/// to `repositories`) and refreshes workstream-linked ones.
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

    let linked: Vec<String> = linked_ids(&db.lock())?
        .into_iter()
        .filter(|id| !mine.contains_key(id))
        .collect();
    let others = client.pull_requests_by_id(&linked).await?;

    let mine: Vec<PullRequest> = mine.into_values().collect();
    apply(&mut db.lock(), &mine, &others, now_ms)
}

/// Writes a sync in one transaction, so a failure leaves the previous data intact.
fn apply(
    conn: &mut Connection,
    mine: &[PullRequest],
    others: &[PullRequest],
    now_ms: i64,
) -> AppResult<GitHubSyncSummary> {
    let tx = conn.transaction()?;
    let before = involved_ids(&tx)?;
    // Whatever this sync doesn't report as involving the user no longer does.
    tx.execute(
        "UPDATE github_pull_requests SET authored_by_me = 0, review_requested = 0",
        [],
    )?;
    {
        let mut upsert = tx.prepare(
            "INSERT INTO github_pull_requests
                 (id, number, title, url, repository, author, state, is_draft,
                  review_decision, updated_at, authored_by_me, review_requested)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                 number = excluded.number, title = excluded.title, url = excluded.url,
                 repository = excluded.repository, author = excluded.author,
                 state = excluded.state, is_draft = excluded.is_draft,
                 review_decision = excluded.review_decision, updated_at = excluded.updated_at,
                 authored_by_me = excluded.authored_by_me,
                 review_requested = excluded.review_requested",
        )?;
        for pr in mine.iter().chain(others) {
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
                pr.review_requested
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
        .prepare("SELECT id FROM github_pull_requests p WHERE (p.authored_by_me = 1 OR p.review_requested = 1)")?
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
        workstream_id: row.get(12)?,
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
        server.reset();
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex("author:@me".into()))
            .with_body(search_page(authored, None))
            .create_async()
            .await;
        server
            .mock("POST", "/graphql")
            .match_body(Matcher::Regex("review-requested:@me".into()))
            .with_body(search_page(requested, None))
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
            .expect(2)
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
