-- Todos are created in osiris itself (not synced from anywhere). They can be
-- linked to a workstream through workstream_items with item_type 'todo'.
CREATE TABLE todos (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    -- Unix epoch milliseconds when completed; NULL while open.
    completed_at INTEGER
);
