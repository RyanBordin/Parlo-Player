//! Tauri commands exposed to the UI.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::library::{self, LibraryStats, SongSummary};
use crate::db::{self, DbError};
use crate::playback::{Playback, PlaybackError, QueueState};
use crate::player::queue::RepeatMode;
use crate::player::{PlayerError, PlayerState};
use crate::session::{self, SavedSession, SessionError};
use crate::state::{ActiveLibrary, AppState, PlaybackSlot};
use crate::subsonic::{ServerInfo, SubsonicClient, SubsonicError};
use crate::sync::{self, SyncError, SyncProgress};

/// Event with a `SyncProgress` payload, sent while a sync runs.
const SYNC_PROGRESS_EVENT: &str = "sync-progress";
/// Event with a `SyncFinished` payload, sent once when a sync ends.
const SYNC_FINISHED_EVENT: &str = "sync-finished";

/// Error sent to the UI. The UI picks the translated message from `kind`,
/// e.g. `{ "kind": "api", "code": 40, "message": "Wrong username or password" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandError {
    InvalidUrl,
    Timeout,
    HttpStatus { status: u16 },
    Network { detail: String },
    InvalidResponse,
    Api { code: i32, message: String },
    Storage { detail: String },
    NotLoggedIn,
    EmptyLibrary { reported: i64 },
    SongNotFound,
    Playback { detail: String },
}

impl From<PlayerError> for CommandError {
    fn from(error: PlayerError) -> Self {
        Self::Playback { detail: error.to_string() }
    }
}

impl From<PlaybackError> for CommandError {
    fn from(error: PlaybackError) -> Self {
        match error {
            PlaybackError::Player(e) => e.into(),
            PlaybackError::Db(e) => e.into(),
            PlaybackError::Server(e) => e.into(),
            PlaybackError::NotLoggedIn => Self::NotLoggedIn,
            PlaybackError::NoSongs => Self::SongNotFound,
            PlaybackError::Task(detail) => Self::Playback { detail },
        }
    }
}

impl From<SubsonicError> for CommandError {
    fn from(error: SubsonicError) -> Self {
        match error {
            SubsonicError::Http(e) if e.is_builder() => Self::InvalidUrl,
            SubsonicError::Http(e) if e.is_timeout() => Self::Timeout,
            SubsonicError::Http(e) if e.is_status() => match e.status() {
                Some(status) => Self::HttpStatus { status: status.as_u16() },
                None => Self::Network { detail: e.to_string() },
            },
            SubsonicError::Http(e) => Self::Network { detail: e.to_string() },
            SubsonicError::InvalidUrl(_) => Self::InvalidUrl,
            SubsonicError::InvalidResponse(_) => Self::InvalidResponse,
            SubsonicError::Api { code, message } => Self::Api { code, message },
        }
    }
}

impl From<SessionError> for CommandError {
    fn from(error: SessionError) -> Self {
        Self::Storage { detail: error.to_string() }
    }
}

impl From<DbError> for CommandError {
    fn from(error: DbError) -> Self {
        Self::Storage { detail: error.to_string() }
    }
}

impl From<SyncError> for CommandError {
    fn from(error: SyncError) -> Self {
        match error {
            SyncError::Server(e) => e.into(),
            SyncError::Db(e) => e.into(),
            SyncError::EmptyLibrary(reported) => Self::EmptyLibrary { reported },
            SyncError::PagingNotSupported => Self::InvalidResponse,
            SyncError::Task(detail) => Self::Storage { detail },
        }
    }
}

/// Library counts plus whether a sync is running right now.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    #[serde(flatten)]
    pub stats: LibraryStats,
    pub syncing: bool,
}

/// Payload of the `sync-finished` event. `error` is absent on success.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncFinished {
    error: Option<CommandError>,
}

/// Pings the server with the given credentials. Nothing is stored.
#[tauri::command]
pub async fn test_connection(
    url: String,
    user: String,
    password: String,
) -> Result<ServerInfo, CommandError> {
    let client = SubsonicClient::new(url, user, password)?;
    Ok(client.ping().await?)
}

