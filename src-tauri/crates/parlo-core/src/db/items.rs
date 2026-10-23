//! What the "..." menus read from the local cache (spec 8): the details of
//! "Information", the local fallback of the artist radio, the songs of
//! "Save to computer", and a deleted playlist.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::DbResult;
use super::artist_page::ARTIST_SONGS;

/// "File information" of a song (spec 8.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongInfo {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// e.g. "flac".
    pub suffix: Option<String>,
    pub content_type: Option<String>,
    /// kbps.
    pub bit_rate: Option<i64>,
    /// Hz.
    pub sampling_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channel_count: Option<i64>,
    /// Bytes.
    pub size: Option<i64>,
    /// Seconds.
    pub duration: Option<i64>,
    /// On the server.
    pub path: Option<String>,
    pub album_id: Option<String>,
    pub starred: bool,
    /// Every artist of the song that is in the library, in the song's order
    /// ("Go to artist" becomes a submenu with more than one, spec 8.1).
    pub artists: Vec<ArtistRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistRef {
    pub id: String,
    pub name: String,
}

pub fn song_info(conn: &Connection, song_id: &str) -> DbResult<Option<SongInfo>> {
    let info = conn
        .query_row(
            "SELECT title, artist, album, suffix, content_type, bit_rate, sampling_rate, bit_depth,
                    channel_count, size, duration, path, album_id, starred IS NOT NULL
             FROM songs WHERE id = ?1",
            [song_id],
            |row| {
                Ok(SongInfo {
                    title: row.get(0)?,
                    artist: row.get(1)?,
                    album: row.get(2)?,
                    suffix: row.get(3)?,
                    content_type: row.get(4)?,
                    bit_rate: row.get(5)?,
                    sampling_rate: row.get(6)?,
                    bit_depth: row.get(7)?,
                    channel_count: row.get(8)?,
                    size: row.get(9)?,
                    duration: row.get(10)?,
                    path: row.get(11)?,
                    album_id: row.get(12)?,
                    starred: row.get(13)?,
                    artists: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut info) = info else { return Ok(None) };

    // OpenSubsonic `artists` in the song's order; without it, the main artist.
    let mut stmt = conn.prepare(
        "SELECT ar.id, ar.name FROM songs s, json_each(s.raw_json, '$.artists') a
           JOIN artists ar ON ar.id = json_extract(a.value, '$.id')
          WHERE s.id = ?1 ORDER BY a.key",
    )?;
    info.artists = stmt
        .query_map([song_id], |row| Ok(ArtistRef { id: row.get(0)?, name: row.get(1)? }))?
        .collect::<Result<_, _>>()?;
    if info.artists.is_empty() {
        let main = conn
            .query_row(
                "SELECT ar.id, ar.name FROM songs s JOIN artists ar ON ar.id = s.artist_id WHERE s.id = ?1",
                [song_id],
                |row| Ok(ArtistRef { id: row.get(0)?, name: row.get(1)? }),
            )
            .optional()?;
        info.artists.extend(main);
    }
    Ok(Some(info))
}

/// "Information" of an album (spec 8.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumInfo {
    pub name: String,
    pub artist: Option<String>,
    pub year: Option<i64>,
    /// Record labels (OpenSubsonic `recordLabels`), when the server sends them.
    pub labels: Vec<String>,
    pub genres: Vec<String>,
    /// Formats of its songs, e.g. ["flac"].
    pub formats: Vec<String>,
    pub song_count: i64,
    /// Seconds.
    pub duration: Option<i64>,
    pub starred: bool,
}

pub fn album_info(conn: &Connection, album_id: &str) -> DbResult<Option<AlbumInfo>> {
    let album = conn
        .query_row(
            "SELECT name, artist, year, duration, starred IS NOT NULL,
                    COALESCE((SELECT json_group_array(json_extract(value, '$.name'))
                              FROM json_each(raw_json, '$.recordLabels')), '[]'),
                    CASE WHEN json_array_length(genres) > 0 THEN genres
                         WHEN genre IS NOT NULL THEN json_array(genre) ELSE '[]' END
             FROM albums WHERE id = ?1",
            [album_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, bool>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?;
    let Some((name, artist, year, duration, starred, labels, genres)) = album else { return Ok(None) };

    let (song_count, formats): (i64, String) = conn.query_row(
        "SELECT COUNT(*), COALESCE((SELECT json_group_array(suffix) FROM
                    (SELECT DISTINCT lower(suffix) AS suffix FROM songs
                     WHERE album_id = ?1 AND suffix IS NOT NULL ORDER BY suffix)), '[]')
         FROM songs WHERE album_id = ?1",
        [album_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let list = |json: &str| -> Vec<String> {
        serde_json::from_str::<Vec<Option<String>>>(json)
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter(|text| !text.trim().is_empty())
            .collect()
    };
    Ok(Some(AlbumInfo {
        name,
        artist,
        year,
        labels: list(&labels),
        genres: list(&genres),
        formats: list(&formats),
        song_count,
        duration,
        starred,
    }))
}

/// "Information" of an artist (spec 8.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistInfo {
    pub name: String,
    /// Kept from the artist page (`getArtistInfo2`), if it was opened.
    pub biography: Option<String>,
    pub album_count: i64,
    pub song_count: i64,
    pub starred: bool,
}

pub fn artist_info(conn: &Connection, artist_id: &str) -> DbResult<Option<ArtistInfo>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT name,
                        (SELECT biography FROM artist_info WHERE artist_id = ?1),
                        COALESCE(album_count, (SELECT COUNT(*) FROM albums WHERE artist_id = ?1)),
                        (SELECT COUNT(*) FROM songs WHERE id IN ({ARTIST_SONGS})),
                        starred IS NOT NULL
                 FROM artists WHERE id = ?1"
            ),
            [artist_id],
            |row| {
                Ok(ArtistInfo {
                    name: row.get(0)?,
                    biography: row.get(1)?,
                    album_count: row.get(2)?,
                    song_count: row.get(3)?,
                    starred: row.get(4)?,
                })
            },
        )
        .optional()?)
}

