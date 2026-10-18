//! Library cache: writes from the syncs and reads for the UI.
//!
//! Full sync, safe against failures:
//! 1. `begin_full_sync` creates empty TEMP staging copies of artists/albums/songs.
//! 2. `stage` writes each downloaded page to staging, one transaction per page.
//!    The real tables are not touched.
//! 3. `finish_full_sync` upserts staging into the real tables and deletes what
//!    the server no longer has, all in one transaction.
//!
//! If the sync fails before step 3, the staging tables are dropped (TEMP tables
//! also vanish when the connection closes) and the previous cache is intact.

use std::collections::HashMap;

use rusqlite::{Connection, Statement, params};
use serde::Serialize;

use super::DbResult;
use crate::subsonic::{Album, Artist, Genre, LibraryItems, Playlist, Song};

/// A table and its columns, in the order the row writers bind them.
struct Table {
    name: &'static str,
    columns: &'static [&'static str],
}

const ARTISTS: Table = Table {
    name: "artists",
    columns: &[
        "id", "name", "sort_name", "album_count", "cover_art", "artist_image_url", "starred",
        "music_brainz_id", "roles", "raw_json",
    ],
};

const ALBUMS: Table = Table {
    name: "albums",
    columns: &[
        "id", "name", "sort_name", "artist", "artist_id", "cover_art", "song_count", "duration",
        "year", "genre", "genres", "created", "starred", "play_count", "played", "is_compilation",
        "music_brainz_id", "raw_json",
    ],
};

const SONGS: Table = Table {
    name: "songs",
    columns: &[
        "id", "title", "sort_name", "album", "album_id", "artist", "artist_id", "album_artist",
        "composer", "track", "disc_number", "year", "genre", "genres", "duration", "bit_rate",
        "sampling_rate", "bit_depth", "channel_count", "suffix", "content_type", "size", "path",
        "created", "starred", "play_count", "played", "cover_art", "bpm", "music_brainz_id",
        "raw_json",
    ],
};

const PLAYLISTS: Table = Table {
    name: "playlists",
    columns: &[
        "id", "name", "comment", "owner", "public", "song_count", "duration", "created",
        "changed", "cover_art", "raw_json",
    ],
};

/// Tables replaced by a full sync.
const FULL_SYNC_TABLES: [&Table; 3] = [&ARTISTS, &ALBUMS, &SONGS];

/// Counts and dates shown in the UI.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub artists: i64,
    pub albums: i64,
    pub songs: i64,
    /// Unix seconds.
    pub last_full_sync_at: Option<i64>,
    /// Unix seconds.
    pub last_light_sync_at: Option<i64>,
}

/// What the sync rules need to decide whether a full sync is due.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncState {
    pub last_full_sync_at: Option<i64>,
    pub last_server_scan: Option<String>,
    pub last_light_sync_at: Option<i64>,
}

// ---------------------------------------------------------------------------
// Full sync
// ---------------------------------------------------------------------------

/// Creates empty staging tables, discarding any left by a failed sync.
pub fn begin_full_sync(conn: &Connection) -> DbResult<()> {
    drop_staging(conn)?;
    for table in FULL_SYNC_TABLES {
        conn.execute_batch(&format!(
            "CREATE TEMP TABLE staging_{name} AS SELECT * FROM main.{name} WHERE 0",
            name = table.name
        ))?;
    }
    Ok(())
}

/// Writes one downloaded page to the staging tables.
pub fn stage(conn: &mut Connection, items: &LibraryItems) -> DbResult<()> {
    let tx = conn.transaction()?;
    write_rows(&tx, &staging(&ARTISTS), &items.artists, false)?;
    write_rows(&tx, &staging(&ALBUMS), &items.albums, false)?;
    write_rows(&tx, &staging(&SONGS), &items.songs, false)?;
    tx.commit()?;
    Ok(())
}

/// Number of songs downloaded so far in the current full sync.
pub fn staged_song_count(conn: &Connection) -> DbResult<i64> {
    let sql = format!("SELECT COUNT(*) FROM {}", staging(&SONGS));
    Ok(conn.query_row(&sql, [], |row| row.get(0))?)
}

