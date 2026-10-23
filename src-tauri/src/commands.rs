//! Tauri commands exposed to the UI.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::artist_page::{self, ArtistAbout};
use crate::covers::Covers;
use crate::db::artist_page::{self as artist_page_db, ArtistPage};
use crate::db::favorites::{self, Favorites};
use crate::db::library::{self, LibraryStats, PlaylistSummary, SongSummary};
use crate::db::related::SongContext;
use crate::db::tracks::{self, AlbumPage, PlaylistHeader, PlaylistPage, TrackListSong};
use crate::db::{self, DbError};
use crate::home::{self, HomeActivity, HomeLibrary, HomeSection, ROW_SIZE, SectionItems};
use crate::now_playing::{self, Lyrics, Related};
use crate::db::albums::{self, AlbumListItem};
use crate::db::artists::{self, ArtistListItem};
use crate::db::search::{self, SearchResults};
use crate::db::songs::{self, SongSort};
use crate::db::settings::LibrarySettings;
use crate::db::history;
use crate::downloads::{self, SaveItem, Saves, naming};
use crate::navidrome;
use crate::db::items::{AlbumInfo, ArtistInfo, SongInfo};
use crate::playback::{self, GroupsPlayMode, Placement, Playback, PlaybackError, QueueState};
use crate::playlists::{self, AddOutcome, Duplicates, PlaylistEntry};
use crate::player::AudioDevice;
use crate::player::queue::RepeatMode;
use crate::player::{EngineError, PlayerState};
use crate::session::{self, SavedSession, SessionError};
use crate::settings::{self, AppSettings, SettingsError};
use crate::state::{ActiveLibrary, AppState, PlaybackSlot};
use crate::subsonic::{
    Album, AlbumListType, Extension, MusicFolder, PlaylistUpdate, ServerInfo, StarItem, SubsonicClient, SubsonicError,
};
use crate::sync::{self, FullSync, SyncError, SyncProgress};
use crate::test_library;

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
    /// Signing in and out are off in the performance test mode.
    TestLibrary,
}

