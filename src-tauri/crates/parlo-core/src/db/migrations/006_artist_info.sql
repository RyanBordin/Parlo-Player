-- What getArtistInfo2 says about each artist (spec 6.7), asked when the
-- artist page opens and kept for 7 days. Not library data: syncs leave it.
CREATE TABLE artist_info (
    artist_id   TEXT PRIMARY KEY NOT NULL,
    biography   TEXT,                         -- plain text, HTML removed by the app
    similar_ids TEXT NOT NULL DEFAULT '[]',   -- JSON array of artist IDs, server order
    has_photo   INTEGER,                      -- 0/1: real photo or the server's generic image; NULL: unknown
    fetched_at  INTEGER NOT NULL,             -- Unix seconds
    raw_json    TEXT NOT NULL
);
