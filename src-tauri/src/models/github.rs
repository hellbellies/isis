use serde::{Deserialize, Serialize};

/// Non-secret GitHub settings, persisted to `github.json`. The token lives in the OS keychain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GitHubConfig {
    /// Optional search scope: `owner/repo` or `owner` entries. Empty means all repositories.
    pub repositories: Vec<String>,
}

/// Mirrored by `GitHubSettings` in `src/types/github.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSettings {
    pub repositories: Vec<String>,
    pub has_token: bool,
}

/// Mirrored by `GitHubSettingsInput` in `src/types/github.ts`.
/// `token: None` keeps the currently stored token.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSettingsInput {
    #[serde(default)]
    pub repositories: Vec<String>,
    pub token: Option<String>,
}

/// The authenticated GitHub account. Mirrored by `GitHubUser` in `src/types/github.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubUser {
    pub login: String,
    pub name: Option<String>,
}

/// Mirrored by `PullRequestState` in `src/types/github.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

/// Mirrored by `ReviewDecision` in `src/types/github.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Approved,
    ChangesRequested,
    ReviewRequired,
}

/// A cached pull request. Mirrored by `PullRequest` in `src/types/github.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    /// GitHub's GraphQL node id.
    pub id: String,
    pub number: i64,
    pub title: String,
    pub url: String,
    /// `owner/name`.
    pub repository: String,
    pub author: String,
    pub state: PullRequestState,
    pub is_draft: bool,
    pub review_decision: Option<ReviewDecision>,
    /// ISO-8601 timestamp of the last change.
    pub updated_at: String,
    pub authored_by_me: bool,
    pub review_requested: bool,
    /// Open in a watched repository, without involving the user.
    pub other_open: bool,
    pub workstream_id: Option<i64>,
}

/// A pull request that mentions a Jira issue's key.
/// Mirrored by `JiraIssuePullRequest` in `src/types/github.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JiraIssuePullRequest {
    /// Jira's issue id (not the key).
    pub issue_id: String,
    pub pull_request: PullRequest,
}

/// Mirrored by `PullRequestList` in `src/types/github.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestList {
    pub pull_requests: Vec<PullRequest>,
    /// Unix epoch milliseconds of the last successful sync.
    pub last_synced_at: Option<i64>,
}

/// Mirrored by `GitHubSyncSummary` in `src/types/github.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubSyncSummary {
    /// Open pull requests that involve the user.
    pub fetched: usize,
    /// Pull requests that dropped out of the user's list.
    pub removed: usize,
    /// Other open pull requests in the watched repositories.
    pub others: usize,
}
