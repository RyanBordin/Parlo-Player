//! Backend state shared by the Tauri commands.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::playback::Playback;
use crate::settings::AppSettings;
use crate::subsonic::SubsonicClient;

/// The playback controller, or why it could not start (e.g. libmpv failed to
/// initialize). The app still opens without it; playback commands then
/// return the error.
pub struct PlaybackSlot(pub Result<Arc<Playback>, String>);

/// The signed-in server and its library database.
#[derive(Debug, Clone)]
pub struct ActiveLibrary {
    pub client: Arc<SubsonicClient>,
    pub db_path: PathBuf,
}

#[derive(Debug, Default)]
pub struct AppState {
    active: Mutex<Option<ActiveLibrary>>,
    syncing: AtomicBool,
    /// The app preferences as saved (read at startup, replaced on each save).
    settings: Mutex<AppSettings>,
}

impl AppState {
    pub fn settings(&self) -> AppSettings {
        self.settings.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    /// Replaces the preferences and returns the previous ones.
    pub fn set_settings(&self, settings: AppSettings) -> AppSettings {
        let mut current = self.settings.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::replace(&mut current, settings)
    }

    pub fn active(&self) -> Option<ActiveLibrary> {
        self.lock().clone()
    }

    pub fn set_active(&self, library: Option<ActiveLibrary>) {
        *self.lock() = library;
    }

    pub fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::SeqCst)
    }

    /// Marks a sync as running. Returns `None` if one is already running.
    /// The returned guard clears the flag when dropped, even on panic.
    pub fn try_start_sync(self: &Arc<Self>) -> Option<SyncGuard> {
        self.syncing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| SyncGuard(Arc::clone(self)))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<ActiveLibrary>> {
        self.active.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct SyncGuard(Arc<AppState>);

impl Drop for SyncGuard {
    fn drop(&mut self) {
        self.0.syncing.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_sync_at_a_time() {
        let state = Arc::new(AppState::default());
        let guard = state.try_start_sync().expect("first sync starts");
        assert!(state.is_syncing());
        assert!(state.try_start_sync().is_none(), "second sync is refused");

        drop(guard);
        assert!(!state.is_syncing());
        assert!(state.try_start_sync().is_some());
    }
}
