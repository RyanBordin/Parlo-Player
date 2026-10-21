//! Versioned schema migrations.
//!
//! The schema version is stored in SQLite's own `PRAGMA user_version`.
//! Migration N (1-based) brings the schema from version N-1 to N. To change
//! the schema, add a new file at the end of `MIGRATIONS`; never edit one that
//! was already released, or existing databases would miss the change.

use rusqlite::Connection;

use super::{DbError, DbResult};

/// A schema change, and optionally a step in Rust that fills the new columns
/// (e.g. values SQLite cannot compute), run in the same transaction.
struct Migration {
    sql: &'static str,
    then: Option<fn(&Connection) -> DbResult<()>>,
}

const MIGRATIONS: &[Migration] = &[
    Migration { sql: include_str!("migrations/001_initial.sql"), then: None },
    Migration { sql: include_str!("migrations/002_saved_queue.sql"), then: None },
    Migration { sql: include_str!("migrations/003_playlist_songs.sql"), then: None },
    Migration {
        sql: include_str!("migrations/004_song_search.sql"),
        then: Some(super::songs::fill_search_columns),
    },
    Migration {
        sql: include_str!("migrations/005_song_artists.sql"),
        then: Some(super::artists::fill_song_artists),
    },
    Migration { sql: include_str!("migrations/006_artist_info.sql"), then: None },
    Migration { sql: include_str!("migrations/007_library_settings.sql"), then: None },
    Migration { sql: include_str!("migrations/008_sync_result.sql"), then: None },
    Migration {
        sql: include_str!("migrations/009_search.sql"),
        then: Some(super::search::fill_search_columns),
    },
    Migration { sql: include_str!("migrations/010_search_index.sql"), then: None },
];

/// Applies every migration newer than the database, each in its own transaction.
pub fn migrate(conn: &mut Connection) -> DbResult<()> {
    migrate_to(conn, MIGRATIONS.len())
}

/// Applies the migrations newer than the database, up to version `target`.
fn migrate_to(conn: &mut Connection, target: usize) -> DbResult<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = MIGRATIONS.len() as i64;
    if current > supported {
        return Err(DbError::TooNew { found: current, supported });
    }

    for (index, migration) in MIGRATIONS.iter().enumerate().take(target).skip(current.max(0) as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        if let Some(then) = migration.then {
            then(&tx)?;
        }
        tx.pragma_update(None, "user_version", index as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap()
    }

    #[test]
    fn applies_all_migrations_on_a_new_database() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn), MIGRATIONS.len() as i64);

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for table in ["albums", "artist_info", "artists", "play_history", "playlist_songs", "playlists", "recent_searches", "saved_queue", "settings", "song_artists", "songs", "songs_fts", "sync_state"] {
            assert!(tables.contains(&table.to_string()), "missing table {table}");
        }
    }

    #[test]
    fn running_again_changes_nothing() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        conn.execute("INSERT INTO play_history (song_id, played_at) VALUES ('s1', 1)", [])
            .unwrap();

        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn), MIGRATIONS.len() as i64);
        let plays: i64 = conn.query_row("SELECT COUNT(*) FROM play_history", [], |r| r.get(0)).unwrap();
        assert_eq!(plays, 1);
    }

    fn check_search_indexes(conn: &Connection) {
        for index in ["songs_fts", "albums_fts", "artists_fts"] {
            let check = conn.execute_batch(&format!("INSERT INTO {index} ({index}, rank) VALUES ('integrity-check', 1)"));
            assert!(check.is_ok(), "{index}: {check:?}");
        }
    }

    #[test]
    fn the_search_indexes_take_the_library_already_cached() {
        use crate::db::search::search;

        // A library synced before the search existed (version 8): its songs
        // already have the `search_text` of migration 4.
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 8).unwrap();
        conn.execute_batch(
            "INSERT INTO artists (id, name, roles, raw_json) VALUES ('ar', 'João Gilberto', '[\"albumartist\"]', '{}');
             INSERT INTO albums (id, name, artist, artist_id, year, genres, raw_json)
                  VALUES ('al', 'Chega de Saudade', 'João Gilberto', 'ar', 1959, '[\"Bossa Nova\"]', '{}');
             INSERT INTO songs (id, title, artist, album_id, search_text, raw_json)
                  VALUES ('s', 'Desafinado', 'João Gilberto', 'al', 'desafinado\njoao gilberto', '{}');",
        )
        .unwrap();

        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn), MIGRATIONS.len() as i64);
        check_search_indexes(&conn);
        let results = search(&conn, "joao").unwrap();
        assert_eq!(results.song_ids, ["s"]);
        assert_eq!(results.albums[0].year, Some(1959));
        assert_eq!(results.artists.len(), 1);
        assert_eq!(search(&conn, "bossa 1959").unwrap().albums.len(), 1, "genre and year of the album");

        // From now on the triggers keep the indexes in step.
        conn.execute("UPDATE songs SET search_name = 'outra', search_text = 'outra' WHERE id = 's'", [])
            .unwrap();
        conn.execute("DELETE FROM artists WHERE id = 'ar'", []).unwrap();
        check_search_indexes(&conn);
        assert_eq!(search(&conn, "outra").unwrap().song_ids, ["s"]);
        assert!(search(&conn, "desafinado").unwrap().song_ids.is_empty());
        assert!(search(&conn, "gilberto").unwrap().artists.is_empty());
    }

    #[test]
    fn refuses_database_from_newer_app() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", MIGRATIONS.len() as i64 + 1).unwrap();
        assert!(matches!(migrate(&mut conn), Err(DbError::TooNew { .. })));
    }
}