/// The artist's most common genre.
fn artist_genre(conn: &Connection, artist_id: &str) -> DbResult<Option<String>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT COALESCE((SELECT value FROM json_each(genres) LIMIT 1), genre) AS g
                 FROM songs WHERE id IN ({ARTIST_SONGS}) AND g IS NOT NULL
                 GROUP BY g ORDER BY COUNT(*) DESC, g LIMIT 1"
            ),
            [artist_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Radio without Last.fm on the server (spec 8.2): the artist's most played
/// songs mixed with songs of the same genre by other artists, at most `size`.
/// `genre` is used when given (e.g. the song's own), else the artist's most
/// common one.
pub fn radio_fallback(conn: &Connection, artist_id: &str, genre: Option<&str>, size: usize) -> DbResult<Vec<String>> {
    let half = size.div_ceil(2);
    let mut stmt = conn.prepare(&format!(
        "SELECT id FROM songs WHERE id IN ({ARTIST_SONGS})
         ORDER BY COALESCE(play_count, 0) DESC, random() LIMIT ?2"
    ))?;
    let mut ids: Vec<String> =
        stmt.query_map(params![artist_id, half as i64], |row| row.get(0))?.collect::<Result<_, _>>()?;

    let genre = match genre {
        Some(genre) => Some(genre.to_string()),
        None => artist_genre(conn, artist_id)?,
    };
    if let Some(genre) = genre {
        let mut stmt = conn.prepare(&format!(
            "SELECT id FROM songs
             WHERE (genre = ?2 OR EXISTS (SELECT 1 FROM json_each(songs.genres) WHERE value = ?2))
               AND id NOT IN ({ARTIST_SONGS})
             ORDER BY random() LIMIT ?3"
        ))?;
        let rest = size.saturating_sub(ids.len());
        let others = stmt.query_map(params![artist_id, genre, rest as i64], |row| row.get::<_, String>(0))?;
        for id in others {
            ids.push(id?);
        }
    }
    Ok(ids)
}

/// What "Save to computer" names a song's file after (spec 8.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaveSong {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub track: Option<i64>,
    pub disc_number: Option<i64>,
    /// Format, e.g. "flac": the extension.
    pub suffix: Option<String>,
    /// Bytes, to recognize the same file already saved.
    pub size: Option<i64>,
    /// Seconds, for the `.m3u8`.
    pub duration: Option<i64>,
    /// On the server; its extension stands in for `suffix`.
    pub path: Option<String>,
}

/// These songs in the order given, repeats kept (a playlist); songs not in
/// the library are left out.
pub fn save_songs(conn: &Connection, ids: &[String]) -> DbResult<Vec<SaveSong>> {
    let json = serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string());
    let mut stmt = conn.prepare(
        "SELECT id, title, artist, track, disc_number, suffix, size, duration, path
         FROM songs WHERE id IN (SELECT value FROM json_each(?1))",
    )?;
    let found: HashMap<String, SaveSong> = stmt
        .query_map([json], |row| {
            Ok(SaveSong {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                track: row.get(3)?,
                disc_number: row.get(4)?,
                suffix: row.get(5)?,
                size: row.get(6)?,
                duration: row.get(7)?,
                path: row.get(8)?,
            })
        })?
        .map(|row| row.map(|song| (song.id.clone(), song)))
        .collect::<Result<_, _>>()?;
    Ok(ids.iter().filter_map(|id| found.get(id).cloned()).collect())
}

