//! Domain services. Pure Rust logic with no knowledge of IPC — commands
//! call into these, which keeps them unit-testable.

pub mod jira;
pub mod jira_client;
pub mod jira_issues;
pub mod settings;
