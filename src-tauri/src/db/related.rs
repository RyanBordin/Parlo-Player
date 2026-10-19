//! "Related" tab of Now Playing (spec 4.4): the local sections, around the
//! song playing.

use rusqlite::{Connection, OptionalExtension, params};

use super::DbResult;
use super::home::{ALBUM_CARD_COLUMNS, AlbumCard, album_card_from_row};
use super::library::{SONG_SUMMARY_COLUMNS, SongSummary, song_summary_from_row};

/// What the sections are built around.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongContext {
    pub title: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub album_id: Option<String>,
    /// The song's first genre.
    pub genre: Option<String>,
}

/// The song's artist, album and genre, `None` if it is not in the library.
pub fn song_context(conn: &Connection, song_id: &str) -> DbResult<Option<SongContext>> {
    Ok(conn
        .query_row(
            "SELECT title, artist, artist_id, album_id,
                    COALESCE((SELECT value FROM json_each(genres) LIMIT 1), genre)
             FROM songs WHERE id = ?1",
            [song_id],
            |row| {
                Ok(SongContext {
                    title: row.get(0)?,
                    artist: row.get(1)?,
                    artist_id: row.get(2)?,
                    album_id: row.get(3)?,
                    genre: row.get(4)?,
                })
            },
        )
        .optional()?)
}

/// "More from [Artist]": the artist's songs, most played first, without the
/// song playing.
pub fn artist_top_songs(conn: &Connection, artist_id: &str, except_song: &str) -> DbResult<Vec<SongSummary>> {
    let sql = format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs WHERE artist_id = ?1 AND id <> ?2
         ORDER BY COALESCE(play_count, 0) DESC, julianday(played) DESC NULLS LAST, title"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map(params![artist_id, except_song], song_summary_from_row)?.collect::<Result<_, _>>()?;
    Ok(songs)
}

/// "Albums by [Artist]": newest first.
pub fn artist_albums(conn: &Connection, artist_id: &str, limit: u32) -> DbResult<Vec<AlbumCard>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums WHERE artist_id = ?1
         ORDER BY year DESC NULLS LAST, name LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt.query_map(params![artist_id, limit], album_card_from_row)?.collect::<Result<_, _>>()?;
    Ok(albums)
}

/// "Same genre ([Genre])": albums of that genre, most played first, without
/// the album playing.
pub fn genre_albums(conn: &Connection, genre: &str, except_album: Option<&str>, limit: u32) -> DbResult<Vec<AlbumCard>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums
         WHERE (genre = ?1 OR EXISTS (SELECT 1 FROM json_each(albums.genres) WHERE value = ?1))
           AND id IS NOT ?2
         ORDER BY COALESCE(play_count, 0) DESC, julianday(played) DESC NULLS LAST, name LIMIT ?3"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt.query_map(params![genre, except_album, limit], album_card_from_row)?.collect::<Result<_, _>>()?;
    Ok(albums)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn song(conn: &Connection, id: &str, artist: &str, plays: Option<i64>, genres: &str) {
        conn.execute(
            "INSERT INTO songs (id, title, artist, artist_id, album_id, play_count, genre, genres, raw_json)
             VALUES (?1, ?1, 'Name ' || ?2, ?2, 'al1', ?3, 'Old genre', ?4, '{}')",
            params![id, artist, plays, genres],
        )
        .unwrap();
    }

    fn album(conn: &Connection, id: &str, artist: &str, year: Option<i64>, genres: &str, plays: Option<i64>) {
        conn.execute(
            "INSERT INTO albums (id, name, artist_id, year, genres, play_count, raw_json)
             VALUES (?1, ?1, ?2, ?3, ?4, ?5, '{}')",
            params![id, artist, year, genres, plays],
        )
        .unwrap();
    }

    #[test]
    fn the_context_takes_the_first_genre() {
        let conn = open_in_memory().unwrap();
        song(&conn, "s1", "ar1", None, r#"["Rock","Pop"]"#);
        song(&conn, "s2", "ar1", None, "[]");
        let context = song_context(&conn, "s1").unwrap().unwrap();
        assert_eq!(context.artist_id.as_deref(), Some("ar1"));
        assert_eq!(context.genre.as_deref(), Some("Rock"));
        assert_eq!(song_context(&conn, "s2").unwrap().unwrap().genre.as_deref(), Some("Old genre"));
        assert_eq!(song_context(&conn, "missing").unwrap(), None);
    }

    #[test]
    fn more_from_the_artist_is_most_played_first_without_the_current_song() {
        let conn = open_in_memory().unwrap();
        song(&conn, "current", "ar1", Some(99), "[]");
        song(&conn, "often", "ar1", Some(10), "[]");
        song(&conn, "never", "ar1", None, "[]");
        song(&conn, "other-artist", "ar2", Some(50), "[]");
        let ids: Vec<String> = artist_top_songs(&conn, "ar1", "current").unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["often", "never"]);
    }

    #[test]
    fn albums_by_the_artist_and_of_the_genre() {
        let conn = open_in_memory().unwrap();
        album(&conn, "old", "ar1", Some(1999), r#"["Rock"]"#, Some(1));
        album(&conn, "new", "ar1", Some(2020), r#"["Jazz"]"#, None);
        album(&conn, "hit", "ar2", Some(2010), r#"["Rock","Pop"]"#, Some(30));
        album(&conn, "playing", "ar2", Some(2011), r#"["Rock"]"#, Some(90));

        let by_artist: Vec<String> = artist_albums(&conn, "ar1", 20).unwrap().into_iter().map(|a| a.id).collect();
        assert_eq!(by_artist, ["new", "old"]);

        let rock: Vec<String> = genre_albums(&conn, "Rock", Some("playing"), 20).unwrap().into_iter().map(|a| a.id).collect();
        assert_eq!(rock, ["hit", "old"], "most played first, without the album playing");
    }
}
