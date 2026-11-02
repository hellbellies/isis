use tauri::{AppHandle, Manager};

use crate::error::AppResult;
use crate::models::jira::{JiraSettings, JiraSettingsInput, JiraUser};
use crate::services::jira::{self, KeyringStore};

const TOKEN_STORE: KeyringStore = KeyringStore::new("ch.imfeld.isis", "jira-api-token");

// Async so keychain access (which may show an OS prompt) runs off the main thread.

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
