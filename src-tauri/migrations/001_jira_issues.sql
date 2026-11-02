-- Jira issues assigned to the current user. `id` is Jira's immutable issue id;
-- `key` can change when an issue moves between projects.
CREATE TABLE jira_issues (
    id      TEXT PRIMARY KEY NOT NULL,
    key     TEXT NOT NULL,
    summary TEXT NOT NULL,
    updated TEXT NOT NULL
);

-- Single row describing the last successful sync.
-- `scope` identifies site, account and projects; a change forces a full sync.
CREATE TABLE jira_sync_state (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    scope          TEXT NOT NULL,
    last_synced_at INTEGER NOT NULL
);