impl From<EngineError> for CommandError {
    fn from(error: EngineError) -> Self {
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

impl From<SettingsError> for CommandError {
    fn from(error: SettingsError) -> Self {
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
    if test_library::ENABLED {
        return open_test_library(&app, &state).await.map(Some);
    }
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
    if test_library::ENABLED {
        return Err(CommandError::TestLibrary);
    }
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
    // The real saved login must stay as it is.
    if test_library::ENABLED {
        return Err(CommandError::TestLibrary);
    }
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
    start_sync(app, &state, SyncKind::Rules { force_full })
}

/// Starts only the light sync (starred items and playlists), like
/// `sync_library`: `false` if a sync is already running (it brings them too).
#[tauri::command]
pub fn sync_light(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<bool, CommandError> {
    start_sync(app, &state, SyncKind::Light)
}

/// "Full sync" of Settings > Sync (spec 6.10): rebuilds the library from
/// scratch, keeping the play history. `false` if a sync is already running.
#[tauri::command]
pub fn rebuild_library(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<bool, CommandError> {
    start_sync(app, &state, SyncKind::Rebuild)
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SyncKind {
    /// Full sync if due (or forced), then the light one.
    Rules { force_full: bool },
    Light,
    /// Full sync rebuilding the library, then the light one.
    Rebuild,
}

pub(crate) fn start_sync(app: AppHandle, state: &Arc<AppState>, kind: SyncKind) -> Result<bool, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    // The test library has no server to sync with.
    if test_library::ENABLED {
        return Ok(false);
    }
    let Some(guard) = state.try_start_sync() else {
        return Ok(false);
    };

    tauri::async_runtime::spawn(async move {
        let emit_progress = |progress: SyncProgress| {
            let _ = app.emit(SYNC_PROGRESS_EVENT, progress);
        };
        let path = active.db_path.clone();
        let result = match kind {
            SyncKind::Rules { force_full } => {
                let mode = if force_full { FullSync::Forced } else { FullSync::IfDue };
                sync::run(&active.client, path, mode, &emit_progress).await
            }
            SyncKind::Light => sync::run_light(&active.client, path, &emit_progress).await,
            SyncKind::Rebuild => sync::run(&active.client, path, FullSync::Rebuild, &emit_progress).await,
        };
        let error = result.err().map(CommandError::from);

        // Spec 6.10: the date and result of the last sync, kept for the settings.
        let error_json = error.as_ref().and_then(|e| serde_json::to_string(e).ok());
        let path = active.db_path.clone();
        let recorded = run_blocking(move || {
            library::record_sync_result(&db::open(&path)?, sync::now(), error_json.as_deref())
        })
        .await;
        if let Err(e) = recorded {
            log::warn!("commands: could not keep the sync result: {e:?}");
        }

        // Clear the flag first, so the UI sees `syncing: false` when it reacts.
        drop(guard);
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

/// Home sections from the local cache: "Listen again" and favorites (spec 6.1).
#[tauri::command]
pub async fn get_home_library(state: State<'_, Arc<AppState>>) -> Result<HomeLibrary, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || home::library(&db::open(&active.db_path)?, crate::sync::now())).await
}

/// "Most played albums", asked from the server, with the local cache
/// standing in when it cannot be reached (spec 6.1).
#[tauri::command]
pub async fn get_home_activity(state: State<'_, Arc<AppState>>) -> Result<HomeActivity, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let frequent = active.client.get_album_list2(AlbumListType::Frequent, ROW_SIZE as u32, 0).await.ok();
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        let frequent = without_hidden_albums(&conn, frequent)?;
        home::activity(&conn, frequent)
    })
    .await
}

/// Everything starred, from the local cache, for the Favorites page (spec 6.2).
#[tauri::command]
pub async fn get_favorites(state: State<'_, Arc<AppState>>) -> Result<Favorites, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || favorites::favorites(&db::open(&active.db_path)?)).await
}

/// Every item of a Home section, for its "See more" grid (spec 6.1).
#[tauri::command]
pub async fn get_home_section(
    state: State<'_, Arc<AppState>>,
    section: HomeSection,
) -> Result<SectionItems, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let server = match section.server_list() {
        Some(kind) => active.client.get_all_album_list2(kind).await.ok(),
        None => None,
    };
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        let server = without_hidden_albums(&conn, server)?;
        home::section(&conn, section, server)
    })
    .await
}

/// With music folders hidden (spec 6.10), only the albums the library has:
/// the others are in a hidden folder.
fn without_hidden_albums(conn: &rusqlite::Connection, albums: Option<Vec<Album>>) -> Result<Option<Vec<Album>>, DbError> {
    let Some(mut albums) = albums else { return Ok(None) };
    if db::settings::load(conn)?.hidden_folders.is_empty() {
        return Ok(Some(albums));
    }
    let ids: Vec<String> = albums.iter().map(|a| a.id.clone()).collect();
    let known = library::known_album_ids(conn, &ids)?;
    albums.retain(|album| known.contains(&album.id));
    Ok(Some(albums))
}

/// Playlists from the local cache, by name (sidebar, spec 4.2).
#[tauri::command]
pub async fn list_playlists(state: State<'_, Arc<AppState>>) -> Result<Vec<PlaylistSummary>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        library::playlists(&conn)
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

/// Plays a whole album (Play button on its card).
#[tauri::command]
pub async fn play_album(playback: State<'_, PlaybackSlot>, album_id: String) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_album(&album_id))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
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

/// Stars or unstars a song on the server (spec 2), then in the local cache
/// and in the player. The UI changes first and goes back if this fails.
#[tauri::command]
pub async fn set_song_starred(
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    song_id: String,
    starred: bool,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    active.client.set_starred(StarItem::Song, std::slice::from_ref(&song_id), starred).await?;

    // The server already has it: a local failure is fixed by the next sync.
    let (db_path, id) = (active.db_path.clone(), song_id.clone());
    let saved = run_blocking(move || {
        let conn = db::open(&db_path)?;
        library::set_song_starred(&conn, &id, starred)
    })
    .await;
    if let Err(error) = saved {
        log::warn!("commands: star saved on the server but not locally: {error:?}");
    }
    if let Ok(playback) = &playback.0 {
        playback.song_starred(&song_id, starred);
    }
    Ok(())
}

/// "Favorite all" of a selection (spec 8.1): one request for every song.
#[tauri::command]
pub async fn set_songs_starred(
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    song_ids: Vec<String>,
    starred: bool,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    active.client.set_starred(StarItem::Song, &song_ids, starred).await?;

    let (db_path, ids) = (active.db_path.clone(), song_ids.clone());
    let saved = run_blocking(move || {
        let conn = db::open(&db_path)?;
        ids.iter().try_for_each(|id| library::set_song_starred(&conn, id, starred))
    })
    .await;
    if let Err(error) = saved {
        log::warn!("commands: stars saved on the server but not locally: {error:?}");
    }
    if let Ok(playback) = &playback.0 {
        for id in &song_ids {
            playback.song_starred(id, starred);
        }
    }
    Ok(())
}

/// Stars or unstars an album (heart of the album page, spec 6.5).
#[tauri::command]
pub async fn set_album_starred(
    state: State<'_, Arc<AppState>>,
    album_id: String,
    starred: bool,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    active.client.set_starred(StarItem::Album, std::slice::from_ref(&album_id), starred).await?;
    let saved = run_blocking(move || tracks::set_album_starred(&db::open(&active.db_path)?, &album_id, starred)).await;
    if let Err(error) = saved {
        log::warn!("commands: album star saved on the server but not locally: {error:?}");
    }
    Ok(())
}

/// An album and its songs, from the local cache (spec 6.5). `None` if the
/// album is not in the library.
#[tauri::command]
pub async fn get_album_page(
    state: State<'_, Arc<AppState>>,
    album_id: String,
) -> Result<Option<AlbumPage>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || tracks::album_page(&db::open(&active.db_path)?, &album_id)).await
}

/// A playlist in its saved order, asked from the server (and kept for
/// offline use), or from the local cache without connection (spec 6.5).
#[tauri::command]
pub async fn get_playlist_page(
    state: State<'_, Arc<AppState>>,
    playlist_id: String,
) -> Result<Option<PlaylistPage>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    match active.client.get_playlist(&playlist_id).await {
        Ok((playlist, mut songs)) => {
            // Spec 6.10: songs of hidden music folders are left out.
            let path = active.db_path.clone();
            let ids: Vec<String> = songs.iter().map(|s| s.id.clone()).collect();
            let known = run_blocking(move || {
                let conn = db::open(&path)?;
                if db::settings::load(&conn)?.hidden_folders.is_empty() {
                    return Ok::<_, DbError>(None);
                }
                Ok(Some(library::song_summaries(&conn, &ids)?))
            })
            .await?;
            if let Some(known) = known {
                songs.retain(|song| known.contains_key(&song.id));
            }
            let page = PlaylistPage {
                playlist: PlaylistHeader {
                    id: playlist.id.clone(),
                    name: playlist.name,
                    owner: playlist.owner,
                    cover_art: playlist.cover_art,
                    song_count: playlist.song_count,
                    duration: playlist.duration,
                },
                songs: songs.iter().map(TrackListSong::from).collect(),
                offline: false,
            };
            let ids: Vec<String> = songs.into_iter().map(|s| s.id).collect();
            let saved = run_blocking(move || {
                tracks::save_playlist_songs(&mut db::open(&active.db_path)?, &playlist.id, &ids)
            })
            .await;
            if let Err(error) = saved {
                log::warn!("commands: could not keep the playlist for offline use: {error:?}");
            }
            Ok(Some(page))
        }
        // Not found on the server: it was deleted.
        Err(SubsonicError::Api { code: 70, .. }) => Ok(None),
        Err(_) => run_blocking(move || tracks::cached_playlist_page(&db::open(&active.db_path)?, &playlist_id)).await,
    }
}

/// Every album for the Albums page, last added first (spec 6.4).
#[tauri::command]
pub async fn list_albums(state: State<'_, Arc<AppState>>) -> Result<Vec<AlbumListItem>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || albums::album_list(&db::open(&active.db_path)?)).await
}

/// The Songs page (spec 6.3): the songs that match `query`, in `sort`
/// order, and how many songs the library has.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongList {
    pub ids: Vec<String>,
    pub total: i64,
}

/// Quick search and sorting of the Songs page, run in SQLite (spec 7).
#[tauri::command]
pub async fn list_song_ids(
    state: State<'_, Arc<AppState>>,
    query: String,
    sort: Option<SongSort>,
) -> Result<SongList, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        let ids = songs::song_ids(&conn, &query, sort.unwrap_or(SongSort::DEFAULT))?;
        Ok::<_, DbError>(SongList { ids, total: songs::song_count(&conn)? })
    })
    .await
}

