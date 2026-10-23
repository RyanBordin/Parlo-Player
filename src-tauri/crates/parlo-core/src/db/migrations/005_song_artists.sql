-- Who appears on each song (spec 6.6, 6.7): the OpenSubsonic `artists` list
-- of the song (featured artists included) and its `albumArtists`. The
-- migration fills it from the raw JSON already cached (no new sync needed);
-- each full sync rebuilds it.
CREATE TABLE song_artists (
    song_id   TEXT NOT NULL,
    artist_id TEXT NOT NULL,
    role      TEXT NOT NULL,                  -- 'artist' or 'albumartist'
    PRIMARY KEY (song_id, artist_id, role)
) WITHOUT ROWID;

CREATE INDEX song_artists_artist_id ON song_artists (artist_id);
