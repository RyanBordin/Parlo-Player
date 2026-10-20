//! Automatic sync (spec 6.10, 12.4): every minute a thread checks whether
//! the interval chosen for the signed-in server has passed since its last
//! sync, and then runs the sync rules (light sync, and the full one when the
//! server's last scan changed). It runs in the backend, so it keeps working
//! while the window is hidden. The sync at app start is the interface's.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::commands::{self, SyncKind};
use crate::db::{self, DbResult, library};
use crate::state::AppState;
use crate::sync;

const CHECK_EVERY: Duration = Duration::from_secs(60);

pub fn start(app: AppHandle) {
    let spawned = thread::Builder::new().name("auto-sync".into()).spawn(move || {
        loop {
            thread::sleep(CHECK_EVERY);
            let state = Arc::clone(&app.state::<Arc<AppState>>());
            let Some(active) = state.active() else { continue };
            if state.is_syncing() {
                continue;
            }
            let due = db::open(&active.db_path).and_then(|conn| {
                let interval = db::settings::load(&conn)?.sync_interval_minutes;
                Ok(is_due(interval, last_sync(&conn)?, sync::now()))
            });
            match due {
                Ok(true) => {
                    let _ = commands::start_sync(app.clone(), &state, SyncKind::Rules { force_full: false });
                }
                Ok(false) => {}
                Err(error) => log::warn!("auto-sync: could not read the library: {error}"),
            }
        }
    });
    if let Err(error) = spawned {
        log::warn!("auto-sync: could not start: {error}");
    }
}

/// When the last sync of any kind ended (failed ones too, so a server that
/// is down is not asked every minute), Unix seconds.
fn last_sync(conn: &Connection) -> DbResult<Option<i64>> {
    let state = library::sync_state(conn)?;
    let result = library::sync_result(conn)?;
    Ok([result.at, state.last_light_sync_at, state.last_full_sync_at].into_iter().flatten().max())
}

/// Whether a sync is due: `interval_minutes` 0 is "manual only".
fn is_due(interval_minutes: u32, last: Option<i64>, now: i64) -> bool {
    if interval_minutes == 0 {
        return false;
    }
    last.is_none_or(|last| now - last >= i64::from(interval_minutes) * 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn a_sync_is_due_after_the_interval() {
        assert!(!is_due(30, Some(1000), 1000 + 29 * 60));
        assert!(is_due(30, Some(1000), 1000 + 30 * 60));
        assert!(is_due(15, None, 0), "never synced");
        assert!(!is_due(0, None, 1_000_000), "manual only");
    }

    #[test]
    fn the_last_sync_is_the_latest_of_any_kind() {
        let conn = open_in_memory().unwrap();
        assert_eq!(last_sync(&conn).unwrap(), None);
        conn.execute("UPDATE sync_state SET last_full_sync_at = 100, last_light_sync_at = 300", []).unwrap();
        library::record_sync_result(&conn, 200, Some("{}")).unwrap();
        assert_eq!(last_sync(&conn).unwrap(), Some(300));
    }
}
