-- The play queue as it was last saved (spec 5.6). Single row (id = 1).
-- The server keeps songs, current song and position; the rest exists only here.
CREATE TABLE saved_queue (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    songs           TEXT NOT NULL,            -- JSON array of song IDs, original order
    play_order      TEXT NOT NULL,            -- JSON array of indexes into songs, playback order
    current         INTEGER NOT NULL,         -- position of the current song in play_order
    position        REAL NOT NULL,            -- seconds into the current song
    shuffle         INTEGER NOT NULL,         -- 0/1
    repeat          TEXT NOT NULL,            -- off / all / one
    label           TEXT NOT NULL,            -- "Playing from"
    continuing_from TEXT,                     -- app name, when continued from another app
    volume          REAL NOT NULL,            -- 0–100
    saved_at        INTEGER NOT NULL          -- Unix milliseconds
);
