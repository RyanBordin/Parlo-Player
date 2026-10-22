//! Local SQLite database: library cache, play history, saved queue and sync state.
//! One database file per server + user (spec 12.4).

pub mod albums;
pub mod artist_page;
pub mod artists;
pub mod favorites;
pub mod history;
pub mod home;
pub mod items;
pub mod library;
pub mod related;
mod migrations;
pub mod saved_queue;
pub mod search;
pub mod settings;
pub mod songs;
pub mod tracks;

use std::fs;
use std::path::Path;
use std::time::Duration;

use md5::{Digest, Md5};
use rusqlite::Connection;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("database file error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database version {found} is newer than this app supports ({supported})")]
    TooNew { found: i64, supported: i64 },
}

pub type DbResult<T> = Result<T, DbError>;

/// Opens (or creates) a database file and brings its schema up to date.
pub fn open(path: &Path) -> DbResult<Connection> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    // WAL lets the UI read while a sync is writing.
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(Duration::from_secs(5))?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}

/// In-memory database with the full schema, for tests.
#[cfg(test)]
pub fn open_in_memory() -> DbResult<Connection> {
    let mut conn = Connection::open_in_memory()?;
    migrations::migrate(&mut conn)?;
    Ok(conn)
}

/// File name for a server + user, e.g. `ryan@192.168.3.24_4533-1a2b3c4d.db`.
/// Readable, safe on every OS, and unique thanks to the hash suffix.
pub fn library_file_name(server_url: &str, username: &str) -> String {
    let host = server_url.split_once("://").map_or(server_url, |(_, rest)| rest);
    let readable: String = format!("{username}@{host}")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || "@.-_".contains(c) { c } else { '_' })
        .take(64)
        .collect();

    let digest = Md5::digest(format!("{username}@{server_url}").as_bytes());
    let hash: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
    format!("{readable}-{hash}.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_is_readable_and_safe() {
        let name = library_file_name("http://192.168.3.24:4533", "ryan");
        assert!(name.starts_with("ryan@192.168.3.24_4533-"), "{name}");
        assert!(name.ends_with(".db"));
        assert!(!name.contains([':', '/', '\\', '?', '*']));
    }

    #[test]
    fn file_name_differs_per_user_and_server() {
        let a = library_file_name("https://music.example.com", "alice");
        let b = library_file_name("https://music.example.com", "bob");
        let c = library_file_name("http://music.example.com", "alice");
        assert_ne!(a, b);
        assert_ne!(a, c, "same host, different scheme is a different server URL");
    }

    #[test]
    fn open_creates_file_with_schema_and_wal() {
        let dir = std::env::temp_dir().join(format!("parlo-db-test-{}", std::process::id()));
        let path = dir.join("libraries").join("test.db");
        let _ = fs::remove_dir_all(&dir);

        let conn = open(&path).unwrap();
        let mode: String = conn.pragma_query_value(None, "journal_mode", |r| r.get(0)).unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
        assert!(path.exists());

        drop(conn);
        let _ = fs::remove_dir_all(&dir);
    }
}
