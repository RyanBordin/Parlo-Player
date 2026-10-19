//! Album and playlist pages (spec 6.5): the songs of a tracklist with every
//! field the quick search looks at (spec 7), and the page headers.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::DbResult;
use crate::subsonic::Song;

/// A line of a tracklist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackListSong {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub album_artist: Option<String>,
    pub composer: Option<String>,
    pub cover_art: Option<String>,
    pub track: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    pub genres: Vec<String>,
    /// Seconds.
    pub duration: Option<i64>,
    pub suffix: Option<String>,
    /// kbps.
    pub bit_rate: Option<i64>,
    pub path: Option<String>,
    pub starred: bool,
}

impl From<&Song> for TrackListSong {
    fn from(song: &Song) -> Self {
        Self {
            id: song.id.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
            artist_id: song.artist_id.clone(),
            album: song.album.clone(),
            album_id: song.album_id.clone(),
            album_artist: song.display_album_artist.clone(),
            composer: song.display_composer.clone(),
            cover_art: song.cover_art.clone(),
            track: song.track,
            disc_number: song.disc_number,
            year: song.year,
            genre: song.genre.clone(),
            genres: song.genres.iter().map(|g| g.name.clone()).collect(),
            duration: song.duration,
            suffix: song.suffix.clone(),
            bit_rate: song.bit_rate,
            path: song.path.clone(),
            starred: song.starred.is_some(),
        }
    }
}

const TRACK_COLUMNS: &str = "id, title, artist, artist_id, album, album_id, album_artist, composer,
    cover_art, track, disc_number, year, genre, genres, duration, suffix, bit_rate, path,
    starred IS NOT NULL";

fn track_from_row(row: &rusqlite::Row) -> rusqlite::Result<TrackListSong> {
    let genres: String = row.get(13)?;
    Ok(TrackListSong {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        artist_id: row.get(3)?,
        album: row.get(4)?,
        album_id: row.get(5)?,
        album_artist: row.get(6)?,
        composer: row.get(7)?,
        cover_art: row.get(8)?,
        track: row.get(9)?,
        disc_number: row.get(10)?,
        year: row.get(11)?,
        genre: row.get(12)?,
        genres: serde_json::from_str(&genres).unwrap_or_default(),
        duration: row.get(14)?,
        suffix: row.get(15)?,
        bit_rate: row.get(16)?,
        path: row.get(17)?,
        starred: row.get(18)?,
    })
}

/// Header of an album page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumHeader {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art: Option<String>,
    pub song_count: Option<i64>,
    /// Seconds.
    pub duration: Option<i64>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    pub genres: Vec<String>,
    /// Date added to the server (ISO 8601).
    pub created: Option<String>,
    pub starred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPage {
    pub album: AlbumHeader,
    /// Disc, then track order.
    pub songs: Vec<TrackListSong>,
}