/// Replaces the cache with the staged data in a single transaction.
pub fn finish_full_sync(conn: &mut Connection, server_scan: Option<&str>, now: i64) -> DbResult<()> {
    let tx = conn.transaction()?;
    for table in FULL_SYNC_TABLES {
        let columns = table.columns.join(", ");
        // `WHERE true` avoids a parsing ambiguity between SELECT and ON CONFLICT.
        tx.execute(
            &format!(
                "INSERT INTO main.{name} ({columns}) SELECT {columns} FROM {staging} WHERE true{upsert}",
                name = table.name,
                staging = staging(table),
                upsert = upsert_clause(table),
            ),
            [],
        )?;
        tx.execute(
            &format!(
                "DELETE FROM main.{name} WHERE id NOT IN (SELECT id FROM {staging})",
                name = table.name,
                staging = staging(table),
            ),
            [],
        )?;
    }
    tx.execute(
        "UPDATE sync_state SET last_full_sync_at = ?1, last_server_scan = ?2 WHERE id = 1",
        params![now, server_scan],
    )?;
    tx.commit()?;
    drop_staging(conn)
}

/// Discards the staging tables. The real tables are left untouched.
pub fn drop_staging(conn: &Connection) -> DbResult<()> {
    for table in FULL_SYNC_TABLES {
        conn.execute_batch(&format!("DROP TABLE IF EXISTS {}", staging(table)))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Light sync
// ---------------------------------------------------------------------------

/// Applies `getStarred2` and `getPlaylists` in one transaction:
/// starred items are upserted, items no longer starred lose their date, and the
/// playlist list is replaced (`getPlaylists` always returns the full list).
pub fn apply_light_sync(
    conn: &mut Connection,
    starred: &LibraryItems,
    playlists: &[Playlist],
    now: i64,
) -> DbResult<()> {
    let tx = conn.transaction()?;

    write_rows(&tx, ARTISTS.name, &starred.artists, true)?;
    write_rows(&tx, ALBUMS.name, &starred.albums, true)?;
    write_rows(&tx, SONGS.name, &starred.songs, true)?;
    clear_unstarred(&tx, &ARTISTS, starred.artists.iter().map(|a| a.id.as_str()))?;
    clear_unstarred(&tx, &ALBUMS, starred.albums.iter().map(|a| a.id.as_str()))?;
    clear_unstarred(&tx, &SONGS, starred.songs.iter().map(|s| s.id.as_str()))?;

    write_rows(&tx, PLAYLISTS.name, playlists, true)?;
    tx.execute(
        "DELETE FROM playlists WHERE id NOT IN (SELECT value FROM json_each(?1))",
        [json_array(playlists.iter().map(|p| p.id.as_str()))],
    )?;

    tx.execute("UPDATE sync_state SET last_light_sync_at = ?1 WHERE id = 1", [now])?;
    tx.commit()?;
    Ok(())
}

fn clear_unstarred<'a>(
    conn: &Connection,
    table: &Table,
    starred_ids: impl Iterator<Item = &'a str>,
) -> DbResult<()> {
    // Also drop `starred` from the raw JSON so it stays consistent with the column.
    conn.execute(
        &format!(
            "UPDATE {} SET starred = NULL, raw_json = json_remove(raw_json, '$.starred')
             WHERE starred IS NOT NULL AND id NOT IN (SELECT value FROM json_each(?1))",
            table.name
        ),
        [json_array(starred_ids)],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Reads
// ---------------------------------------------------------------------------

/// Song fields shown in lists and by the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongSummary {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    /// Seconds.
    pub duration: Option<i64>,
}

const SONG_SUMMARY_COLUMNS: &str = "id, title, artist, album, duration";

fn song_summary_from_row(row: &rusqlite::Row) -> rusqlite::Result<SongSummary> {
    Ok(SongSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        artist: row.get(2)?,
        album: row.get(3)?,
        duration: row.get(4)?,
    })
}

/// Most recently added songs first (default order of the Songs screen, spec 6.3).
pub fn recent_songs(conn: &Connection, limit: u32) -> DbResult<Vec<SongSummary>> {
    let sql = format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs ORDER BY created DESC, title LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map([limit], song_summary_from_row)?.collect::<Result<_, _>>()?;
    Ok(songs)
}

/// Several songs at once (e.g. a whole queue), keyed by ID. IDs that are not
/// in the cache are simply absent.
pub fn song_summaries(conn: &Connection, ids: &[String]) -> DbResult<HashMap<String, SongSummary>> {
    let sql = format!(
        "SELECT {SONG_SUMMARY_COLUMNS} FROM songs WHERE id IN (SELECT value FROM json_each(?1))"
    );
    let ids = serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string());
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt
        .query_map([ids], song_summary_from_row)?
        .map(|row| row.map(|song| (song.id.clone(), song)))
        .collect::<Result<_, _>>()?;
    Ok(songs)
}

