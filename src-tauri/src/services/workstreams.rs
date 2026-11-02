//! Workstreams bundle related items of any type (Jira issues today).
//!
//! Links live in `workstream_items`, keyed by `(item_type, item_id)` so an item
//! belongs to at most one workstream. Removing a link may make a cached item
//! obsolete, so every unlink path asks the item's service to prune.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{AppError, AppResult};
use crate::models::workstreams::{
    Workstream, WorkstreamDetail, WorkstreamItemRef, WorkstreamItemType,
};
use crate::services::jira_issues;

const MAX_NAME_LENGTH: usize = 80;

/// All workstreams with their item counts, sorted by name.
pub fn list(conn: &Connection) -> AppResult<Vec<Workstream>> {
    Ok(conn
        .prepare(
            "SELECT w.id, w.name, w.created_at, COUNT(i.item_id)
             FROM workstreams w LEFT JOIN workstream_items i ON i.workstream_id = w.id
             GROUP BY w.id ORDER BY w.name COLLATE NOCASE",
        )?
        .query_map([], |row| {
            Ok(Workstream {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                item_count: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Workstream> {
    conn.query_row(
        "SELECT w.id, w.name, w.created_at,
                (SELECT COUNT(*) FROM workstream_items i WHERE i.workstream_id = w.id)
         FROM workstreams w WHERE w.id = ?1",
        [id],
        |row| {
            Ok(Workstream {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                item_count: row.get(3)?,
            })
        },
    )
    .optional()?
    .ok_or_else(not_found)
}

/// A workstream with all its items.
pub fn detail(conn: &Connection, id: i64) -> AppResult<WorkstreamDetail> {
    Ok(WorkstreamDetail {
        workstream: get(conn, id)?,
        jira_issues: jira_issues::list_for_workstream(conn, id)?,
    })
}

pub fn create(conn: &Connection, name: &str, now_ms: i64) -> AppResult<Workstream> {
    let name = validate_name(conn, name, None)?;
    conn.execute(
        "INSERT INTO workstreams (name, created_at) VALUES (?1, ?2)",
        params![name, now_ms],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn rename(conn: &Connection, id: i64, name: &str) -> AppResult<Workstream> {
    let name = validate_name(conn, name, Some(id))?;
    if conn.execute(
        "UPDATE workstreams SET name = ?1 WHERE id = ?2",
        params![name, id],
    )? == 0
    {
        return Err(not_found());
    }
    get(conn, id)
}

/// Deletes a workstream. Its items are unlinked, not deleted.
pub fn delete(conn: &mut Connection, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    // `workstream_items` rows go with it via ON DELETE CASCADE.
    if tx.execute("DELETE FROM workstreams WHERE id = ?1", [id])? == 0 {
        return Err(not_found());
    }
    jira_issues::prune_unlinked(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Moves an item into `workstream_id`, or out of any workstream when `None`.
pub fn set_item_workstream(
    conn: &mut Connection,
    item: &WorkstreamItemRef,
    workstream_id: Option<i64>,
    now_ms: i64,
) -> AppResult<()> {
    let tx = conn.transaction()?;
    let item_type = item.item_type.as_str();
    match workstream_id {
        Some(workstream_id) => {
            get(&tx, workstream_id)?;
            if !item_exists(&tx, item)? {
                return Err(AppError::Validation("This item no longer exists".into()));
            }
            tx.execute(
                "INSERT INTO workstream_items (item_type, item_id, workstream_id, added_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (item_type, item_id) DO UPDATE SET
                     workstream_id = excluded.workstream_id, added_at = excluded.added_at
                 WHERE workstream_id != excluded.workstream_id",
                params![item_type, item.item_id, workstream_id, now_ms],
            )?;
        }
        None => {
            tx.execute(
                "DELETE FROM workstream_items WHERE item_type = ?1 AND item_id = ?2",
                params![item_type, item.item_id],
            )?;
            prune(&tx, item.item_type)?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn item_exists(conn: &Connection, item: &WorkstreamItemRef) -> AppResult<bool> {
    match item.item_type {
        WorkstreamItemType::JiraIssue => jira_issues::exists(conn, &item.item_id),
    }
}

/// Lets the item's own service drop cached items that were only kept for their link.
fn prune(conn: &Connection, item_type: WorkstreamItemType) -> AppResult<()> {
    match item_type {
        WorkstreamItemType::JiraIssue => jira_issues::prune_unlinked(conn).map(|_| ()),
    }
}

/// Trims the name and checks it is non-empty, not too long and unique (ignoring case).
fn validate_name(conn: &Connection, name: &str, except_id: Option<i64>) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation("Enter a workstream name".into()));
    }
    if name.chars().count() > MAX_NAME_LENGTH {
        return Err(AppError::Validation(format!(
            "Workstream names can be at most {MAX_NAME_LENGTH} characters"
        )));
    }
    let taken = conn
        .query_row(
            "SELECT 1 FROM workstreams WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2",
            params![name, except_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if taken {
        return Err(AppError::Validation(format!(
            "A workstream named \"{name}\" already exists"
        )));
    }
    Ok(name.to_owned())
}

fn not_found() -> AppError {
    AppError::Validation("This workstream no longer exists".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const NOW: i64 = 1_800_000_000_000;

    fn jira(id: &str) -> WorkstreamItemRef {
        WorkstreamItemRef {
            item_type: WorkstreamItemType::JiraIssue,
            item_id: id.into(),
        }
    }

    /// Caches Jira issue `id`, assigned to the user or not.
    fn insert_issue(conn: &Connection, id: &str, assigned: bool) {
        conn.execute(
            "INSERT INTO jira_issues (id, key, summary, updated, assigned_to_me)
             VALUES (?1, 'FOO-' || ?1, 'Summary', '2026-11-02', ?2)",
            params![id, assigned],
        )
        .unwrap();
    }

    #[test]
    fn create_validates_and_list_sorts_by_name() {
        let conn = db::open_in_memory();
        create(&conn, "  zeta ", NOW).unwrap();
        let alpha = create(&conn, "Alpha", NOW).unwrap();
        assert_eq!(alpha.item_count, 0);

        let names: Vec<_> = list(&conn).unwrap().into_iter().map(|w| w.name).collect();
        assert_eq!(names, ["Alpha", "zeta"]);

        for bad in ["", "   ", "ALPHA", &"x".repeat(MAX_NAME_LENGTH + 1)] {
            assert!(
                matches!(create(&conn, bad, NOW), Err(AppError::Validation(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn rename_allows_same_name_with_different_case_on_itself_only() {
        let conn = db::open_in_memory();
        let alpha = create(&conn, "Alpha", NOW).unwrap();
        create(&conn, "Beta", NOW).unwrap();
        assert_eq!(rename(&conn, alpha.id, "ALPHA").unwrap().name, "ALPHA");
        assert!(rename(&conn, alpha.id, "beta").is_err());
        assert!(rename(&conn, 999, "Gamma").is_err());
    }

    #[test]
    fn items_belong_to_one_workstream_and_can_move() {
        let mut conn = db::open_in_memory();
        insert_issue(&conn, "1", true);
        let a = create(&conn, "A", NOW).unwrap();
        let b = create(&conn, "B", NOW).unwrap();

        set_item_workstream(&mut conn, &jira("1"), Some(a.id), NOW).unwrap();
        set_item_workstream(&mut conn, &jira("1"), Some(b.id), NOW).unwrap();
        assert_eq!(get(&conn, a.id).unwrap().item_count, 0);
        let detail = detail(&conn, b.id).unwrap();
        assert_eq!(detail.workstream.item_count, 1);
        assert_eq!(detail.jira_issues[0].workstream_id, Some(b.id));

        // Shows up in the user's issue list with its workstream.
        let listed = jira_issues::list(&conn).unwrap().issues;
        assert_eq!(listed[0].workstream_id, Some(b.id));
    }

    #[test]
    fn set_item_workstream_rejects_unknown_item_or_workstream() {
        let mut conn = db::open_in_memory();
        insert_issue(&conn, "1", true);
        let a = create(&conn, "A", NOW).unwrap();
        assert!(set_item_workstream(&mut conn, &jira("404"), Some(a.id), NOW).is_err());
        assert!(set_item_workstream(&mut conn, &jira("1"), Some(999), NOW).is_err());
    }

    #[test]
    fn unlinking_prunes_issues_not_assigned_to_the_user() {
        let mut conn = db::open_in_memory();
        insert_issue(&conn, "mine", true);
        insert_issue(&conn, "theirs", false);
        let a = create(&conn, "A", NOW).unwrap();
        for id in ["mine", "theirs"] {
            set_item_workstream(&mut conn, &jira(id), Some(a.id), NOW).unwrap();
        }

        set_item_workstream(&mut conn, &jira("theirs"), None, NOW).unwrap();
        assert!(!jira_issues::exists(&conn, "theirs").unwrap());

        // Deleting the workstream unlinks the rest; assigned issues stay cached.
        delete(&mut conn, a.id).unwrap();
        assert!(jira_issues::exists(&conn, "mine").unwrap());
        assert!(list(&conn).unwrap().is_empty());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM workstream_items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
