-- Workstreams bundle related items (Jira issues today; pull requests, todos, … later).
CREATE TABLE workstreams (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX workstreams_name ON workstreams (name COLLATE NOCASE);

-- Links items of any type to a workstream. `item_type` names the item's table
-- (e.g. 'jira_issue'), `item_id` is its id there. The primary key makes an item
-- belong to at most one workstream.
CREATE TABLE workstream_items (
    item_type     TEXT NOT NULL,
    item_id       TEXT NOT NULL,
    workstream_id INTEGER NOT NULL REFERENCES workstreams (id) ON DELETE CASCADE,
    added_at      INTEGER NOT NULL,
    PRIMARY KEY (item_type, item_id)
);
CREATE INDEX workstream_items_workstream ON workstream_items (workstream_id);

-- Issues no longer assigned to the user are kept while they belong to a workstream.
ALTER TABLE jira_issues ADD COLUMN assigned_to_me INTEGER NOT NULL DEFAULT 1;
