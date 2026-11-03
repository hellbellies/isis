-- Name of the issue's work type, e.g. Task, Bug or Epic.
ALTER TABLE jira_issues ADD COLUMN issue_type TEXT NOT NULL DEFAULT '';

-- Existing rows have no issue type yet: drop the sync state so the next fetch is a full sync.
DELETE FROM jira_sync_state;
