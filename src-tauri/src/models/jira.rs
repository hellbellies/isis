use serde::{Deserialize, Serialize};

/// Non-secret Jira connection details, persisted to `jira.json`.
/// The API token is never stored here — it lives in the OS keychain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
}

/// What the frontend sees. Mirrored by `JiraSettings` in `src/types/jira.ts`.
/// Only reports whether a token exists; the token itself never crosses IPC outbound.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraSettings {
    pub base_url: String,
    pub email: String,
    pub has_token: bool,
}

/// Mirrored by `JiraSettingsInput` in `src/types/jira.ts`.
/// `api_token: None` keeps the currently stored token.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraSettingsInput {
    pub base_url: String,
    pub email: String,
    pub api_token: Option<String>,
}

/// The authenticated user, as returned by Jira's `/myself` endpoint.
/// Mirrored by `JiraUser` in `src/types/jira.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraUser {
    pub display_name: String,
    #[serde(default)]
    pub email_address: Option<String>,
}
