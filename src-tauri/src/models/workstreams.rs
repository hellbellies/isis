use serde::{Deserialize, Serialize};

use crate::models::jira::JiraIssue;

/// Mirrored by `Workstream` in `src/types/workstreams.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workstream {
    pub id: i64,
    pub name: String,
    /// Unix epoch milliseconds.
    pub created_at: i64,
    /// Number of linked items of all types.
    pub item_count: i64,
}

/// A workstream with its items, grouped by item type.
/// Mirrored by `WorkstreamDetail` in `src/types/workstreams.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkstreamDetail {
    pub workstream: Workstream,
    pub jira_issues: Vec<JiraIssue>,
}

/// Kinds of items a workstream can hold. Mirrored by `WorkstreamItemType` in `src/types/workstreams.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkstreamItemType {
    JiraIssue,
}

impl WorkstreamItemType {
    /// Value stored in `workstream_items.item_type`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::JiraIssue => "jira_issue",
        }
    }
}

/// Identifies one item of any type. Mirrored by `WorkstreamItemRef` in `src/types/workstreams.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkstreamItemRef {
    pub item_type: WorkstreamItemType,
    /// The item's id in its own table, e.g. the Jira issue id.
    pub item_id: String,
}
