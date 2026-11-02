-- Pull requests the user authored or was asked to review, plus any linked to a workstream.
-- `id` is GitHub's GraphQL node id, which survives renames and transfers.
CREATE TABLE github_pull_requests (
    id               TEXT PRIMARY KEY NOT NULL,
    number           INTEGER NOT NULL,
    title            TEXT NOT NULL,
    url              TEXT NOT NULL,
    repository       TEXT NOT NULL,          -- owner/name
    author           TEXT NOT NULL,          -- login; empty for deleted accounts
    state            TEXT NOT NULL,          -- open, closed, merged
    is_draft         INTEGER NOT NULL,
    review_decision  TEXT,                   -- approved, changes_requested, review_required
    updated_at       TEXT NOT NULL,
    -- Why the PR is in the user's list. Both 0: kept only for its workstream link.
    authored_by_me   INTEGER NOT NULL,
    review_requested INTEGER NOT NULL
);

CREATE TABLE github_sync_state (
    id             INTEGER PRIMARY KEY CHECK (id = 1),
    last_synced_at INTEGER NOT NULL
);
