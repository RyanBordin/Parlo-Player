-- Preferences tied to this server + user (spec 6.10): scrobble, and later
-- the sync interval, continuing another device's queue and visible folders.
-- One row per preference, its value as JSON, so new ones need no migration.
CREATE TABLE settings (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL                       -- JSON
) WITHOUT ROWID;