/// Returns the saved login, if any, and makes its library active.
/// The password never leaves the backend.
#[tauri::command]
pub async fn get_session(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<SavedSession>, CommandError> {
    let dir = config_dir(&app)?;
    let Some(stored) = run_blocking(move || session::load(&dir)).await? else {
        state.set_active(None);
        return Ok(None);
    };

    let client = SubsonicClient::new(
        stored.session.server_url.clone(),
        stored.session.username.clone(),
        stored.password,
    )?;
    state.set_active(Some(active_library(&app, client, &stored.session)?));
    Ok(Some(stored.session))
}

/// Checks the credentials again, saves them and makes the library active.
#[tauri::command]
pub async fn login(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    url: String,
    user: String,
    password: String,
) -> Result<SavedSession, CommandError> {
    let client = SubsonicClient::new(url, user.clone(), password.clone())?;
    client.ping().await?;

    let saved = SavedSession {
        server_url: client.base_url().to_string(),
        username: user,
    };
    let dir = config_dir(&app)?;
    let to_save = saved.clone();
    run_blocking(move || session::save(&dir, &to_save, &password)).await?;

    state.set_active(Some(active_library(&app, client, &saved)?));
    Ok(saved)
}

/// Deletes the saved password and config file. The library database is kept,
/// so the play history is not lost (spec 12.4).
#[tauri::command]
pub async fn logout(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
) -> Result<(), CommandError> {
    let dir = config_dir(&app)?;
    run_blocking(move || session::clear(&dir)).await?;
    // The music would keep streaming with the signed-out credentials.
    if let Ok(playback) = &playback.0 {
        playback.stop()?;
    }
    state.set_active(None);
    Ok(())
}

/// Counts and sync dates of the active library.
#[tauri::command]
pub async fn get_library_status(
    state: State<'_, Arc<AppState>>,
) -> Result<LibraryStatus, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let stats = run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        library::stats(&conn)
    })
    .await?;
    Ok(LibraryStatus { stats, syncing: state.is_syncing() })
}

/// Starts a sync in the background and returns right away: `false` if one is
/// already running. With `force_full`, the full sync runs regardless of the
/// rules ("Sync now"). Progress comes as `sync-progress` events, the end as
/// one `sync-finished` event.
#[tauri::command]
pub fn sync_library(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    force_full: bool,
) -> Result<bool, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let Some(guard) = state.try_start_sync() else {
        return Ok(false);
    };

    tauri::async_runtime::spawn(async move {
        let emit_progress = |progress: SyncProgress| {
            let _ = app.emit(SYNC_PROGRESS_EVENT, progress);
        };
        let result = sync::run(&active.client, active.db_path, force_full, &emit_progress).await;

        // Clear the flag first, so the UI sees `syncing: false` when it reacts.
        drop(guard);
        let error = result.err().map(CommandError::from);
        let _ = app.emit(SYNC_FINISHED_EVENT, SyncFinished { error });
    });
    Ok(true)
}

/// Songs from the local cache, most recently added first (temporary list).
#[tauri::command]
pub async fn list_songs(
    state: State<'_, Arc<AppState>>,
    limit: u32,
) -> Result<Vec<SongSummary>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        library::recent_songs(&conn, limit)
    })
    .await
}

/// Replaces the queue with a whole context and plays from `start_index`
/// (spec 5.1). `context_label` is the "Playing from" text.
#[tauri::command]
pub async fn play_context(
    playback: State<'_, PlaybackSlot>,
    song_ids: Vec<String>,
    start_index: usize,
    context_label: String,
) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    // Reads the library cache, so it runs outside the async runtime.
    tauri::async_runtime::spawn_blocking(move || {
        playback.play_context(song_ids, start_index, context_label)
    })
    .await
    .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// Resumes the queue saved locally or on the server, paused (spec 5.6). Only
/// the first call per library does something.
#[tauri::command]
pub async fn restore_queue(playback: State<'_, PlaybackSlot>) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.restore().await?)
}