/// The lines of the Songs table that are in view, in the order asked.
#[tauri::command]
pub async fn get_song_rows(state: State<'_, Arc<AppState>>, ids: Vec<String>) -> Result<Vec<TrackListSong>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || songs::song_rows(&db::open(&active.db_path)?, &ids)).await
}

/// Global search of the Topbar (spec 6.9): songs, albums and artists, best
/// first, and which one is the best result.
#[tauri::command]
pub async fn search(state: State<'_, Arc<AppState>>, query: String) -> Result<SearchResults, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || search::search(&db::open(&active.db_path)?, &query)).await
}

/// The last searches, newest first (spec 6.9).
#[tauri::command]
pub async fn get_recent_searches(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || search::recent_searches(&db::open(&active.db_path)?)).await
}

/// Puts a search first in the recent list; gives the list back.
#[tauri::command]
pub async fn add_recent_search(state: State<'_, Arc<AppState>>, query: String) -> Result<Vec<String>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64);
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        search::add_recent_search(&conn, &query, now)?;
        search::recent_searches(&conn)
    })
    .await
}

/// Removes a search from the recent list; gives the list back.
#[tauri::command]
pub async fn remove_recent_search(state: State<'_, Arc<AppState>>, query: String) -> Result<Vec<String>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        search::remove_recent_search(&conn, &query)?;
        search::recent_searches(&conn)
    })
    .await
}

/// "Awesome Mix" of a list of songs: at most the mix size (spec 9).
#[tauri::command]
pub async fn play_mix(
    playback: State<'_, PlaybackSlot>,
    song_ids: Vec<String>,
    context_label: String,
) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_mix(song_ids, context_label))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// The three buttons of the Albums page, on the albums shown (spec 6.4, 9).
#[tauri::command]
pub async fn play_albums(
    playback: State<'_, PlaybackSlot>,
    album_ids: Vec<String>,
    mode: GroupsPlayMode,
    context_label: String,
) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_albums(&album_ids, mode, context_label))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// Album artists and featured artists for the Artists page, by name (spec 6.6).
#[tauri::command]
pub async fn list_artists(state: State<'_, Arc<AppState>>) -> Result<Vec<ArtistListItem>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || artists::artist_list(&db::open(&active.db_path)?)).await
}

/// The three buttons of the Artists page, on the artists shown (spec 6.6, 9).
#[tauri::command]
pub async fn play_artists(
    playback: State<'_, PlaybackSlot>,
    artist_ids: Vec<String>,
    mode: GroupsPlayMode,
    context_label: String,
) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_artists(&artist_ids, mode, context_label))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// The artist page from the local cache (spec 6.7). `None` if the artist is
/// not in the library.
#[tauri::command]
pub async fn get_artist_page(
    state: State<'_, Arc<AppState>>,
    artist_id: String,
) -> Result<Option<ArtistPage>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || artist_page_db::artist_page(&db::open(&active.db_path)?, &artist_id)).await
}

/// The artist page's parts from the server (spec 6.7): `getArtistInfo2`,
/// kept 7 days (the old answer stands in without connection), and the
/// `getTopSongs` order while the artist has no plays. `None` if the artist
/// is not in the library.
#[tauri::command]
pub async fn get_artist_about(
    state: State<'_, Arc<AppState>>,
    covers: State<'_, Arc<Covers>>,
    artist_id: String,
) -> Result<Option<ArtistAbout>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let (db_path, id) = (active.db_path.clone(), artist_id.clone());
    let inputs = run_blocking(move || artist_page_db::info_inputs(&db::open(&db_path)?, &id)).await?;
    let Some(inputs) = inputs else { return Ok(None) };

    let now = sync::now();
    let mut info = inputs.cached.clone().filter(|cached| artist_page::is_fresh(cached.fetched_at, now));
    if info.is_none() {
        info = match active.client.get_artist_info2(&artist_id).await {
            Ok(fetched) => {
                let has_photo = match inputs.cover_art.as_deref() {
                    Some(cover) => covers.is_artist_photo(&active.client, &active.db_path, cover).await,
                    None => Some(false),
                };
                let fresh = artist_page::to_cache(&fetched, has_photo, now);
                let (db_path, id, kept) = (active.db_path.clone(), artist_id.clone(), fresh.clone());
                let saved = run_blocking(move || {
                    artist_page_db::save_info(&db::open(&db_path)?, &id, &kept, &fetched.raw_json)
                })
                .await;
                if let Err(error) = saved {
                    log::warn!("commands: could not keep the artist info: {error:?}");
                }
                Some(fresh)
            }
            Err(_) => inputs.cached,
        };
    }

    let top_songs: Vec<String> = if inputs.has_plays {
        Vec::new()
    } else {
        let songs = active.client.get_top_songs(&inputs.name, artist_page::TOP_SONGS_COUNT).await;
        songs.unwrap_or_default().into_iter().map(|s| s.id).collect()
    };
    run_blocking(move || {
        artist_page::about(&db::open(&active.db_path)?, &artist_id, info.as_ref(), &top_songs).map(Some)
    })
    .await
}