/// A playlist deleted in the app: it leaves the sidebar at once.
pub fn delete_playlist(conn: &Connection, playlist_id: &str) -> DbResult<()> {
    conn.execute("DELETE FROM playlist_songs WHERE playlist_id = ?1", [playlist_id])?;
    conn.execute("DELETE FROM playlists WHERE id = ?1", [playlist_id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::artists::fill_song_artists;
    use crate::db::open_in_memory;

    fn song(conn: &Connection, id: &str, artist_id: &str, album_id: &str, plays: i64, genre: &str, suffix: &str) {
        conn.execute(
            "INSERT INTO songs (id, title, artist, artist_id, album_id, play_count, genre, suffix, size, path, raw_json)
             VALUES (?1, 'Title ' || ?1, 'Name ' || ?2, ?2, ?3, ?4, ?5, ?6, 1000, 'a/b/' || ?1, '{}')",
            params![id, artist_id, album_id, plays, genre, suffix],
        )
        .unwrap();
    }

    fn library() -> Connection {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO artists (id, name, raw_json) VALUES ('ar1', 'Artist', '{}'), ('ar2', 'Other', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO albums (id, name, artist, artist_id, year, genre, duration, raw_json)
             VALUES ('al1', 'Album', 'Artist', 'ar1', 2001, 'Rock', 600,
                     '{\"recordLabels\":[{\"name\":\"Label A\"},{\"name\":\"Label B\"}]}')",
            [],
        )
        .unwrap();
        song(&conn, "s1", "ar1", "al1", 9, "Rock", "flac");
        song(&conn, "s2", "ar1", "al1", 1, "Rock", "FLAC");
        song(&conn, "s3", "ar1", "al1", 5, "Rock", "mp3");
        song(&conn, "o1", "ar2", "al2", 0, "Rock", "mp3");
        song(&conn, "o2", "ar2", "al2", 0, "Jazz", "mp3");
        fill_song_artists(&conn).unwrap();
        conn
    }

    #[test]
    fn song_info_has_the_file_details() {
        let conn = library();
        let info = song_info(&conn, "s1").unwrap().unwrap();
        assert_eq!(info.suffix.as_deref(), Some("flac"));
        assert_eq!(info.size, Some(1000));
        assert_eq!(info.path.as_deref(), Some("a/b/s1"));
        assert_eq!(info.artists, [ArtistRef { id: "ar1".into(), name: "Artist".into() }], "main artist stands in");
        assert_eq!(song_info(&conn, "missing").unwrap(), None);
    }

    #[test]
    fn song_info_lists_every_artist_in_order() {
        let conn = library();
        conn.execute(
            "UPDATE songs SET raw_json = '{\"artists\":[{\"id\":\"ar2\"},{\"id\":\"gone\"},{\"id\":\"ar1\"}]}'
             WHERE id = 's1'",
            [],
        )
        .unwrap();
        let names: Vec<String> = song_info(&conn, "s1").unwrap().unwrap().artists.into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Other", "Artist"], "artists not in the library are left out");
    }

    #[test]
    fn album_info_gathers_labels_genres_and_formats() {
        let conn = library();
        let info = album_info(&conn, "al1").unwrap().unwrap();
        assert_eq!(info.labels, ["Label A", "Label B"]);
        assert_eq!(info.genres, ["Rock"], "the single genre column stands in");
        assert_eq!(info.formats, ["flac", "mp3"], "one per format, whatever the case");
        assert_eq!(info.song_count, 3);
        assert_eq!(info.year, Some(2001));
        assert_eq!(album_info(&conn, "missing").unwrap(), None);
    }

    #[test]
    fn artist_info_counts_albums_and_songs() {
        let conn = library();
        let info = artist_info(&conn, "ar1").unwrap().unwrap();
        assert_eq!((info.album_count, info.song_count), (1, 3));
        assert_eq!(info.biography, None);
    }

    #[test]
    fn radio_fallback_mixes_top_songs_with_the_genre() {
        let conn = library();
        let ids = radio_fallback(&conn, "ar1", None, 4).unwrap();
        assert_eq!(ids[..2], ["s1", "s3"], "the two most played first");
        assert_eq!(ids[2..], ["o1"], "same genre (Rock) from other artists only");

        let jazz = radio_fallback(&conn, "ar1", Some("Jazz"), 4).unwrap();
        assert_eq!(jazz[2..], ["o2"]);
    }

    #[test]
    fn save_songs_keep_the_order_and_repeats() {
        let conn = library();
        let ids = ["s3", "gone", "s1", "s3"].map(String::from);
        let songs = save_songs(&conn, &ids).unwrap();
        let got: Vec<&str> = songs.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(got, ["s3", "s1", "s3"]);
        assert_eq!(songs[1].suffix.as_deref(), Some("flac"));
        assert_eq!(songs[1].size, Some(1000));
    }

    #[test]
    fn deleting_a_playlist_removes_its_songs() {
        let conn = library();
        conn.execute("INSERT INTO playlists (id, name, raw_json) VALUES ('p1', 'Mix', '{}')", []).unwrap();
        conn.execute("INSERT INTO playlist_songs (playlist_id, position, song_id) VALUES ('p1', 0, 's1')", [])
            .unwrap();
        delete_playlist(&conn, "p1").unwrap();
        let left: i64 = conn
            .query_row("SELECT (SELECT COUNT(*) FROM playlists) + (SELECT COUNT(*) FROM playlist_songs)", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(left, 0);
    }
}
