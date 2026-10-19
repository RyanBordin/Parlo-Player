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

pub(super) const ALBUM_CARD_COLUMNS: &str = "id, name, artist, artist_id, cover_art";

pub(super) fn album_card_from_row(row: &rusqlite::Row) -> rusqlite::Result<AlbumCard> {
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
pub fn starred_albums(conn: &Connection, limit: Option<u32>) -> DbResult<Vec<AlbumCard>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums WHERE starred IS NOT NULL
         ORDER BY starred DESC, name LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt.query_map([sql_limit(limit)], album_card_from_row)?.collect::<Result<_, _>>()?;
    Ok(albums)
}

/// Starred artists, last starred first, each with its latest album's cover
/// (highest year; on a tie, the last added) as the photo fallback.
pub fn starred_artists(conn: &Connection, limit: Option<u32>) -> DbResult<Vec<ArtistCard>> {
    let mut stmt = conn.prepare(
        "SELECT ar.id, ar.name, ar.cover_art,
                (SELECT al.cover_art FROM albums al
                 WHERE al.artist_id = ar.id AND al.cover_art IS NOT NULL
                 ORDER BY al.year DESC NULLS LAST, al.created DESC LIMIT 1)
         FROM artists ar WHERE ar.starred IS NOT NULL
         ORDER BY ar.starred DESC, ar.name LIMIT ?1",
    )?;
    let artists = stmt
        .query_map([sql_limit(limit)], |row| {
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

/// Songs last played in the 30 days before `now` (Unix seconds), most played
/// first; on a tie, the most recently played. `played` comes from the server
/// (plays in other apps too) and from the plays counted here. SQLite reads
/// both date formats (with or without fractions of a second).
pub fn recently_played_top_songs(conn: &Connection, now: i64) -> DbResult<Vec<SongSummary>> {
    let sql = format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs
         WHERE julianday(played) >= julianday(?1, 'unixepoch', '-30 days')
         ORDER BY COALESCE(play_count, 0) DESC, julianday(played) DESC, title"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map([now], song_summary_from_row)?.collect::<Result<_, _>>()?;
    Ok(songs)
}

/// Most played songs of all time (at least one play), then the most recent.
pub fn most_played_songs(conn: &Connection, limit: Option<u32>) -> DbResult<Vec<SongSummary>> {
    let sql = format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs WHERE COALESCE(play_count, 0) > 0
         ORDER BY play_count DESC, julianday(played) DESC NULLS LAST, title LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map([sql_limit(limit)], song_summary_from_row)?.collect::<Result<_, _>>()?;
    Ok(songs)
}

/// Offline fallback of "Most played albums": play count, then last play.
pub fn most_played_albums(conn: &Connection, limit: Option<u32>) -> DbResult<Vec<AlbumCard>> {
    let sql = format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums WHERE COALESCE(play_count, 0) > 0
         ORDER BY play_count DESC, played DESC, name LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let albums = stmt.query_map([sql_limit(limit)], album_card_from_row)?.collect::<Result<_, _>>()?;
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

/// SQLite reads `LIMIT -1` as no limit.
fn sql_limit(limit: Option<u32>) -> i64 {
    limit.map_or(-1, i64::from)
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

        let artists = starred_artists(&conn, Some(20)).unwrap();
        assert_eq!(artists.len(), 1, "only starred artists");
        assert_eq!(starred_artists(&conn, None).unwrap(), artists, "no limit gives every row");
        assert_eq!(artists[0].cover_art.as_deref(), Some("ar-ar1"));
        assert_eq!(artists[0].fallback_cover_art.as_deref(), Some("al-latest"));
    }

    /// 2026-10-18T12:00:00Z.
    const NOW: i64 = 1_792_324_800;

    #[test]
    fn listen_again_takes_the_last_30_days_by_play_count() {
        let conn = open_in_memory().unwrap();
        // Server dates carry nanoseconds; the app's own have whole seconds.
        song(&conn, "yesterday", "al1", None, Some(3), Some("2026-10-17T08:00:00.123456789Z"));
        song(&conn, "this-month", "al1", None, Some(10), Some("2026-10-01T00:00:00Z"));
        song(&conn, "same-count", "al1", None, Some(3), Some("2026-10-02T00:00:00Z"));
        song(&conn, "31-days-ago", "al1", None, Some(50), Some("2026-09-17T12:00:00Z"));
        song(&conn, "never", "al1", None, None, None);

        assert_eq!(
            ids(&recently_played_top_songs(&conn, NOW).unwrap()),
            ["this-month", "yesterday", "same-count"],
            "most played first; on a tie, the most recent"
        );
    }

    #[test]
    fn most_played_songs_skip_the_never_played() {
        let conn = open_in_memory().unwrap();
        song(&conn, "often", "al1", None, Some(50), Some("2026-01-01T00:00:00Z"));
        song(&conn, "once", "al1", None, Some(1), Some("2026-10-01T00:00:00Z"));
        song(&conn, "zero", "al1", None, Some(0), None);
        song(&conn, "never", "al1", None, None, None);
        assert_eq!(ids(&most_played_songs(&conn, None).unwrap()), ["often", "once"]);
        assert_eq!(ids(&most_played_songs(&conn, Some(1)).unwrap()), ["often"]);
    }

    #[test]
    fn local_fallback_orders_albums_by_play_count() {
        let conn = open_in_memory().unwrap();
        album(&conn, "a", "ar1", None, "2026-01-01T00:00:00Z");
        album(&conn, "b", "ar1", None, "2026-01-01T00:00:00Z");
        album(&conn, "c", "ar1", None, "2026-01-01T00:00:00Z");
        conn.execute_batch(
            "UPDATE albums SET play_count = 5, played = '2026-10-01T00:00:00Z' WHERE id = 'a';
             UPDATE albums SET play_count = 9, played = '2026-09-01T00:00:00Z' WHERE id = 'b';",
        )
        .unwrap();

        let frequent: Vec<String> = most_played_albums(&conn, Some(20)).unwrap().into_iter().map(|a| a.id).collect();
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