/// Stars or unstars an artist (heart of the artist page, spec 6.7).
#[tauri::command]
pub async fn set_artist_starred(
    state: State<'_, Arc<AppState>>,
    artist_id: String,
    starred: bool,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    active.client.set_starred(StarItem::Artist, std::slice::from_ref(&artist_id), starred).await?;
    let saved = run_blocking(move || artists::set_artist_starred(&db::open(&active.db_path)?, &artist_id, starred)).await;
    if let Err(error) = saved {
        log::warn!("commands: artist star saved on the server but not locally: {error:?}");
    }
    Ok(())
}

/// Lyrics of a song (Now Playing, spec 4.4): synced ones from the
/// `songLyrics` extension, else plain ones (`getLyrics`), else none.
#[tauri::command]
pub async fn get_lyrics(state: State<'_, Arc<AppState>>, song_id: String) -> Result<Lyrics, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let structured = active.client.get_lyrics_by_song_id(&song_id).await?;
    if let Some(lyrics) = now_playing::from_structured(structured) {
        return Ok(lyrics);
    }
    let db_path = active.db_path.clone();
    let context = run_blocking(move || db::related::song_context(&db::open(&db_path)?, &song_id)).await?;
    let Some(SongContext { title, artist: Some(artist), .. }) = context else {
        return Ok(Lyrics::None);
    };
    Ok(now_playing::from_plain(active.client.get_lyrics(&artist, &title).await?))
}

/// The Related tab (spec 4.4): similar songs from the server, the rest from
/// the local cache. `None` if the song is not in the library.
#[tauri::command]
pub async fn get_related(state: State<'_, Arc<AppState>>, song_id: String) -> Result<Option<Related>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let (db_path, id) = (active.db_path.clone(), song_id.clone());
    let context = run_blocking(move || db::related::song_context(&db::open(&db_path)?, &id)).await?;
    let Some(context) = context else { return Ok(None) };

    // Only "You might also like" needs the server; without it, it is empty.
    let similar = match context.artist_id.as_deref() {
        Some(artist_id) => active
            .client
            .get_similar_songs2(artist_id, now_playing::SIMILAR_COUNT)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    run_blocking(move || {
        now_playing::related(&db::open(&active.db_path)?, &song_id, &context, similar).map(Some)
    })
    .await
}

/// Up Next: drags a song to another position of the queue.
#[tauri::command]
pub fn move_in_queue(playback: State<'_, PlaybackSlot>, from: usize, to: usize) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.move_in_queue(from, to)?)
}

/// Up Next: removes songs (one, or a selection) from the queue.
#[tauri::command]
pub fn remove_from_queue(playback: State<'_, PlaybackSlot>, positions: Vec<usize>) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.remove_from_queue(&positions)?)
}

/// What a "..." menu acts on (spec 8.1): songs (one, a selection or a
/// list), or a whole album, artist or playlist.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ItemTarget {
    Songs { ids: Vec<String> },
    Album { id: String },
    Artist { id: String },
    Playlist { id: String },
}

/// The songs of an item, in its order: an album by disc and track, an artist
/// as on its page, a playlist as saved (from the server, or the local cache
/// without connection).
async fn songs_of(active: &ActiveLibrary, target: ItemTarget) -> Result<Vec<String>, CommandError> {
    let db_path = active.db_path.clone();
    match target {
        ItemTarget::Songs { ids } => Ok(ids),
        ItemTarget::Album { id } => {
            run_blocking(move || db::home::album_songs(&db::open(&db_path)?, &id).map(|(_, ids)| ids)).await
        }
        ItemTarget::Artist { id } => run_blocking(move || artist_page_db::song_ids(&db::open(&db_path)?, &id)).await,
        ItemTarget::Playlist { id } => match active.client.get_playlist(&id).await {
            Ok((_, songs)) => Ok(songs.into_iter().map(|song| song.id).collect()),
            Err(_) => run_blocking(move || {
                let page = tracks::cached_playlist_page(&db::open(&db_path)?, &id)?;
                Ok::<_, DbError>(page.map(|page| page.songs.into_iter().map(|song| song.id).collect()).unwrap_or_default())
            })
            .await,
        },
    }
}

