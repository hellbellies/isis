use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::db::Db;
use crate::error::AppResult;
use crate::models::jira::{
    JiraIssueList, JiraSettings, JiraSettingsInput, JiraSyncSummary, JiraUser,
};
use crate::services::jira::{self, KeyringStore};
use crate::services::jira_client::JiraClient;
use crate::services::jira_issues;

const TOKEN_STORE: KeyringStore = KeyringStore::new("ch.imfeld.isis", "jira-api-token");

// Async so keychain, network and database work runs off the main thread.

#[tauri::command]
pub async fn get_jira_settings(app: AppHandle) -> AppResult<JiraSettings> {
    let dir = app.path().app_config_dir()?;
    jira::get(&dir, &TOKEN_STORE)
}

#[tauri::command]
pub async fn update_jira_settings(
    app: AppHandle,
    input: JiraSettingsInput,
) -> AppResult<JiraSettings> {
    let dir = app.path().app_config_dir()?;
    jira::save(&dir, &TOKEN_STORE, input)
}

#[tauri::command]
pub async fn clear_jira_token(app: AppHandle) -> AppResult<JiraSettings> {
    let dir = app.path().app_config_dir()?;
    jira::clear_token(&dir, &TOKEN_STORE)
}

/// Verifies the given (unsaved) settings against Jira without persisting them.
#[tauri::command]
pub async fn test_jira_connection(input: JiraSettingsInput) -> AppResult<JiraUser> {
    jira::test_connection(&TOKEN_STORE, input).await
}

#[tauri::command]
pub async fn list_jira_issues(db: State<'_, Db>) -> AppResult<JiraIssueList> {
    jira_issues::list(&db.lock())
}

/// Fetches the user's assigned issues; a delta sync unless `full` is set
/// or the site, account or project selection changed.
#[tauri::command]
pub async fn sync_jira_issues(
    app: AppHandle,
    db: State<'_, Db>,
    full: bool,
) -> AppResult<JiraSyncSummary> {
    let dir = app.path().app_config_dir()?;
    let config = jira::load_config(&dir)?;
    let client = JiraClient::new(jira::credentials(&dir, &TOKEN_STORE)?)?;
    jira_issues::sync(&db, &client, &config.project_keys, full, now_ms()).await
}

/// Opens an issue of the configured site in the default browser.
#[tauri::command]
pub async fn open_jira_issue(app: AppHandle, key: String) -> AppResult<()> {
    let dir = app.path().app_config_dir()?;
    let url = jira::issue_url(&dir, &key)?;
    app.opener().open_url(url, None::<&str>)?;
    Ok(())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}
