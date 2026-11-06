-- Pull requests (in any state) whose title, branch or description mentions a cached
-- Jira issue's key, found by GitHub search. Replaced on every GitHub sync; links go
-- with their issue, and linked pull requests are kept in the cache.
CREATE TABLE jira_issue_pull_requests (
    issue_id        TEXT NOT NULL REFERENCES jira_issues (id) ON DELETE CASCADE,
    pull_request_id TEXT NOT NULL REFERENCES github_pull_requests (id) ON DELETE CASCADE,
    PRIMARY KEY (issue_id, pull_request_id)
);
CREATE INDEX jira_issue_pull_requests_pull_request ON jira_issue_pull_requests (pull_request_id);