/// "Play", "Random order" and "Awesome Mix" of an album, artist or
/// playlist (spec 8.1, 9): `GroupsPlayMode` on the item. An artist's groups
/// are its albums; the other items are one group.
#[tauri::command]
pub async fn play_item(
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    target: ItemTarget,
    mode: GroupsPlayMode,
    context_label: String,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let playback = playback_of(&playback)?;
    let groups = match (&target, mode) {
        (ItemTarget::Artist { id }, GroupsPlayMode::Shuffled) => {
            let (db_path, id) = (active.db_path.clone(), id.clone());
            run_blocking(move || artist_page_db::album_groups(&db::open(&db_path)?, &id)).await?
        }
        _ => vec![songs_of(&active, target).await?],
    };
    tauri::async_runtime::spawn_blocking(move || match mode {
        // "Play" keeps the item's own order (an artist: most played first).
        GroupsPlayMode::InOrder => playback.play_context(groups.concat(), 0, context_label),
        _ => playback.play_groups(groups, mode, context_label),
    })
    .await
    .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// "Play next" / "Add to queue" (spec 8.1). With nothing queued, the songs
/// start playing, "Playing from" `context_label`.
#[tauri::command]
pub async fn enqueue(
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    target: ItemTarget,
    placement: Placement,
    context_label: String,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let playback = playback_of(&playback)?;
    let ids = songs_of(&active, target).await?;
    tauri::async_runtime::spawn_blocking(move || playback.enqueue(ids, placement, context_label))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// "Play similar songs" of a song (spec 8.1): `getSimilarSongs2` with the
/// song's ID; without Last.fm on the server, the local mix of the artist
/// radio around the song's artist and genre.
#[tauri::command]
pub async fn play_similar_songs(
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    song_id: String,
    context_label: String,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let size = usize::from(state.settings().mix_size);
    let similar = active.client.get_similar_songs2(&song_id, size as u32).await.unwrap_or_default();
    let mut ids: Vec<String> = similar.into_iter().map(|song| song.id).filter(|id| *id != song_id).collect();
    if ids.is_empty() {
        ids = run_blocking(move || {
            let conn = db::open(&active.db_path)?;
            let Some(context) = db::related::song_context(&conn, &song_id)? else {
                return Ok::<_, DbError>(Vec::new());
            };
            let Some(artist_id) = context.artist_id else { return Ok(Vec::new()) };
            let mut ids = db::items::radio_fallback(&conn, &artist_id, context.genre.as_deref(), size + 1)?;
            ids.retain(|id| *id != song_id);
            ids.truncate(size);
            Ok(shuffled(ids))
        })
        .await?;
    }
    play_radio(&playback, ids, context_label).await
}

/// "Artist radio" (spec 8.2): songs like the artist's (`getSimilarSongs2`
/// with the artist's ID); without Last.fm, the artist's most played songs
/// mixed with songs of the same genre. At most the mix size (spec 9).
#[tauri::command]
pub async fn play_artist_radio(
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    artist_id: String,
    context_label: String,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let size = usize::from(state.settings().mix_size);
    let similar = active.client.get_similar_songs2(&artist_id, size as u32).await.unwrap_or_default();
    let mut ids: Vec<String> = similar.into_iter().map(|song| song.id).collect();
    if ids.is_empty() {
        ids = run_blocking(move || {
            db::items::radio_fallback(&db::open(&active.db_path)?, &artist_id, None, size).map(shuffled)
        })
        .await?;
    }
    play_radio(&playback, ids, context_label).await
}

fn shuffled(mut ids: Vec<String>) -> Vec<String> {
    use rand::seq::SliceRandom;
    ids.shuffle(&mut rand::rng());
    ids
}

/// A radio plays in the order built, from its first song.
async fn play_radio(playback: &State<'_, PlaybackSlot>, ids: Vec<String>, label: String) -> Result<(), CommandError> {
    if ids.is_empty() {
        return Err(CommandError::SongNotFound);
    }
    let playback = playback_of(playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_context(ids, 0, label))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
}

/// A playlist changed on the server: reads it again into the local cache, so
/// the sidebar and the "last used" order follow. A failure is only logged:
/// the next light sync fixes it.
async fn refresh_playlist(active: &ActiveLibrary, playlist_id: &str) {
    let result = match active.client.get_playlist(playlist_id).await {
        Ok((playlist, songs)) => {
            let db_path = active.db_path.clone();
            run_blocking(move || {
                let mut conn = db::open(&db_path)?;
                library::save_playlist(&conn, &playlist)?;
                let ids: Vec<String> = songs.into_iter().map(|song| song.id).collect();
                tracks::save_playlist_songs(&mut conn, &playlist.id, &ids)
            })
            .await
        }
        Err(error) => Err(error.into()),
    };
    if let Err(error) = result {
        log::warn!("commands: could not read the changed playlist again: {error:?}");
    }
}

/// "+ New playlist" (sidebar, "Add to playlist" › and the Save of Up Next,
/// spec 4.4, 8.2, 9): a new playlist on the server with the item's songs,
/// also kept locally so the sidebar shows it at once. Returns its ID, `None`
/// when the server did not send it back (a light sync will bring it).
#[tauri::command]
pub async fn create_playlist(
    state: State<'_, Arc<AppState>>,
    name: String,
    target: ItemTarget,
) -> Result<Option<String>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let song_ids = songs_of(&active, target).await?;
    let Some(playlist) = active.client.create_playlist(&name, &song_ids).await? else {
        return Ok(None);
    };
    let id = playlist.id.clone();
    run_blocking(move || {
        let mut conn = db::open(&active.db_path)?;
        library::save_playlist(&conn, &playlist)?;
        tracks::save_playlist_songs(&mut conn, &playlist.id, &song_ids)
    })
    .await?;
    Ok(Some(id))
}

/// "Add to playlist ›" (spec 8.2). Without `duplicates`, nothing is added
/// when some songs are already there: the answer says how many, and the UI
/// asks again with the user's choice.
#[tauri::command]
pub async fn add_to_playlist(
    state: State<'_, Arc<AppState>>,
    playlist_id: String,
    target: ItemTarget,
    duplicates: Option<Duplicates>,
) -> Result<AddOutcome, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let adding = songs_of(&active, target).await?;
    let (_, existing) = active.client.get_playlist(&playlist_id).await?;
    let existing: Vec<String> = existing.into_iter().map(|song| song.id).collect();

    let choice = match duplicates {
        Some(choice) => choice,
        None => match playlists::duplicate_count(&existing, &adding) {
            0 => Duplicates::AddAll,
            count => return Ok(AddOutcome::Duplicates { count, total: adding.len() }),
        },
    };
    let add = playlists::songs_to_add(&existing, adding, choice);
    if !add.is_empty() {
        let update = PlaylistUpdate { add, ..PlaylistUpdate::default() };
        active.client.update_playlist(&playlist_id, &update).await?;
        refresh_playlist(&active, &playlist_id).await;
        return Ok(AddOutcome::Added { count: update.add.len() });
    }
    Ok(AddOutcome::Added { count: 0 })
}

/// "Rename / edit description" and "Make public / private" (spec 8.1).
/// `None` keeps a field as it is.
#[tauri::command]
pub async fn update_playlist(
    state: State<'_, Arc<AppState>>,
    playlist_id: String,
    name: Option<String>,
    comment: Option<String>,
    public: Option<bool>,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let update = PlaylistUpdate { name, comment, public, ..PlaylistUpdate::default() };
    active.client.update_playlist(&playlist_id, &update).await?;
    refresh_playlist(&active, &playlist_id).await;
    Ok(())
}

