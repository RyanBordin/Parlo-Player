-- Initial schema (spec section 12.4).
--
-- Conventions:
-- - IDs are TEXT, exactly as the server sends them.
-- - Server dates (created, starred, played) are ISO 8601 TEXT, as sent by the API.
-- - App timestamps (sync times, play history) are Unix seconds (INTEGER).
-- - raw_json keeps the original API item, so new fields can be used without a re-sync.
-- - No foreign keys between library tables: the server is the source of truth and
--   items arrive in any order. Linking columns are indexed instead.

CREATE TABLE artists (
    id               TEXT PRIMARY KEY NOT NULL,
    name             TEXT NOT NULL,
    sort_name        TEXT,
    album_count      INTEGER,
    cover_art        TEXT,
    artist_image_url TEXT,
    starred          TEXT,
    music_brainz_id  TEXT,
    roles            TEXT NOT NULL DEFAULT '[]', -- JSON array: "albumartist", "artist", ...
    raw_json         TEXT NOT NULL
);

CREATE TABLE albums (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    sort_name       TEXT,
    artist          TEXT,
    artist_id       TEXT,
    cover_art       TEXT,
    song_count      INTEGER,
    duration        INTEGER,                  -- seconds
    year            INTEGER,
    genre           TEXT,
    genres          TEXT NOT NULL DEFAULT '[]', -- JSON array of names
    created         TEXT,                     -- date added to the server
    starred         TEXT,
    play_count      INTEGER,
    played          TEXT,                     -- last played
    is_compilation  INTEGER,                  -- 0/1
    music_brainz_id TEXT,
    raw_json        TEXT NOT NULL
);

CREATE INDEX albums_artist_id ON albums (artist_id);
CREATE INDEX albums_created ON albums (created);

CREATE TABLE songs (
    id              TEXT PRIMARY KEY NOT NULL,
    title           TEXT NOT NULL,
    sort_name       TEXT,
    album           TEXT,
    album_id        TEXT,
    artist          TEXT,
    artist_id       TEXT,
    album_artist    TEXT,
    composer        TEXT,
    track           INTEGER,
    disc_number     INTEGER,
    year            INTEGER,
    genre           TEXT,
    genres          TEXT NOT NULL DEFAULT '[]', -- JSON array of names
    duration        INTEGER,                  -- seconds
    bit_rate        INTEGER,                  -- kbps
    sampling_rate   INTEGER,                  -- Hz
    bit_depth       INTEGER,
    channel_count   INTEGER,
    suffix          TEXT,                     -- format: flac, mp3, ...
    content_type    TEXT,
    size            INTEGER,                  -- bytes
    path            TEXT,
    created         TEXT,                     -- date added to the server
    starred         TEXT,
    play_count      INTEGER,
    played          TEXT,                     -- last played
    cover_art       TEXT,
    bpm             INTEGER,
    music_brainz_id TEXT,
    raw_json        TEXT NOT NULL
);

CREATE INDEX songs_album_id ON songs (album_id);
CREATE INDEX songs_artist_id ON songs (artist_id);
CREATE INDEX songs_created ON songs (created);

CREATE TABLE playlists (
    id         TEXT PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    comment    TEXT,
    owner      TEXT,
    public     INTEGER,                       -- 0/1
    song_count INTEGER,
    duration   INTEGER,                       -- seconds
    created    TEXT,
    changed    TEXT,
    cover_art  TEXT,
    raw_json   TEXT NOT NULL
);

-- Single row (id = 1) with the state of the last syncs.
CREATE TABLE sync_state (
    id                 INTEGER PRIMARY KEY CHECK (id = 1),
    last_full_sync_at  INTEGER,               -- Unix seconds, last successful full sync
    last_server_scan   TEXT,                  -- server's lastScan seen by that full sync
    last_light_sync_at INTEGER                -- Unix seconds, last successful light sync
);

INSERT INTO sync_state (id) VALUES (1);

-- Every play with date and time (spec 11.4). Filled by the playback step.
-- No foreign key: history must survive songs removed from the server.
CREATE TABLE play_history (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    song_id   TEXT NOT NULL,
    played_at INTEGER NOT NULL,               -- Unix seconds
    scrobbled INTEGER NOT NULL DEFAULT 0      -- 1 once the server accepted the scrobble
);

CREATE INDEX play_history_played_at ON play_history (played_at);
CREATE INDEX play_history_song_id ON play_history (song_id);
