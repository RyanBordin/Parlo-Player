-- Songs of each playlist in the saved order (repeats allowed), as last read
-- with getPlaylist. Lets a playlist open without connection (spec 6.5).
CREATE TABLE playlist_songs (
    playlist_id TEXT NOT NULL,
    position    INTEGER NOT NULL,             -- 0-based, saved order
    song_id     TEXT NOT NULL,
    PRIMARY KEY (playlist_id, position)
);