/// "Remove from this playlist" (spec 8.1), one song or a selection.
#[tauri::command]
pub async fn remove_from_playlist(
    state: State<'_, Arc<AppState>>,
    playlist_id: String,
    entries: Vec<PlaylistEntry>,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let (_, songs) = active.client.get_playlist(&playlist_id).await?;
    let ids: Vec<String> = songs.into_iter().map(|song| song.id).collect();
    let remove = playlists::server_positions(&ids, &entries);
    if remove.is_empty() {
        return Ok(());
    }
    let update = PlaylistUpdate { remove, ..PlaylistUpdate::default() };
    active.client.update_playlist(&playlist_id, &update).await?;
    refresh_playlist(&active, &playlist_id).await;
    Ok(())
}

/// "Delete playlist" (spec 8.1), after the UI asked.
#[tauri::command]
pub async fn delete_playlist(state: State<'_, Arc<AppState>>, playlist_id: String) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    active.client.delete_playlist(&playlist_id).await?;
    run_blocking(move || db::items::delete_playlist(&db::open(&active.db_path)?, &playlist_id)).await
}

/// "File information" of a song (spec 8.1), from the local cache.
#[tauri::command]
pub async fn get_song_info(state: State<'_, Arc<AppState>>, song_id: String) -> Result<Option<SongInfo>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || db::items::song_info(&db::open(&active.db_path)?, &song_id)).await
}

/// "Information" of an album (spec 8.1), from the local cache.
#[tauri::command]
pub async fn get_album_info(state: State<'_, Arc<AppState>>, album_id: String) -> Result<Option<AlbumInfo>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || db::items::album_info(&db::open(&active.db_path)?, &album_id)).await
}

/// What "Save to computer..." saves (spec 8.2): one song or a selection, an
/// album, a playlist.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SaveTarget {
    Songs { ids: Vec<String> },
    Album { id: String },
    Playlist { id: String },
}

/// How the songs of a save are laid out, before the user chose where.
enum SaveLayout {
    Songs,
    Album { folder_name: String, cover_art: Option<String> },
    Playlist { name: String },
}

/// "Save to computer..." (spec 8.2): asks where with the system's dialog
/// ("Save as" for one song, a folder otherwise, opening on the folder chosen
/// last, else the system's Music folder), then queues the original files.
/// Returns how many songs, `None` if the dialog was closed.
#[tauri::command]
pub async fn save_to_computer(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    saves: State<'_, Arc<Saves>>,
    target: SaveTarget,
) -> Result<Option<usize>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let db_path = active.db_path.clone();
    let (layout, ids) = match target {
        SaveTarget::Songs { ids } => (SaveLayout::Songs, ids),
        SaveTarget::Album { id } => {
            let page = run_blocking(move || tracks::album_page(&db::open(&db_path)?, &id))
                .await?
                .ok_or(CommandError::SongNotFound)?;
            let album = page.album;
            let folder_name = naming::album_folder(album.artist.as_deref(), &album.name, album.year);
            let ids = page.songs.into_iter().map(|song| song.id).collect();
            (SaveLayout::Album { folder_name, cover_art: album.cover_art }, ids)
        }
        SaveTarget::Playlist { id } => match active.client.get_playlist(&id).await {
            Ok((playlist, songs)) => {
                (SaveLayout::Playlist { name: playlist.name }, songs.into_iter().map(|song| song.id).collect())
            }
            Err(_) => {
                let page = run_blocking(move || tracks::cached_playlist_page(&db::open(&db_path)?, &id))
                    .await?
                    .ok_or(CommandError::SongNotFound)?;
                let ids = page.songs.into_iter().map(|song| song.id).collect();
                (SaveLayout::Playlist { name: page.playlist.name }, ids)
            }
        },
    };
    let db_path = active.db_path.clone();
    let mut songs = run_blocking(move || db::items::save_songs(&db::open(&db_path)?, &ids)).await?;
    if songs.is_empty() {
        return Err(CommandError::SongNotFound);
    }

    let config = config_dir(&app)?;
    let start_dir = downloads::last_used_folder(&config)
        .or_else(|| app.path().audio_dir().ok())
        .or_else(|| app.path().home_dir().ok());
    let single = matches!(layout, SaveLayout::Songs) && songs.len() == 1;
    let suggested = single.then(|| (naming::song_file(&songs[0]), naming::extension(&songs[0])));
    let dialog_app = app.clone();
    // The system's dialog waits for the user: not on the async threads.
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = dialog_app.dialog().file();
        if let Some(dir) = &start_dir {
            dialog = dialog.set_directory(dir);
        }
        #[cfg(desktop)]
        if let Some(window) = dialog_app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
        let picked = match suggested {
            Some((name, ext)) => dialog.set_file_name(name).add_filter(ext.to_uppercase(), &[ext.as_str()]).blocking_save_file(),
            #[cfg(desktop)]
            None => dialog.blocking_pick_folder(),
            // Android has no folder picker: saving several songs waits for its own downloads.
            #[cfg(mobile)]
            None => None,
        };
        picked.and_then(|path| path.into_path().ok())
    })
    .await
    .map_err(|e| CommandError::Storage { detail: e.to_string() })?;
    let Some(mut chosen) = chosen else { return Ok(None) };

    let item = match layout {
        SaveLayout::Songs if single => {
            let song = songs.remove(0);
            if chosen.extension().is_none() {
                chosen.set_extension(naming::extension(&song));
            }
            SaveItem::Song { song, path: chosen.clone() }
        }
        SaveLayout::Songs => SaveItem::Selection { songs },
        SaveLayout::Album { folder_name, cover_art } => SaveItem::Album { folder_name, cover_art, songs },
        SaveLayout::Playlist { name } => SaveItem::Playlist { name, songs },
    };
    let folder = if single { chosen.parent().map(Path::to_path_buf).unwrap_or_default() } else { chosen.clone() };
    if let Err(error) = downloads::remember_folder(&config, &folder) {
        log::warn!("commands: could not remember the save folder: {error}");
    }

    let plan = run_blocking(move || {
        downloads::plan(item, &folder).map_err(|e| CommandError::Storage { detail: e.to_string() })
    })
    .await?;
    let count = plan.song_count();
    saves.start(Arc::clone(&active.client), plan);
    Ok(Some(count))
}

