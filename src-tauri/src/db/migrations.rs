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
];

/// Applies every migration newer than the database, each in its own transaction.
pub fn migrate(conn: &mut Connection) -> DbResult<()> {
    let current: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let supported = MIGRATIONS.len() as i64;
    if current > supported {
        return Err(DbError::TooNew { found: current, supported });
    }

    for (index, migration) in MIGRATIONS.iter().enumerate().skip(current.max(0) as usize) {
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
        for table in ["albums", "artists", "play_history", "playlist_songs", "playlists", "saved_queue", "song_artists", "songs", "sync_state"] {
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

    #[test]
    fn refuses_database_from_newer_app() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", MIGRATIONS.len() as i64 + 1).unwrap();
        assert!(matches!(migrate(&mut conn), Err(DbError::TooNew { .. })));
    }
}
