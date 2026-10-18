//! Reads for the Home screen (spec 6.1) from the local cache: favorites,
//! play history and the fallbacks used when the server cannot be reached.

use rusqlite::{Connection, params};
use serde::Serialize;

use super::DbResult;
use super::library::{SONG_SUMMARY_COLUMNS, SongSummary, song_summary_from_row};

/// Album fields shown on a card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumCard {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art: Option<String>,
}

/// Artist fields shown on a card. Without a photo on the server, the card
/// shows `fallback_cover_art`: the cover of the artist's latest album.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistCard {
    pub id: String,
    pub name: String,
    pub cover_art: Option<String>,
    pub fallback_cover_art: Option<String>,
}

const ALBUM_CARD_COLUMNS: &str = "id, name, artist, artist_id, cover_art";

fn album_card_from_row(row: &rusqlite::Row) -> rusqlite::Result<AlbumCard> {
    Ok(AlbumCard {
        id: row.get(0)?,
        name: row.get(1)?,
        artist: row.get(2)?,
        artist_id: row.get(3)?,
        cover_art: row.get(4)?,
    })
}

/// Starred songs, last starred first.
pub fn starred_songs(conn: &Connection) -> DbResult<Vec<SongSummary>> {
    songs(conn, &format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs WHERE starred IS NOT NULL ORDER BY starred DESC, title"
    ))
}

/// "Forgotten favorites": starred songs played the least (no plays counts as
/// 0); on a tie, the ones played longest ago, never played first.
pub fn forgotten_starred_songs(conn: &Connection) -> DbResult<Vec<SongSummary>> {
    songs(conn, &format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs WHERE starred IS NOT NULL
         ORDER BY COALESCE(play_count, 0), played IS NOT NULL, played, title"
    ))
}

/// Starred albums, last starred first.
pub fn starred_albums(conn: &Connection, limit: u32) -> DbResult<Vec<AlbumCard>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums WHERE starred IS NOT NULL
         ORDER BY starred DESC, name LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt.query_map([limit], album_card_from_row)?.collect::<Result<_, _>>()?;
    Ok(albums)
}

