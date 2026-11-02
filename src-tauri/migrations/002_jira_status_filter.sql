ALTER TABLE jira_issues ADD COLUMN project_key TEXT NOT NULL DEFAULT '';
ALTER TABLE jira_issues ADD COLUMN status_id TEXT NOT NULL DEFAULT '';
ALTER TABLE jira_issues ADD COLUMN status_name TEXT NOT NULL DEFAULT '';

-- Existing rows have no project/status yet: drop the sync state so the next fetch is a full sync.
DELETE FROM jira_sync_state;

-- Every status a project's workflows can use, refreshed on each sync.
-- `category` is Jira's status category key: new, indeterminate or done.
CREATE TABLE jira_project_statuses (
    project_key TEXT NOT NULL,
    status_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    category    TEXT NOT NULL,
    PRIMARY KEY (project_key, status_id)
);

-- Statuses the user wants to see, per project. No rows means "show everything".
CREATE TABLE jira_status_filter (
    project_key TEXT NOT NULL,
    status_id   TEXT NOT NULL,
    PRIMARY KEY (project_key, status_id)
);
