use serde::{Deserialize, Serialize};

/// Mirrored by `Todo` in `src/types/todos.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: i64,
    pub title: String,
    /// Plain text; may be empty.
    pub description: String,
    /// Unix epoch milliseconds.
    pub created_at: i64,
    pub updated_at: i64,
    /// When the todo was completed; `None` while open.
    pub completed_at: Option<i64>,
    pub workstream_id: Option<i64>,
}

/// Title and description of a todo. Mirrored by `TodoInput` in `src/types/todos.ts`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoInput {
    pub title: String,
    #[serde(default)]
    pub description: String,
}