/// Starred artists, last starred first, each with its latest album's cover
/// (highest year; on a tie, the last added) as the photo fallback.
pub fn starred_artists(conn: &Connection, limit: u32) -> DbResult<Vec<ArtistCard>> {
    let mut stmt = conn.prepare(
        "SELECT ar.id, ar.name, ar.cover_art,
                (SELECT al.cover_art FROM albums al
                 WHERE al.artist_id = ar.id AND al.cover_art IS NOT NULL
                 ORDER BY al.year DESC NULLS LAST, al.created DESC LIMIT 1)
         FROM artists ar WHERE ar.starred IS NOT NULL
         ORDER BY ar.starred DESC, ar.name LIMIT ?1",
    )?;
    let artists = stmt
        .query_map([limit], |row| {
            Ok(ArtistCard {
                id: row.get(0)?,
                name: row.get(1)?,
                cover_art: row.get(2)?,
                fallback_cover_art: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(artists)
}

/// Songs of the local history, each once, by its last play (Unix seconds),
/// newest first. Songs no longer in the library are left out.
pub fn recently_played_songs(conn: &Connection, limit: u32) -> DbResult<Vec<(SongSummary, i64)>> {
    let sql = format!(
        "SELECT {SONG_SUMMARY_COLUMNS}, last_played FROM songs
         JOIN (SELECT song_id, MAX(played_at) AS last_played FROM play_history GROUP BY song_id)
           ON song_id = songs.id
         ORDER BY last_played DESC LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt
        .query_map([limit], |row| Ok((song_summary_from_row(row)?, row.get(9)?)))?
        .collect::<Result<_, _>>()?;
    Ok(songs)
}

/// Offline fallback of "Listen again": albums by last play (ISO date, as the
/// server writes it), newest first.
pub fn recently_played_albums(conn: &Connection, limit: u32) -> DbResult<Vec<(AlbumCard, String)>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS}, played FROM albums WHERE played IS NOT NULL
         ORDER BY played DESC LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt
        .query_map([limit], |row| Ok((album_card_from_row(row)?, row.get(5)?)))?
        .collect::<Result<_, _>>()?;
    Ok(albums)
}

/// Offline fallback of "Most played albums": play count, then last play.
pub fn most_played_albums(conn: &Connection, limit: u32) -> DbResult<Vec<AlbumCard>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums WHERE COALESCE(play_count, 0) > 0
         ORDER BY play_count DESC, played DESC, name LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt.query_map([limit], album_card_from_row)?.collect::<Result<_, _>>()?;
    Ok(albums)
}

/// An album's name and its songs in order: disc, then track number.
pub fn album_songs(conn: &Connection, album_id: &str) -> DbResult<(Option<String>, Vec<String>)> {
    let name = conn
        .query_row("SELECT name FROM albums WHERE id = ?1", [album_id], |row| row.get(0))
        .ok();
    let mut stmt = conn.prepare(
        "SELECT id FROM songs WHERE album_id = ?1
         ORDER BY COALESCE(disc_number, 1), COALESCE(track, 0), title",
    )?;
    let ids = stmt.query_map(params![album_id], |row| row.get(0))?.collect::<Result<_, _>>()?;
    Ok((name, ids))
}

fn songs(conn: &Connection, sql: &str) -> DbResult<Vec<SongSummary>> {
    let mut stmt = conn.prepare(sql)?;
    let songs = stmt.query_map([], song_summary_from_row)?.collect::<Result<_, _>>()?;
    Ok(songs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    /// A song row with only what these reads look at.
    fn song(conn: &Connection, id: &str, album: &str, starred: Option<&str>, plays: Option<i64>, played: Option<&str>) {
        conn.execute(
            "INSERT INTO songs (id, title, album_id, starred, play_count, played, raw_json)
             VALUES (?1, ?1, ?2, ?3, ?4, ?5, '{}')",
            params![id, album, starred, plays, played],
        )
        .unwrap();
    }

    fn album(conn: &Connection, id: &str, artist: &str, year: Option<i64>, created: &str) {
        conn.execute(
            "INSERT INTO albums (id, name, artist_id, cover_art, year, created, raw_json)
             VALUES (?1, ?1, ?2, 'al-' || ?1, ?3, ?4, '{}')",
            params![id, artist, year, created],
        )
        .unwrap();
    }

    fn ids(songs: &[SongSummary]) -> Vec<&str> {
        songs.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn starred_songs_come_last_starred_first() {
        let conn = open_in_memory().unwrap();
        song(&conn, "old", "al1", Some("2026-01-01T00:00:00Z"), None, None);
        song(&conn, "new", "al1", Some("2026-05-01T00:00:00Z"), None, None);
        song(&conn, "not", "al1", None, None, None);
        assert_eq!(ids(&starred_songs(&conn).unwrap()), ["new", "old"]);
    }

    #[test]
    fn forgotten_favorites_are_least_played_then_played_longest_ago() {
        let conn = open_in_memory().unwrap();
        let star = Some("2026-01-01T00:00:00Z");
        song(&conn, "often", "al1", star, Some(30), Some("2026-10-01T00:00:00Z"));
        song(&conn, "twice-recent", "al1", star, Some(2), Some("2026-10-10T00:00:00Z"));
        song(&conn, "twice-old", "al1", star, Some(2), Some("2026-02-01T00:00:00Z"));
        song(&conn, "never", "al1", star, None, None);
        song(&conn, "zero-played", "al1", star, Some(0), Some("2026-03-01T00:00:00Z"));
        song(&conn, "not-starred", "al1", None, Some(0), None);

        assert_eq!(
            ids(&forgotten_starred_songs(&conn).unwrap()),
            ["never", "zero-played", "twice-old", "twice-recent", "often"]
        );
    }

    #[test]
    fn artist_cards_fall_back_to_the_latest_album() {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO artists (id, name, cover_art, starred, raw_json)
             VALUES ('ar1', 'Artist', 'ar-ar1', '2026-01-01T00:00:00Z', '{}'),
                    ('ar2', 'Other', 'ar-ar2', NULL, '{}')",
            [],
        )
        .unwrap();
        album(&conn, "first", "ar1", Some(2001), "2026-01-01T00:00:00Z");
        album(&conn, "latest", "ar1", Some(2019), "2026-01-01T00:00:00Z");
        album(&conn, "undated", "ar1", None, "2026-09-01T00:00:00Z");

        let artists = starred_artists(&conn, 20).unwrap();
        assert_eq!(artists.len(), 1, "only starred artists");
        assert_eq!(artists[0].cover_art.as_deref(), Some("ar-ar1"));
        assert_eq!(artists[0].fallback_cover_art.as_deref(), Some("al-latest"));
    }

    #[test]
    fn history_gives_each_song_once_by_its_last_play() {
        let conn = open_in_memory().unwrap();
        song(&conn, "s1", "al1", None, None, None);
        song(&conn, "s2", "al1", None, None, None);
        conn.execute(
            "INSERT INTO play_history (song_id, played_at) VALUES
             ('s1', 100), ('s2', 200), ('s1', 300), ('gone', 400)",
            [],
        )
        .unwrap();

        let played = recently_played_songs(&conn, 20).unwrap();
        let summary: Vec<(&str, i64)> = played.iter().map(|(s, at)| (s.id.as_str(), *at)).collect();
        assert_eq!(summary, [("s1", 300), ("s2", 200)], "songs not in the library are left out");
    }

    #[test]
    fn local_fallbacks_order_albums_by_play() {
        let conn = open_in_memory().unwrap();
        album(&conn, "a", "ar1", None, "2026-01-01T00:00:00Z");
        album(&conn, "b", "ar1", None, "2026-01-01T00:00:00Z");
        album(&conn, "c", "ar1", None, "2026-01-01T00:00:00Z");
        conn.execute_batch(
            "UPDATE albums SET play_count = 5, played = '2026-10-01T00:00:00Z' WHERE id = 'a';
             UPDATE albums SET play_count = 9, played = '2026-09-01T00:00:00Z' WHERE id = 'b';",
        )
        .unwrap();

        let recent: Vec<String> = recently_played_albums(&conn, 20).unwrap().into_iter().map(|(a, _)| a.id).collect();
        assert_eq!(recent, ["a", "b"]);
        let frequent: Vec<String> = most_played_albums(&conn, 20).unwrap().into_iter().map(|a| a.id).collect();
        assert_eq!(frequent, ["b", "a"], "never played albums are left out");
    }

    #[test]
    fn album_songs_follow_disc_and_track() {
        let conn = open_in_memory().unwrap();
        album(&conn, "al1", "ar1", None, "2026-01-01T00:00:00Z");
        conn.execute_batch(
            "INSERT INTO songs (id, title, album_id, disc_number, track, raw_json) VALUES
             ('d2t1', 'x', 'al1', 2, 1, '{}'),
             ('d1t2', 'x', 'al1', 1, 2, '{}'),
             ('d1t1', 'x', 'al1', 1, 1, '{}'),
             ('other', 'x', 'al2', 1, 1, '{}');",
        )
        .unwrap();
        let (name, ids) = album_songs(&conn, "al1").unwrap();
        assert_eq!(name.as_deref(), Some("al1"));
        assert_eq!(ids, ["d1t1", "d1t2", "d2t1"]);
    }
}