#[tauri::command]
pub fn next(playback: State<'_, PlaybackSlot>) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.next()?)
}

#[tauri::command]
pub fn previous(playback: State<'_, PlaybackSlot>) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.previous()?)
}

/// Click on a song of the queue list: only jumps to it.
#[tauri::command]
pub fn play_queue_index(playback: State<'_, PlaybackSlot>, index: usize) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.jump_to(index)?)
}

#[tauri::command]
pub fn set_shuffle(playback: State<'_, PlaybackSlot>, enabled: bool) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.set_shuffle(enabled)?)
}

/// Off → repeat queue → repeat one. Returns the new mode.
#[tauri::command]
pub fn cycle_repeat(playback: State<'_, PlaybackSlot>) -> Result<RepeatMode, CommandError> {
    Ok(playback_of(&playback)?.cycle_repeat()?)
}

#[tauri::command]
pub fn get_queue(playback: State<'_, PlaybackSlot>) -> Result<QueueState, CommandError> {
    Ok(playback_of(&playback)?.queue_state())
}

#[tauri::command]
pub fn toggle_pause(playback: State<'_, PlaybackSlot>) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.player().toggle_pause()?)
}

#[tauri::command]
pub fn seek(playback: State<'_, PlaybackSlot>, seconds: f64) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.player().seek(seconds)?)
}

/// Sets the volume (0–100) and returns the value applied.
#[tauri::command]
pub fn set_volume(playback: State<'_, PlaybackSlot>, volume: f64) -> Result<f64, CommandError> {
    Ok(playback_of(&playback)?.player().set_volume(volume)?)
}

/// Current state, for a screen that opens while something is playing.
#[tauri::command]
pub fn get_player_state(playback: State<'_, PlaybackSlot>) -> Result<PlayerState, CommandError> {
    Ok(playback_of(&playback)?.player().state())
}

fn playback_of(slot: &State<'_, PlaybackSlot>) -> Result<Arc<Playback>, CommandError> {
    slot.0.clone().map_err(|detail| CommandError::Playback { detail })
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path().app_config_dir().map_err(|e| CommandError::Storage {
        detail: e.to_string(),
    })
}

/// `{local data dir}/libraries/{user@server-hash}.db`. Local (not roaming) on
/// Windows, since the cache can get large.
fn active_library(
    app: &AppHandle,
    client: SubsonicClient,
    session: &SavedSession,
) -> Result<ActiveLibrary, CommandError> {
    let dir = app.path().app_local_data_dir().map_err(|e| CommandError::Storage {
        detail: e.to_string(),
    })?;
    let file = db::library_file_name(&session.server_url, &session.username);
    Ok(ActiveLibrary {
        client: Arc::new(client),
        db_path: dir.join("libraries").join(file),
    })
}

// Credential store and database calls block the thread, so they run outside
// the async runtime.
async fn run_blocking<T, E, F>(task: F) -> Result<T, CommandError>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Into<CommandError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| CommandError::Storage { detail: e.to_string() })?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn api_error_serializes_with_kind_tag() {
        let error = CommandError::from(SubsonicError::Api {
            code: 40,
            message: "Wrong username or password".into(),
        });

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "kind": "api", "code": 40, "message": "Wrong username or password" })
        );
    }

    #[test]
    fn unit_variants_serialize_in_camel_case() {
        assert_eq!(
            serde_json::to_value(CommandError::InvalidUrl).unwrap(),
            json!({ "kind": "invalidUrl" })
        );
        assert_eq!(
            serde_json::to_value(CommandError::HttpStatus { status: 404 }).unwrap(),
            json!({ "kind": "httpStatus", "status": 404 })
        );
    }

    #[test]
    fn invalid_response_maps_to_its_own_kind() {
        let error = CommandError::from(SubsonicError::InvalidResponse("html".into()));
        assert_eq!(error, CommandError::InvalidResponse);
    }

    #[test]
    fn empty_library_error_reaches_the_ui() {
        let error = CommandError::from(SyncError::EmptyLibrary(5120));
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "kind": "emptyLibrary", "reported": 5120 })
        );
    }
}
