-- Unix epoch milliseconds when the workstream was marked completed; NULL while active.
ALTER TABLE workstreams ADD COLUMN completed_at INTEGER;
