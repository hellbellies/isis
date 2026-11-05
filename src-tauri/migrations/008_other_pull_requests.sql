-- Open pull requests in the watched repositories that involve the user neither as
-- author nor as reviewer. Watched: the configured repositories, or else the
-- repositories of the user's own pull requests.
ALTER TABLE github_pull_requests ADD COLUMN other_open INTEGER NOT NULL DEFAULT 0;