/// "Cancel" of the save progress (spec 8.2).
#[tauri::command]
pub fn cancel_saves(saves: State<'_, Arc<Saves>>) {
    saves.cancel();
}

/// "Open folder" once a save finished (spec 8.2).
#[tauri::command]
pub fn open_save_folder(app: AppHandle, saves: State<'_, Arc<Saves>>) -> Result<(), CommandError> {
    let Some(folder) = saves.last_folder() else { return Ok(()) };
    app.opener()
        .open_path(folder.to_string_lossy(), None::<&str>)
        .map_err(|e| CommandError::Storage { detail: e.to_string() })
}

/// Whether the server is a Navidrome ("Open in Navidrome", spec 8.2). `false`
/// when it cannot be reached; asked again later.
#[tauri::command]
pub async fn is_navidrome(state: State<'_, Arc<AppState>>) -> Result<bool, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    Ok(active.client.server_type().await.is_ok_and(|kind| navidrome::is_navidrome(kind.as_deref())))
}

/// What "Open in Navidrome" opens.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NavidromeItem {
    /// Navidrome has no song page: its album's opens.
    Song,
    Album,
    Artist,
}

/// "Open in Navidrome" (spec 8.2): the item's page in the default browser.
/// The address is the server's, without credentials.
#[tauri::command]
pub async fn open_in_navidrome(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    kind: NavidromeItem,
    id: String,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let (page, id) = match kind {
        NavidromeItem::Album => (navidrome::Page::Album, id),
        NavidromeItem::Artist => (navidrome::Page::Artist, id),
        NavidromeItem::Song => {
            let db_path = active.db_path.clone();
            let info = run_blocking(move || db::items::song_info(&db::open(&db_path)?, &id)).await?;
            let album_id = info.and_then(|song| song.album_id).ok_or(CommandError::SongNotFound)?;
            (navidrome::Page::Album, album_id)
        }
    };
    let url = navidrome::page_url(active.client.base_url(), page, &id);
    app.opener().open_url(url, None::<&str>).map_err(|e| CommandError::Storage { detail: e.to_string() })
}

/// "Information" of an artist (spec 8.1), from the local cache.
#[tauri::command]
pub async fn get_artist_info(
    state: State<'_, Arc<AppState>>,
    artist_id: String,
) -> Result<Option<ArtistInfo>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || db::items::artist_info(&db::open(&active.db_path)?, &artist_id)).await
}


#[tauri::command]
pub fn toggle_pause(playback: State<'_, PlaybackSlot>) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.player().toggle_pause()?)
}

#[tauri::command]
pub fn seek(playback: State<'_, PlaybackSlot>, seconds: f64) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.player().seek(seconds)?)
}

/// Jumps `seconds` forward (negative: back), e.g. ±5 s from the keyboard.
#[tauri::command]
pub fn seek_by(playback: State<'_, PlaybackSlot>, seconds: f64) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.player().seek_by(seconds)?)
}

/// Mutes or unmutes, keeping the volume. Returns the new state.
#[tauri::command]
pub fn toggle_mute(playback: State<'_, PlaybackSlot>) -> Result<bool, CommandError> {
    Ok(playback_of(&playback)?.player().toggle_mute()?)
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

/// Settings > Server and account (spec 6.10): asked when the sector opens
/// and by "Test connection".
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: bool,
    /// Why the server could not be reached.
    pub error: Option<CommandError>,
    pub server: Option<ServerInfo>,
    /// OpenSubsonic extensions the server announces.
    pub extensions: Vec<Extension>,
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, Arc<AppState>>) -> Result<ServerStatus, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    match active.client.ping().await {
        Ok(info) => {
            let extensions = if info.open_subsonic {
                active.client.get_open_subsonic_extensions().await.unwrap_or_default()
            } else {
                Vec::new()
            };
            Ok(ServerStatus { online: true, error: None, server: Some(info), extensions })
        }
        Err(error) => Ok(ServerStatus { online: false, error: Some(error.into()), server: None, extensions: Vec::new() }),
    }
}

/// Settings > Sync (spec 6.10): the last sync and the plays waiting to be sent.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// Unix seconds.
    pub last_sync_at: Option<i64>,
    /// The `CommandError` of the last sync, if it failed.
    pub last_error: Option<serde_json::Value>,
    pub pending_scrobbles: i64,
    pub syncing: bool,
}

#[tauri::command]
pub async fn get_sync_status(state: State<'_, Arc<AppState>>) -> Result<SyncStatus, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let syncing = state.is_syncing();
    run_blocking(move || {
        let conn = db::open(&active.db_path)?;
        let result = library::sync_result(&conn)?;
        Ok::<_, DbError>(SyncStatus {
            last_sync_at: result.at,
            last_error: result.error.and_then(|text| serde_json::from_str(&text).ok()),
            pending_scrobbles: history::pending_count(&conn)?,
            syncing,
        })
    })
    .await
}

/// "Send again" of the pending plays. Returns how many were sent.
#[tauri::command]
pub async fn resend_scrobbles(state: State<'_, Arc<AppState>>) -> Result<usize, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    Ok(playback::send_all_pending_scrobbles(&active.client, &active.db_path).await?)
}

/// Settings > Library (spec 6.10): counts and space on disk.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub songs: i64,
    pub albums: i64,
    pub artists: i64,
    pub database_bytes: u64,
    pub cover_cache_bytes: u64,
}

#[tauri::command]
pub async fn get_library_summary(
    state: State<'_, Arc<AppState>>,
    covers: State<'_, Arc<Covers>>,
) -> Result<LibrarySummary, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let covers = Arc::clone(&covers);
    run_blocking(move || {
        let stats = library::stats(&db::open(&active.db_path)?)?;
        Ok::<_, DbError>(LibrarySummary {
            songs: stats.songs,
            albums: stats.albums,
            artists: stats.artists,
            database_bytes: library::database_size(&active.db_path),
            cover_cache_bytes: covers.cache().size(),
        })
    })
    .await
}

