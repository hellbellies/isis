//! Local cache of the user's Jira issues and the full/delta sync that fills it.
//!
//! A full sync replaces every stored issue. A delta sync asks Jira only for issues
//! updated since the last sync (relative JQL, so time zones don't matter) and
//! - upserts those still assigned to the user,
//! - removes those that were reassigned or unassigned.
//!
//! Issues deleted in Jira are only noticed by a full sync.

use rusqlite::{Connection, OptionalExtension, params};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::jira::{JiraIssue, JiraIssueList, JiraSyncMode, JiraSyncSummary};
use crate::services::jira_client::JiraClient;

/// Extra minutes added to the delta window to absorb clock skew and search-index lag.
const DELTA_MARGIN_MINUTES: i64 = 5;

/// What the next sync has to do.
#[derive(Debug, PartialEq, Eq)]
enum SyncPlan {
    Full,
    /// Fetch issues updated within the last `minutes`.
    Delta {
        minutes: i64,
    },
}

struct SyncState {
    scope: String,
    last_synced_at: i64,
}

/// All stored issues, most recently updated first.
pub fn list(conn: &Connection) -> AppResult<JiraIssueList> {
    let mut statement = conn
        .prepare("SELECT id, key, summary, updated FROM jira_issues ORDER BY updated DESC, key")?;
    let issues = statement
        .query_map([], |row| {
            Ok(JiraIssue {
                id: row.get(0)?,
                key: row.get(1)?,
                summary: row.get(2)?,
                updated: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(JiraIssueList {
        issues,
        last_synced_at: load_state(conn)?.map(|state| state.last_synced_at),
    })
}

/// Fetches the user's issues in `project_keys` and stores them.
/// Runs a delta sync when possible; `force_full` or a changed site/account/project
/// selection triggers a full sync. `now_ms` is recorded as the sync time.
pub async fn sync(
    db: &Db,
    client: &JiraClient,
    project_keys: &[String],
    force_full: bool,
    now_ms: i64,
) -> AppResult<JiraSyncSummary> {
    if project_keys.is_empty() {
        return Err(AppError::Validation(
            "Add at least one project key in the Jira settings".into(),
        ));
    }
    let scope = scope(client, project_keys);
    // The lock is released at the end of this statement — never hold it across `.await`.
    let state = load_state(&db.lock())?;
    let projects = jql_projects(project_keys);

    let (mode, upserts, removed_ids) = match plan(state.as_ref(), &scope, force_full, now_ms) {
        SyncPlan::Full => {
            let jql = format!("{projects} AND assignee = currentUser() ORDER BY updated DESC");
            (JiraSyncMode::Full, client.search(&jql).await?, Vec::new())
        }
        SyncPlan::Delta { minutes } => {
            let changed = format!("{projects} AND updated >= \"-{minutes}m\"");
            let mine = client
                .search(&format!("{changed} AND assignee = currentUser()"))
                .await?;
            let not_mine = client
                .search(&format!(
                    "{changed} AND (assignee != currentUser() OR assignee IS EMPTY)"
                ))
                .await?;
            let removed_ids = not_mine.into_iter().map(|issue| issue.id).collect();
            (JiraSyncMode::Delta, mine, removed_ids)
        }
    };

    apply(&mut db.lock(), mode, &upserts, &removed_ids, &scope, now_ms)
}

/// Decides between a full and a delta sync.
fn plan(state: Option<&SyncState>, scope: &str, force_full: bool, now_ms: i64) -> SyncPlan {
    match state {
        Some(state) if !force_full && state.scope == scope => {
            let elapsed_ms = (now_ms - state.last_synced_at).max(0);
            let minutes = (elapsed_ms + 59_999) / 60_000 + DELTA_MARGIN_MINUTES;
            SyncPlan::Delta { minutes }
        }
        _ => SyncPlan::Full,
    }
}

/// Identifies what the stored issues belong to. If it changes, a delta is meaningless.
fn scope(client: &JiraClient, project_keys: &[String]) -> String {
    let mut keys = project_keys.to_vec();
    keys.sort();
    format!(
        "{}|{}|{}",
        client.base_url(),
        client.email(),
        keys.join(",")
    )
}

/// `project in ("FOO", "BAR")`. Keys are validated on save, so quoting is enough.
fn jql_projects(project_keys: &[String]) -> String {
    let quoted: Vec<String> = project_keys
        .iter()
        .map(|key| format!("\"{key}\""))
        .collect();
    format!("project in ({})", quoted.join(", "))
}

/// Writes a sync result in one transaction, so a failure leaves the previous data intact.
fn apply(
    conn: &mut Connection,
    mode: JiraSyncMode,
    upserts: &[JiraIssue],
    removed_ids: &[String],
    scope: &str,
    now_ms: i64,
) -> AppResult<JiraSyncSummary> {
    let tx = conn.transaction()?;
    if mode == JiraSyncMode::Full {
        tx.execute("DELETE FROM jira_issues", [])?;
    }
    {
        let mut upsert = tx.prepare(
            "INSERT INTO jira_issues (id, key, summary, updated) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET
                 key = excluded.key, summary = excluded.summary, updated = excluded.updated",
        )?;
        for issue in upserts {
            upsert.execute(params![issue.id, issue.key, issue.summary, issue.updated])?;
        }
    }
    let mut removed = 0;
    {
        let mut delete = tx.prepare("DELETE FROM jira_issues WHERE id = ?1")?;
        for id in removed_ids {
            removed += delete.execute(params![id])?;
        }
    }
    tx.execute(
        "INSERT INTO jira_sync_state (id, scope, last_synced_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET scope = excluded.scope, last_synced_at = excluded.last_synced_at",
        params![scope, now_ms],
    )?;
    tx.commit()?;
    Ok(JiraSyncSummary {
        mode,
        upserted: upserts.len(),
        removed,
    })
}

fn load_state(conn: &Connection) -> AppResult<Option<SyncState>> {
    Ok(conn
        .query_row(
            "SELECT scope, last_synced_at FROM jira_sync_state WHERE id = 1",
            [],
            |row| {
                Ok(SyncState {
                    scope: row.get(0)?,
                    last_synced_at: row.get(1)?,
                })
            },
        )
        .optional()?)
}

#[cfg(test)]
mod tests {
    use mockito::{Matcher, Server};

    use super::*;
    use crate::db;
    use crate::services::jira_client::tests::{SEARCH, client};

    const NOW: i64 = 1_800_000_000_000;

    fn keys(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    fn issue_json(id: &str, key: &str, summary: &str) -> String {
        format!(
            r#"{{"id":"{id}","key":"{key}","fields":{{"summary":"{summary}","updated":"2026-11-02T10:00:00.000+0000"}}}}"#
        )
    }

    fn page(issues: &[String]) -> String {
        format!(r#"{{"issues":[{}],"isLast":true}}"#, issues.join(","))
    }

    async fn mock_search(server: &mut Server, jql_part: &str, body: String) -> mockito::Mock {
        server
            .mock("POST", SEARCH)
            .match_body(Matcher::Regex(regex_escape(jql_part)))
            .with_body(body)
            .create_async()
            .await
    }

    fn regex_escape(text: &str) -> String {
        text.chars()
            .flat_map(|c| {
                let escape = "\\.+*?()|[]{}^$".contains(c);
                escape.then_some('\\').into_iter().chain(Some(c))
            })
            .collect()
    }

    fn stored_keys(db: &Db) -> Vec<String> {
        list(&db.lock())
            .unwrap()
            .issues
            .into_iter()
            .map(|issue| issue.key)
            .collect()
    }

    #[test]
    fn plan_uses_delta_only_for_same_scope() {
        let state = SyncState {
            scope: "s".into(),
            last_synced_at: NOW - 10 * 60_000 - 1,
        };
        assert_eq!(
            plan(Some(&state), "s", false, NOW),
            SyncPlan::Delta {
                minutes: 11 + DELTA_MARGIN_MINUTES
            }
        );
        assert_eq!(plan(Some(&state), "other", false, NOW), SyncPlan::Full);
        assert_eq!(plan(Some(&state), "s", true, NOW), SyncPlan::Full);
        assert_eq!(plan(None, "s", false, NOW), SyncPlan::Full);
    }

    #[test]
    fn jql_projects_quotes_keys() {
        assert_eq!(
            jql_projects(&keys(&["FOO", "BAR"])),
            r#"project in ("FOO", "BAR")"#
        );
    }

    #[tokio::test]
    async fn first_sync_is_full_then_delta_applies_changes() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let client = client(&server.url());
        let projects = keys(&["FOO"]);

        let full = mock_search(
            &mut server,
            "assignee = currentUser() ORDER BY",
            page(&[
                issue_json("1", "FOO-1", "One"),
                issue_json("2", "FOO-2", "Two"),
            ]),
        )
        .await;
        let summary = sync(&db, &client, &projects, false, NOW).await.unwrap();
        assert_eq!(summary.mode, JiraSyncMode::Full);
        assert_eq!(summary.upserted, 2);
        assert_eq!(stored_keys(&db).len(), 2);
        full.assert_async().await;

        // 3 minutes later: FOO-1 was renamed, FOO-2 reassigned, FOO-3 newly assigned.
        server.reset();
        let mine = mock_search(
            &mut server,
            r#"updated >= \"-8m\" AND assignee = currentUser()"#,
            page(&[
                issue_json("1", "FOO-1", "One (renamed)"),
                issue_json("3", "FOO-3", "Three"),
            ]),
        )
        .await;
        let not_mine = mock_search(
            &mut server,
            "assignee IS EMPTY",
            page(&[issue_json("2", "FOO-2", "Two")]),
        )
        .await;
        let summary = sync(&db, &client, &projects, false, NOW + 3 * 60_000)
            .await
            .unwrap();
        assert_eq!(
            summary,
            JiraSyncSummary {
                mode: JiraSyncMode::Delta,
                upserted: 2,
                removed: 1
            }
        );
        mine.assert_async().await;
        not_mine.assert_async().await;

        let list = list(&db.lock()).unwrap();
        assert_eq!(list.last_synced_at, Some(NOW + 3 * 60_000));
        let mut summaries: Vec<_> = list.issues.iter().map(|i| i.summary.as_str()).collect();
        summaries.sort();
        assert_eq!(summaries, ["One (renamed)", "Three"]);
    }

    #[tokio::test]
    async fn changed_projects_force_full_sync_replacing_old_issues() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let client = client(&server.url());

        mock_search(
            &mut server,
            r#"project in (\"FOO\")"#,
            page(&[issue_json("1", "FOO-1", "One")]),
        )
        .await;
        sync(&db, &client, &keys(&["FOO"]), false, NOW)
            .await
            .unwrap();

        mock_search(
            &mut server,
            r#"project in (\"BAR\")"#,
            page(&[issue_json("9", "BAR-9", "Nine")]),
        )
        .await;
        let summary = sync(&db, &client, &keys(&["BAR"]), false, NOW + 60_000)
            .await
            .unwrap();
        assert_eq!(summary.mode, JiraSyncMode::Full);
        assert_eq!(stored_keys(&db), ["BAR-9"]);
    }

    #[tokio::test]
    async fn failed_sync_keeps_existing_data() {
        let mut server = Server::new_async().await;
        let db = Db::new(db::open_in_memory());
        let client = client(&server.url());
        let projects = keys(&["FOO"]);

        mock_search(&mut server, "FOO", page(&[issue_json("1", "FOO-1", "One")])).await;
        sync(&db, &client, &projects, false, NOW).await.unwrap();

        server.reset();
        server
            .mock("POST", SEARCH)
            .with_status(500)
            .create_async()
            .await;
        assert!(
            sync(&db, &client, &projects, true, NOW + 60_000)
                .await
                .is_err()
        );
        let list = list(&db.lock()).unwrap();
        assert_eq!(list.issues.len(), 1);
        assert_eq!(list.last_synced_at, Some(NOW));
    }

    #[tokio::test]
    async fn sync_requires_project_keys() {
        let db = Db::new(db::open_in_memory());
        let client = client("http://127.0.0.1:9");
        assert!(matches!(
            sync(&db, &client, &[], false, NOW).await,
            Err(AppError::Validation(_))
        ));
    }
}