pub fn stats(conn: &Connection) -> DbResult<LibraryStats> {
    let count = |table: &str| -> DbResult<i64> {
        Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))?)
    };
    let state = sync_state(conn)?;
    Ok(LibraryStats {
        artists: count(ARTISTS.name)?,
        albums: count(ALBUMS.name)?,
        songs: count(SONGS.name)?,
        last_full_sync_at: state.last_full_sync_at,
        last_light_sync_at: state.last_light_sync_at,
    })
}

pub fn sync_state(conn: &Connection) -> DbResult<SyncState> {
    Ok(conn.query_row(
        "SELECT last_full_sync_at, last_server_scan, last_light_sync_at FROM sync_state WHERE id = 1",
        [],
        |row| {
            Ok(SyncState {
                last_full_sync_at: row.get(0)?,
                last_server_scan: row.get(1)?,
                last_light_sync_at: row.get(2)?,
            })
        },
    )?)
}

// ---------------------------------------------------------------------------
// Row writers
// ---------------------------------------------------------------------------

/// Binds an item's fields in the same order as its table's `columns`.
trait Row {
    const TABLE: &'static Table;
    fn bind(&self, stmt: &mut Statement) -> rusqlite::Result<usize>;
}

impl Row for Artist {
    const TABLE: &'static Table = &ARTISTS;
    fn bind(&self, stmt: &mut Statement) -> rusqlite::Result<usize> {
        stmt.execute(params![
            self.id, self.name, self.sort_name, self.album_count, self.cover_art,
            self.artist_image_url, self.starred, self.music_brainz_id,
            json_array(self.roles.iter().map(String::as_str)), self.raw_json,
        ])
    }
}

impl Row for Album {
    const TABLE: &'static Table = &ALBUMS;
    fn bind(&self, stmt: &mut Statement) -> rusqlite::Result<usize> {
        stmt.execute(params![
            self.id, self.name, self.sort_name, self.artist, self.artist_id, self.cover_art,
            self.song_count, self.duration, self.year, self.genre, genre_names(&self.genres),
            self.created, self.starred, self.play_count, self.played, self.is_compilation,
            self.music_brainz_id, self.raw_json,
        ])
    }
}

impl Row for Song {
    const TABLE: &'static Table = &SONGS;
    fn bind(&self, stmt: &mut Statement) -> rusqlite::Result<usize> {
        stmt.execute(params![
            self.id, self.title, self.sort_name, self.album, self.album_id, self.artist,
            self.artist_id, self.display_album_artist, self.display_composer, self.track,
            self.disc_number, self.year, self.genre, genre_names(&self.genres), self.duration,
            self.bit_rate, self.sampling_rate, self.bit_depth, self.channel_count, self.suffix,
            self.content_type, self.size, self.path, self.created, self.starred, self.play_count,
            self.played, self.cover_art, self.bpm, self.music_brainz_id, self.raw_json,
        ])
    }
}

impl Row for Playlist {
    const TABLE: &'static Table = &PLAYLISTS;
    fn bind(&self, stmt: &mut Statement) -> rusqlite::Result<usize> {
        stmt.execute(params![
            self.id, self.name, self.comment, self.owner, self.public, self.song_count,
            self.duration, self.created, self.changed, self.cover_art, self.raw_json,
        ])
    }
}

/// Inserts rows into `target` (the table itself or its staging copy).
/// With `upsert`, existing IDs are updated instead of failing.
fn write_rows<T: Row>(conn: &Connection, target: &str, rows: &[T], upsert: bool) -> DbResult<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let columns = T::TABLE.columns;
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
    let mut sql = format!(
        "INSERT INTO {target} ({}) VALUES ({})",
        columns.join(", "),
        placeholders.join(", ")
    );
    if upsert {
        sql.push_str(&upsert_clause(T::TABLE));
    }

    let mut stmt = conn.prepare_cached(&sql)?;
    for row in rows {
        row.bind(&mut stmt)?;
    }
    Ok(())
}

fn upsert_clause(table: &Table) -> String {
    let updates: Vec<String> = table
        .columns
        .iter()
        .filter(|column| **column != "id")
        .map(|column| format!("{column} = excluded.{column}"))
        .collect();
    format!(" ON CONFLICT(id) DO UPDATE SET {}", updates.join(", "))
}

fn staging(table: &Table) -> String {
    format!("temp.staging_{}", table.name)
}

fn json_array<'a>(values: impl Iterator<Item = &'a str>) -> String {
    serde_json::to_string(&values.collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string())
}

