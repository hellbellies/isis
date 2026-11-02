use tauri::State;

use super::now_ms;
use crate::db::Db;
use crate::error::AppResult;
use crate::models::workstreams::{Workstream, WorkstreamDetail, WorkstreamItemRef};
use crate::services::workstreams;

#[tauri::command]
pub async fn list_workstreams(db: State<'_, Db>) -> AppResult<Vec<Workstream>> {
    workstreams::list(&db.lock())
}

#[tauri::command]
pub async fn get_workstream(db: State<'_, Db>, id: i64) -> AppResult<WorkstreamDetail> {
    workstreams::detail(&db.lock(), id)
}

#[tauri::command]
pub async fn create_workstream(db: State<'_, Db>, name: String) -> AppResult<Workstream> {
    workstreams::create(&db.lock(), &name, now_ms())
}

#[tauri::command]
pub async fn rename_workstream(db: State<'_, Db>, id: i64, name: String) -> AppResult<Workstream> {
    workstreams::rename(&db.lock(), id, &name)
}

#[tauri::command]
pub async fn delete_workstream(db: State<'_, Db>, id: i64) -> AppResult<()> {
    workstreams::delete(&mut db.lock(), id)
}

/// Assigns an item to a workstream, or removes it from its workstream when `workstream_id` is null.
#[tauri::command]
pub async fn set_item_workstream(
    db: State<'_, Db>,
    item: WorkstreamItemRef,
    workstream_id: Option<i64>,
) -> AppResult<()> {
    workstreams::set_item_workstream(&mut db.lock(), &item, workstream_id, now_ms())
}