/// The server's music folders (spec 6.10), to choose which ones show.
#[tauri::command]
pub async fn get_music_folders(state: State<'_, Arc<AppState>>) -> Result<Vec<MusicFolder>, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    Ok(active.client.get_music_folders().await?)
}

/// "Clear cache" of the covers: they are downloaded again when shown.
#[tauri::command]
pub async fn clear_cover_cache(covers: State<'_, Arc<Covers>>) -> Result<(), CommandError> {
    let covers = Arc::clone(&covers);
    run_blocking(move || covers.cache().clear().map_err(DbError::from)).await
}

/// Links of Settings > About, fixed here so the interface cannot open anything else.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalLink {
    Repository,
    Email,
}

const REPOSITORY_URL: &str = "https://github.com/RyanBordin/Parlo-Player";
const CONTACT_EMAIL: &str = "mailto:ryanbordin@proton.me";

#[tauri::command]
pub fn open_link(app: AppHandle, link: ExternalLink) -> Result<(), CommandError> {
    let url = match link {
        ExternalLink::Repository => REPOSITORY_URL,
        ExternalLink::Email => CONTACT_EMAIL,
    };
    app.opener().open_url(url, None::<&str>).map_err(|e| CommandError::Storage { detail: e.to_string() })
}

/// "Open logs folder" (spec 6.10): for reporting problems.
#[tauri::command]
pub fn open_logs_folder(app: AppHandle) -> Result<(), CommandError> {
    let dir = app.path().app_log_dir().map_err(|e| CommandError::Storage { detail: e.to_string() })?;
    std::fs::create_dir_all(&dir).map_err(|e| CommandError::Storage { detail: e.to_string() })?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| CommandError::Storage { detail: e.to_string() })
}

fn playback_of(slot: &State<'_, PlaybackSlot>) -> Result<Arc<Playback>, CommandError> {
    slot.0.clone().map_err(|detail| CommandError::Playback { detail })
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path().app_config_dir().map_err(|e| CommandError::Storage {
        detail: e.to_string(),
    })
}

/// The app preferences (spec 6.10), or the defaults.
#[tauri::command]
pub async fn get_app_settings(app: AppHandle) -> Result<AppSettings, CommandError> {
    let dir = config_dir(&app)?;
    run_blocking(move || Ok::<_, SettingsError>(settings::load(&dir))).await
}

/// Saves the app preferences and applies the ones the backend handles (the
/// interface scale, audio and streaming). Returns them as saved (values out
/// of range fixed).
#[tauri::command]
pub async fn save_app_settings(
    app: AppHandle,
    window: tauri::WebviewWindow,
    state: State<'_, Arc<AppState>>,
    playback: State<'_, PlaybackSlot>,
    covers: State<'_, Arc<Covers>>,
    settings: AppSettings,
) -> Result<AppSettings, CommandError> {
    let settings = settings.normalized();
    let dir = config_dir(&app)?;
    let saved = settings.clone();
    run_blocking(move || settings::save(&dir, &saved)).await?;

    let old = state.set_settings(settings.clone());
    if old.scale != settings.scale
        && let Err(error) = settings::apply_scale(&window, settings.scale)
    {
        log::warn!("commands: could not apply the interface scale: {error}");
    }
    if old.cover_cache_mb != settings.cover_cache_mb {
        let (covers, limit) = (Arc::clone(&covers), settings.cover_cache_bytes());
        // A smaller limit deletes files.
        tauri::async_runtime::spawn_blocking(move || covers.cache().set_limit(limit));
    }
    if let Ok(playback) = &playback.0 {
        let (playback, new) = (Arc::clone(playback), settings.clone());
        // Loading the next song reads the library cache.
        tauri::async_runtime::spawn_blocking(move || playback.settings_changed(&old, &new));
    }
    Ok(settings)
}

/// The audio outputs (spec 6.10), the system's default first.
#[tauri::command]
pub fn get_audio_devices(playback: State<'_, PlaybackSlot>) -> Result<Vec<AudioDevice>, CommandError> {
    Ok(playback_of(&playback)?.audio_devices())
}

/// The preferences of the signed-in server (spec 6.10).
#[tauri::command]
pub async fn get_library_settings(state: State<'_, Arc<AppState>>) -> Result<LibrarySettings, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || db::settings::load(&db::open(&active.db_path)?)).await
}

#[tauri::command]
pub async fn save_library_settings(
    state: State<'_, Arc<AppState>>,
    settings: LibrarySettings,
) -> Result<(), CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    run_blocking(move || db::settings::save(&mut db::open(&active.db_path)?, &settings)).await
}

/// Whether this build is the performance test mode (`npm run tauri:perf`).
#[tauri::command]
pub fn is_test_library() -> bool {
    test_library::ENABLED
}

/// Performance test mode: the made-up library of 50 000 songs, created on
/// first use, with a server that does not exist. The real login and
/// library are not touched.
async fn open_test_library(app: &AppHandle, state: &State<'_, Arc<AppState>>) -> Result<SavedSession, CommandError> {
    let db_path = test_library::db_path(&local_data_dir(app)?);
    let path = db_path.clone();
    run_blocking(move || test_library::ensure(&path)).await?;
    let session = test_library::session();
    let client = SubsonicClient::new(session.server_url.clone(), session.username.clone(), String::new())?;
    state.set_active(Some(ActiveLibrary { client: Arc::new(client), db_path }));
    Ok(session)
}

/// Local (not roaming) on Windows, since the cache can get large.
fn local_data_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path().app_local_data_dir().map_err(|e| CommandError::Storage {
        detail: e.to_string(),
    })
}

/// `{local data dir}/libraries/{user@server-hash}.db`.
fn active_library(
    app: &AppHandle,
    client: SubsonicClient,
    session: &SavedSession,
) -> Result<ActiveLibrary, CommandError> {
    let dir = local_data_dir(app)?;
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