/// An album and its songs, `None` if it is not in the library.
pub fn album_page(conn: &Connection, album_id: &str) -> DbResult<Option<AlbumPage>> {
    let album = conn
        .query_row(
            "SELECT id, name, artist, artist_id, cover_art, song_count, duration, year, genre,
                    genres, created, starred IS NOT NULL
             FROM albums WHERE id = ?1",
            [album_id],
            |row| {
                let genres: String = row.get(9)?;
                Ok(AlbumHeader {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    artist: row.get(2)?,
                    artist_id: row.get(3)?,
                    cover_art: row.get(4)?,
                    song_count: row.get(5)?,
                    duration: row.get(6)?,
                    year: row.get(7)?,
                    genre: row.get(8)?,
                    genres: serde_json::from_str(&genres).unwrap_or_default(),
                    created: row.get(10)?,
                    starred: row.get(11)?,
                })
            },
        )
        .optional()?;
    let Some(album) = album else { return Ok(None) };

    let sql = format!(
        "SELECT {TRACK_COLUMNS} FROM songs WHERE album_id = ?1
         ORDER BY COALESCE(disc_number, 1), COALESCE(track, 0), title"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map([album_id], track_from_row)?.collect::<Result<_, _>>()?;
    Ok(Some(AlbumPage { album, songs }))
}

/// Header of a playlist page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistHeader {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    pub cover_art: Option<String>,
    pub song_count: Option<i64>,
    /// Seconds.
    pub duration: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistPage {
    pub playlist: PlaylistHeader,
    /// Saved order; a song can appear more than once.
    pub songs: Vec<TrackListSong>,
    /// Read from the local cache because the server could not be reached.
    pub offline: bool,
}

/// Keeps the saved order of a playlist just read from the server.
pub fn save_playlist_songs(conn: &mut Connection, playlist_id: &str, song_ids: &[String]) -> DbResult<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM playlist_songs WHERE playlist_id = ?1", [playlist_id])?;
    {
        let mut insert =
            tx.prepare("INSERT INTO playlist_songs (playlist_id, position, song_id) VALUES (?1, ?2, ?3)")?;
        for (position, song_id) in song_ids.iter().enumerate() {
            insert.execute(params![playlist_id, position as i64, song_id])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// The playlist from the local cache (offline): the header of the last light
/// sync and the songs of the last `getPlaylist`, without the songs that are
/// not in the library. `None` if the playlist is unknown.
pub fn cached_playlist_page(conn: &Connection, playlist_id: &str) -> DbResult<Option<PlaylistPage>> {
    let playlist = conn
        .query_row(
            "SELECT id, name, owner, cover_art, song_count, duration FROM playlists WHERE id = ?1",
            [playlist_id],
            |row| {
                Ok(PlaylistHeader {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    owner: row.get(2)?,
                    cover_art: row.get(3)?,
                    song_count: row.get(4)?,
                    duration: row.get(5)?,
                })
            },
        )
        .optional()?;
    let Some(playlist) = playlist else { return Ok(None) };

    let sql = format!(
        "SELECT {TRACK_COLUMNS} FROM playlist_songs JOIN songs ON songs.id = playlist_songs.song_id
         WHERE playlist_id = ?1 ORDER BY position"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map([playlist_id], track_from_row)?.collect::<Result<_, _>>()?;
    Ok(Some(PlaylistPage { playlist, songs, offline: true }))
}

/// Records a star / unstar of an album done in the app (see
/// `library::set_song_starred`).
pub fn set_album_starred(conn: &Connection, album_id: &str, starred: bool) -> DbResult<()> {
    if starred {
        conn.execute(
            "UPDATE albums SET starred = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                               raw_json = json_set(raw_json, '$.starred', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
             WHERE id = ?1",
            [album_id],
        )?;
    } else {
        conn.execute(
            "UPDATE albums SET starred = NULL, raw_json = json_remove(raw_json, '$.starred') WHERE id = ?1",
            [album_id],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn album(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO albums (id, name, artist, genres, created, raw_json)
             VALUES (?1, 'Album', 'Various Artists', '[\"Rock\",\"Pop\"]', '2026-07-21T17:10:32Z', '{}')",
            [id],
        )
        .unwrap();
    }

    fn song(conn: &Connection, id: &str, album_id: &str, disc: i64, track: i64) {
        conn.execute(
            "INSERT INTO songs (id, title, album_id, artist, disc_number, track, genres, raw_json)
             VALUES (?1, ?1, ?2, 'Artist ' || ?1, ?3, ?4, '[\"Rock\"]', '{}')",
            params![id, album_id, disc, track],
        )
        .unwrap();
    }

    fn ids(songs: &[TrackListSong]) -> Vec<&str> {
        songs.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn album_page_has_its_header_and_songs_in_disc_and_track_order() {
        let conn = open_in_memory().unwrap();
        album(&conn, "al1");
        song(&conn, "d2t1", "al1", 2, 1);
        song(&conn, "d1t2", "al1", 1, 2);
        song(&conn, "d1t1", "al1", 1, 1);
        song(&conn, "other", "al2", 1, 1);

        let page = album_page(&conn, "al1").unwrap().unwrap();
        assert_eq!(page.album.genres, ["Rock", "Pop"]);
        assert_eq!(page.album.artist.as_deref(), Some("Various Artists"));
        assert_eq!(ids(&page.songs), ["d1t1", "d1t2", "d2t1"]);
        assert_eq!(page.songs[0].genres, ["Rock"]);
        assert_eq!(page.songs[0].artist.as_deref(), Some("Artist d1t1"), "the song's own artist");
        assert_eq!(album_page(&conn, "missing").unwrap(), None);
    }

    #[test]
    fn cached_playlist_keeps_the_order_and_repeats_and_skips_missing_songs() {
        let mut conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO playlists (id, name, owner, song_count, raw_json) VALUES ('p1', 'Mix', 'ryan', 4, '{}')",
            [],
        )
        .unwrap();
        song(&conn, "a", "al1", 1, 1);
        song(&conn, "b", "al1", 1, 2);
        let order = ["b", "a", "gone", "b"].map(String::from);
        save_playlist_songs(&mut conn, "p1", &order).unwrap();

        let page = cached_playlist_page(&conn, "p1").unwrap().unwrap();
        assert_eq!(page.playlist.owner.as_deref(), Some("ryan"));
        assert_eq!(ids(&page.songs), ["b", "a", "b"]);
        assert!(page.offline);

        // A new read replaces the old order.
        save_playlist_songs(&mut conn, "p1", &["a".to_string()]).unwrap();
        assert_eq!(ids(&cached_playlist_page(&conn, "p1").unwrap().unwrap().songs), ["a"]);
        assert_eq!(cached_playlist_page(&conn, "unknown").unwrap(), None);
    }

    #[test]
    fn starring_an_album_updates_the_column_and_the_json() {
        let conn = open_in_memory().unwrap();
        album(&conn, "al1");
        set_album_starred(&conn, "al1", true).unwrap();
        assert!(album_page(&conn, "al1").unwrap().unwrap().album.starred);
        let raw: String = conn.query_row("SELECT raw_json FROM albums", [], |r| r.get(0)).unwrap();
        assert!(raw.contains("starred"));

        set_album_starred(&conn, "al1", false).unwrap();
        assert!(!album_page(&conn, "al1").unwrap().unwrap().album.starred);
    }
}
