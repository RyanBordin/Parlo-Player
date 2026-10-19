//! The Albums page (spec 6.4): every album with the fields its cards, quick
//! search and "Sort by" need, and the songs of several albums at once.

use rusqlite::Connection;
use serde::Serialize;

use super::DbResult;

/// An album of the Albums grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumListItem {
    pub id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art: Option<String>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    pub genres: Vec<String>,
    /// Date added to the server (ISO 8601).
    pub created: Option<String>,
    pub starred: bool,
    pub play_count: Option<i64>,
    /// Last play (ISO 8601).
    pub played: Option<String>,
}

/// Every album, last added first (the page's default order).
pub fn album_list(conn: &Connection) -> DbResult<Vec<AlbumListItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, sort_name, artist, artist_id, cover_art, year, genre, genres, created,
                starred IS NOT NULL, play_count, played
         FROM albums ORDER BY julianday(created) DESC, name",
    )?;
    let albums = stmt
        .query_map([], |row| {
            let genres: String = row.get(8)?;
            Ok(AlbumListItem {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_name: row.get(2)?,
                artist: row.get(3)?,
                artist_id: row.get(4)?,
                cover_art: row.get(5)?,
                year: row.get(6)?,
                genre: row.get(7)?,
                genres: serde_json::from_str(&genres).unwrap_or_default(),
                created: row.get(9)?,
                starred: row.get(10)?,
                play_count: row.get(11)?,
                played: row.get(12)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(albums)
}

/// The songs of each album, in the order of `album_ids`, each album in disc
/// and track order. One list per album (empty if none of its songs are in
/// the library).
pub fn songs_of_albums(conn: &Connection, album_ids: &[String]) -> DbResult<Vec<Vec<String>>> {
    let ids = serde_json::to_string(album_ids).unwrap_or_else(|_| "[]".into());
    // `json_each.key` is the position in the list, so albums keep their order.
    let mut stmt = conn.prepare(
        "SELECT j.key, s.id FROM json_each(?1) AS j JOIN songs AS s ON s.album_id = j.value
         ORDER BY j.key, COALESCE(s.disc_number, 1), COALESCE(s.track, 0), s.title",
    )?;
    let mut albums = vec![Vec::new(); album_ids.len()];
    let rows = stmt.query_map([ids], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (position, song_id) = row?;
        if let Some(songs) = albums.get_mut(position as usize) {
            songs.push(song_id);
        }
    }
    Ok(albums)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use rusqlite::params;

    fn album(conn: &Connection, id: &str, created: &str) {
        conn.execute(
            "INSERT INTO albums (id, name, sort_name, artist, year, genres, created, play_count, played, raw_json)
             VALUES (?1, ?1, 'sort ' || ?1, 'Artist', 2001, '[\"Jazz\"]', ?2, 4, '2026-10-01T00:00:00Z', '{}')",
            params![id, created],
        )
        .unwrap();
    }

    fn song(conn: &Connection, id: &str, album_id: &str, disc: i64, track: i64) {
        conn.execute(
            "INSERT INTO songs (id, title, album_id, disc_number, track, raw_json) VALUES (?1, ?1, ?2, ?3, ?4, '{}')",
            params![id, album_id, disc, track],
        )
        .unwrap();
    }

    #[test]
    fn album_list_has_the_sort_fields_last_added_first() {
        let conn = open_in_memory().unwrap();
        album(&conn, "old", "2026-01-01T00:00:00Z");
        // Server dates can carry nanoseconds.
        album(&conn, "new", "2026-07-21T17:10:32.999752282Z");

        let albums = album_list(&conn).unwrap();
        assert_eq!(albums.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["new", "old"]);
        let first = &albums[0];
        assert_eq!(first.sort_name.as_deref(), Some("sort new"));
        assert_eq!(first.genres, ["Jazz"]);
        assert_eq!((first.year, first.play_count), (Some(2001), Some(4)));
        assert_eq!(first.played.as_deref(), Some("2026-10-01T00:00:00Z"));
        assert!(!first.starred);
    }

    #[test]
    fn songs_of_albums_keep_the_album_order_and_each_album_in_order() {
        let conn = open_in_memory().unwrap();
        song(&conn, "a2", "al-a", 1, 2);
        song(&conn, "a1", "al-a", 1, 1);
        song(&conn, "b-d2", "al-b", 2, 1);
        song(&conn, "b-d1", "al-b", 1, 9);

        let ids = ["al-b".to_string(), "al-empty".to_string(), "al-a".to_string()];
        assert_eq!(
            songs_of_albums(&conn, &ids).unwrap(),
            [vec!["b-d1", "b-d2"], vec![], vec!["a1", "a2"]]
        );
    }
}
