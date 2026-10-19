//! Library sync (spec 12.4).
//!
//! - Full: artists, albums and songs. Runs on the first sync, when the server's
//!   last scan changed, or when the last full sync is older than 24 hours.
//! - Light: starred items and playlists. Runs every time.

mod full;
mod light;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;
use serde::Serialize;
use thiserror::Error;

use crate::db::library::{self, SyncState};
use crate::db::{self, DbError, DbResult};
use crate::subsonic::{SubsonicClient, SubsonicError};

/// A full sync also picks up play counts changed by other apps (spec 12.4).
const FULL_SYNC_MAX_AGE_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Artists,
    Albums,
    Songs,
    Saving,
    Starred,
    Playlists,
}

/// Payload of the `sync-progress` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub phase: Phase,
    /// Items processed in this phase.
    pub done: u64,
    /// Total for this phase, when the server tells it (songs only).
    pub total: Option<u64>,
    /// Whether this run includes a full sync.
    pub full: bool,
}

#[derive(Debug, Error)]
pub enum SyncError {
    #[error(transparent)]
    Server(#[from] SubsonicError),

    #[error(transparent)]
    Db(#[from] DbError),

    /// Protects the cache: an empty answer must not wipe the library.
    #[error("the server returned no songs although it reports {0}")]
    EmptyLibrary(i64),

    #[error("the server repeated the same page; paging is not supported")]
    PagingNotSupported,

    #[error("background task failed: {0}")]
    Task(String),
}

/// Runs the sync rules: a full sync if due (or forced), then a light sync.
pub async fn run(
    client: &SubsonicClient,
    db_path: PathBuf,
    force_full: bool,
    progress: &(dyn Fn(SyncProgress) + Send + Sync),
) -> Result<(), SyncError> {
    let db = SyncDb::open(db_path.clone()).await?;
    let scan = client.get_scan_status().await?;
    let state = db.call(|conn| library::sync_state(conn)).await?;

    let full = force_full || needs_full_sync(&state, scan.last_scan.as_deref(), now());
    if full {
        full::run(client, &db, &scan, progress).await?;
    }
    light::run(client, &db, &db_path, full, progress).await
}

/// Whether a full sync is due (spec 12.4).
fn needs_full_sync(state: &SyncState, server_last_scan: Option<&str>, now: i64) -> bool {
    let Some(last_full) = state.last_full_sync_at else {
        return true; // never synced
    };
    let scan_changed =
        server_last_scan.is_some() && server_last_scan != state.last_server_scan.as_deref();
    scan_changed || now - last_full >= FULL_SYNC_MAX_AGE_SECS
}

pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The sync's own database connection, used from blocking threads so the
/// async runtime is never blocked. One connection keeps the TEMP staging
/// tables alive for the whole full sync.
struct SyncDb(Arc<Mutex<Connection>>);

impl SyncDb {
    async fn open(path: PathBuf) -> Result<Self, SyncError> {
        let conn = blocking(move || db::open(&path)).await?;
        Ok(Self(Arc::new(Mutex::new(conn))))
    }

    async fn call<T, F>(&self, task: F) -> Result<T, SyncError>
    where
        F: FnOnce(&mut Connection) -> DbResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let conn = Arc::clone(&self.0);
        blocking(move || {
            let mut conn = conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            task(&mut conn)
        })
        .await
    }
}

async fn blocking<T, F>(task: F) -> Result<T, SyncError>
where
    F: FnOnce() -> DbResult<T> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| SyncError::Task(e.to_string()))?
        .map_err(SyncError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 60 * 60;

    fn synced(at: i64, scan: &str) -> SyncState {
        SyncState {
            last_full_sync_at: Some(at),
            last_server_scan: Some(scan.into()),
            last_light_sync_at: Some(at),
        }
    }

    #[test]
    fn first_sync_is_full() {
        assert!(needs_full_sync(&SyncState::default(), Some("scan-1"), 1000));
        assert!(needs_full_sync(&SyncState::default(), None, 1000));
    }

    #[test]
    fn new_server_scan_triggers_full_sync() {
        let state = synced(1000, "scan-1");
        assert!(needs_full_sync(&state, Some("scan-2"), 1000 + HOUR));
    }

    #[test]
    fn same_scan_within_a_day_skips_full_sync() {
        let state = synced(1000, "scan-1");
        assert!(!needs_full_sync(&state, Some("scan-1"), 1000 + 23 * HOUR));
    }

    #[test]
    fn full_sync_runs_at_least_once_a_day() {
        let state = synced(1000, "scan-1");
        assert!(needs_full_sync(&state, Some("scan-1"), 1000 + 24 * HOUR));
    }

    #[test]
    fn server_without_last_scan_relies_on_daily_rule() {
        let state = synced(1000, "scan-1");
        assert!(!needs_full_sync(&state, None, 1000 + HOUR));
        assert!(needs_full_sync(&state, None, 1000 + 25 * HOUR));
    }

    #[test]
    fn progress_serializes_for_the_ui() {
        let progress = SyncProgress { phase: Phase::Songs, done: 500, total: Some(5120), full: true };
        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            serde_json::json!({ "phase": "songs", "done": 500, "total": 5120, "full": true })
        );
    }
}
