-- The result of the last sync, shown in Settings > Sync (spec 6.10) and kept
-- across restarts.
ALTER TABLE sync_state ADD COLUMN last_sync_at INTEGER;   -- Unix seconds, when the last sync ended
ALTER TABLE sync_state ADD COLUMN last_sync_error TEXT;   -- JSON error sent to the UI; NULL on success