fn genre_names(genres: &[Genre]) -> String {
    json_array(genres.iter().map(|g| g.name.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn artist(id: &str) -> Artist {
        Artist {
            id: id.into(),
            name: format!("Artist {id}"),
            roles: vec!["albumartist".into()],
            raw_json: format!(r#"{{"id":"{id}"}}"#),
            ..Default::default()
        }
    }

    fn album(id: &str, artist_id: &str) -> Album {
        Album {
            id: id.into(),
            name: format!("Album {id}"),
            artist_id: Some(artist_id.into()),
            genres: vec![Genre { name: "Rock".into() }],
            raw_json: format!(r#"{{"id":"{id}"}}"#),
            ..Default::default()
        }
    }

    fn song(id: &str, album_id: &str) -> Song {
        Song {
            id: id.into(),
            title: format!("Song {id}"),
            album_id: Some(album_id.into()),
            suffix: Some("flac".into()),
            play_count: Some(1),
            raw_json: format!(r#"{{"id":"{id}"}}"#),
            ..Default::default()
        }
    }

    fn library(songs: Vec<Song>) -> LibraryItems {
        LibraryItems {
            artists: vec![artist("ar1")],
            albums: vec![album("al1", "ar1")],
            songs,
        }
    }

    /// Runs a complete, successful full sync with the given pages.
    fn full_sync(conn: &mut Connection, pages: &[LibraryItems], now: i64) {
        begin_full_sync(conn).unwrap();
        for page in pages {
            stage(conn, page).unwrap();
        }
        finish_full_sync(conn, Some("scan-1"), now).unwrap();
    }

    fn song_ids(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT id FROM songs ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn play_count(conn: &Connection, song_id: &str) -> Option<i64> {
        conn.query_row("SELECT play_count FROM songs WHERE id = ?1", [song_id], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn full_sync_inserts_items() {
        let mut conn = open_in_memory().unwrap();
        full_sync(&mut conn, &[library(vec![song("s1", "al1"), song("s2", "al1")])], 1000);

        let stats = stats(&conn).unwrap();
        assert_eq!((stats.artists, stats.albums, stats.songs), (1, 1, 2));
        assert_eq!(stats.last_full_sync_at, Some(1000));

        let (genres, roles): (String, String) = conn
            .query_row(
                "SELECT albums.genres, artists.roles FROM albums, artists",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(genres, r#"["Rock"]"#);
        assert_eq!(roles, r#"["albumartist"]"#);
        assert_eq!(sync_state(&conn).unwrap().last_server_scan.as_deref(), Some("scan-1"));
    }

    #[test]
    fn full_sync_across_several_pages() {
        let mut conn = open_in_memory().unwrap();
        let page1 = library(vec![song("s1", "al1")]);
        let page2 = LibraryItems { songs: vec![song("s2", "al1")], ..Default::default() };
        full_sync(&mut conn, &[page1, page2], 1000);
        assert_eq!(song_ids(&conn), ["s1", "s2"]);
    }

    #[test]
    fn second_full_sync_updates_changed_fields() {
        let mut conn = open_in_memory().unwrap();
        full_sync(&mut conn, &[library(vec![song("s1", "al1")])], 1000);

        let mut changed = song("s1", "al1");
        changed.play_count = Some(42);
        changed.title = "Renamed".into();
        full_sync(&mut conn, &[library(vec![changed])], 2000);

        assert_eq!(play_count(&conn, "s1"), Some(42));
        let title: String =
            conn.query_row("SELECT title FROM songs WHERE id = 's1'", [], |r| r.get(0)).unwrap();
        assert_eq!(title, "Renamed");
        assert_eq!(stats(&conn).unwrap().songs, 1);
    }

    #[test]
    fn missing_items_are_removed_only_when_the_sync_finishes() {
        let mut conn = open_in_memory().unwrap();
        full_sync(&mut conn, &[library(vec![song("s1", "al1"), song("s2", "al1")])], 1000);

        begin_full_sync(&conn).unwrap();
        stage(&mut conn, &library(vec![song("s1", "al1")])).unwrap();
        // Still downloading: nothing removed yet.
        assert_eq!(song_ids(&conn), ["s1", "s2"]);

        finish_full_sync(&mut conn, Some("scan-2"), 2000).unwrap();
        assert_eq!(song_ids(&conn), ["s1"]);
    }

    #[test]
    fn failed_full_sync_leaves_cache_intact() {
        let mut conn = open_in_memory().unwrap();
        full_sync(&mut conn, &[library(vec![song("s1", "al1"), song("s2", "al1")])], 1000);

        // A new sync downloads a changed, partial page, then fails (no finish).
        begin_full_sync(&conn).unwrap();
        let mut changed = song("s1", "al1");
        changed.play_count = Some(99);
        stage(&mut conn, &library(vec![changed])).unwrap();
        drop_staging(&conn).unwrap();

        assert_eq!(song_ids(&conn), ["s1", "s2"]);
        assert_eq!(play_count(&conn, "s1"), Some(1));
        let state = sync_state(&conn).unwrap();
        assert_eq!(state.last_full_sync_at, Some(1000));
        assert_eq!(state.last_server_scan.as_deref(), Some("scan-1"));
    }

    #[test]
    fn duplicated_items_across_pages_are_tolerated() {
        let mut conn = open_in_memory().unwrap();
        let page1 = library(vec![song("s1", "al1")]);
        let mut again = song("s1", "al1");
        again.play_count = Some(5);
        let page2 = LibraryItems { songs: vec![again], ..Default::default() };

        full_sync(&mut conn, &[page1, page2], 1000);
        assert_eq!(song_ids(&conn), ["s1"]);
    }

    #[test]
    fn play_history_survives_removal_of_its_song() {
        let mut conn = open_in_memory().unwrap();
        full_sync(&mut conn, &[library(vec![song("s1", "al1")])], 1000);
        conn.execute("INSERT INTO play_history (song_id, played_at) VALUES ('s1', 500)", [])
            .unwrap();

        full_sync(&mut conn, &[library(vec![])], 2000);
        assert!(song_ids(&conn).is_empty());
        let plays: i64 =
            conn.query_row("SELECT COUNT(*) FROM play_history", [], |r| r.get(0)).unwrap();
        assert_eq!(plays, 1);
    }

    #[test]
    fn light_sync_updates_starred_and_replaces_playlists() {
        let mut conn = open_in_memory().unwrap();
        let mut starred_song = song("s1", "al1");
        starred_song.starred = Some("2026-01-01T00:00:00Z".into());
        starred_song.raw_json = r#"{"id":"s1","starred":"2026-01-01T00:00:00Z"}"#.into();
        full_sync(&mut conn, &[library(vec![starred_song, song("s2", "al1")])], 1000);

        let playlist = |id: &str| Playlist {
            id: id.into(),
            name: format!("Playlist {id}"),
            raw_json: "{}".into(),
            ..Default::default()
        };
        apply_light_sync(&mut conn, &LibraryItems::default(), &[playlist("p1"), playlist("p2")], 1500)
            .unwrap();

        // s1 was unstarred on another app: column and raw JSON are cleared.
        let (starred, raw): (Option<String>, String) = conn
            .query_row("SELECT starred, raw_json FROM songs WHERE id = 's1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(starred, None);
        assert!(!raw.contains("starred"));

        // Now s2 is starred and p1 was deleted on the server.
        let mut newly_starred = song("s2", "al1");
        newly_starred.starred = Some("2026-02-02T00:00:00Z".into());
        let starred = LibraryItems { songs: vec![newly_starred], ..Default::default() };
        apply_light_sync(&mut conn, &starred, &[playlist("p2")], 1600).unwrap();

        let starred_ids: Vec<String> = conn
            .prepare("SELECT id FROM songs WHERE starred IS NOT NULL")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(starred_ids, ["s2"]);
        let playlists: Vec<String> = conn
            .prepare("SELECT id FROM playlists")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(playlists, ["p2"]);
        assert_eq!(stats(&conn).unwrap().last_light_sync_at, Some(1600));
    }

    #[test]
    fn recent_songs_come_newest_first_and_respect_the_limit() {
        let mut conn = open_in_memory().unwrap();
        let dated = |id: &str, created: &str| {
            let mut s = song(id, "al1");
            s.created = Some(created.into());
            s.duration = Some(200);
            s
        };
        full_sync(
            &mut conn,
            &[library(vec![
                dated("old", "2024-01-01T00:00:00Z"),
                dated("new", "2026-01-01T00:00:00Z"),
                dated("mid", "2025-01-01T00:00:00Z"),
            ])],
            1000,
        );

        let ids: Vec<String> = recent_songs(&conn, 2).unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["new", "mid"]);
    }

    #[test]
    fn song_summaries_returns_the_songs_found() {
        let mut conn = open_in_memory().unwrap();
        full_sync(&mut conn, &[library(vec![song("s1", "al1"), song("s2", "al1")])], 1000);

        let found = song_summaries(&conn, &["s2".into(), "missing".into(), "s1".into()]).unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found["s2"].title, "Song s2");
    }

    #[test]
    fn new_database_has_empty_stats() {
        let conn = open_in_memory().unwrap();
        assert_eq!(stats(&conn).unwrap(), LibraryStats::default());
    }
}
