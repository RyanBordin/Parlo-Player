//! Tauri commands exposed to the UI.

use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

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
use crate::db::songs::{self, SongSort};
use crate::playback::{AlbumsPlayMode, Playback, PlaybackError, QueueState};
use crate::player::queue::RepeatMode;
use crate::player::{PlayerError, PlayerState};
use crate::session::{self, SavedSession, SessionError};
use crate::state::{ActiveLibrary, AppState, PlaybackSlot};
use crate::subsonic::{AlbumListType, ServerInfo, StarItem, SubsonicClient, SubsonicError};
use crate::sync::{self, SyncError, SyncProgress};
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

#[derive(Debug, Clone, Copy)]
enum SyncKind {
    /// Full sync if due (or forced), then the light one.
    Rules { force_full: bool },
    Light,
}

fn start_sync(app: AppHandle, state: &Arc<AppState>, kind: SyncKind) -> Result<bool, CommandError> {
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
        let result = match kind {
            SyncKind::Rules { force_full } => sync::run(&active.client, active.db_path, force_full, &emit_progress).await,
            SyncKind::Light => sync::run_light(&active.client, active.db_path, &emit_progress).await,
        };

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
    run_blocking(move || home::activity(&db::open(&active.db_path)?, frequent)).await
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
    run_blocking(move || home::section(&db::open(&active.db_path)?, section, server)).await
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
        eprintln!("commands: star saved on the server but not locally: {error:?}");
    }
    if let Ok(playback) = &playback.0 {
        playback.song_starred(&song_id, starred);
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
        eprintln!("commands: album star saved on the server but not locally: {error:?}");
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
        Ok((playlist, songs)) => {
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
                eprintln!("commands: could not keep the playlist for offline use: {error:?}");
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

/// "Shuffle" of the Songs page: at most 100 songs of the list (spec 9).
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

/// The buttons of the Albums page, on the albums shown (spec 6.4).
#[tauri::command]
pub async fn play_albums(
    playback: State<'_, PlaybackSlot>,
    album_ids: Vec<String>,
    mode: AlbumsPlayMode,
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

/// "Random mix" of the Artists page: at most 100 songs of the artists shown (spec 6.6).
#[tauri::command]
pub async fn play_artists_mix(
    playback: State<'_, PlaybackSlot>,
    artist_ids: Vec<String>,
    context_label: String,
) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_artists_mix(&artist_ids, context_label))
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
                    eprintln!("commands: could not keep the artist info: {error:?}");
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
        eprintln!("commands: artist star saved on the server but not locally: {error:?}");
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

/// Up Next: removes a song from the queue.
#[tauri::command]
pub fn remove_from_queue(playback: State<'_, PlaybackSlot>, position: usize) -> Result<(), CommandError> {
    Ok(playback_of(&playback)?.remove_from_queue(position)?)
}

/// "Save" of Up Next (spec 4.4, 9): a new playlist on the server with these
/// songs, also kept locally so the sidebar shows it at once. Returns `false`
/// when the server did not send it back (a light sync will bring it).
#[tauri::command]
pub async fn create_playlist(
    state: State<'_, Arc<AppState>>,
    name: String,
    song_ids: Vec<String>,
) -> Result<bool, CommandError> {
    let active = state.active().ok_or(CommandError::NotLoggedIn)?;
    let Some(playlist) = active.client.create_playlist(&name, &song_ids).await? else {
        return Ok(false);
    };
    run_blocking(move || {
        let mut conn = db::open(&active.db_path)?;
        library::save_playlist(&conn, &playlist)?;
        tracks::save_playlist_songs(&mut conn, &playlist.id, &song_ids)
    })
    .await?;
    Ok(true)
}

/// "Shuffle" button of a list: shuffle on, from a random song.
#[tauri::command]
pub async fn play_shuffled(
    playback: State<'_, PlaybackSlot>,
    song_ids: Vec<String>,
    context_label: String,
) -> Result<(), CommandError> {
    let playback = playback_of(&playback)?;
    tauri::async_runtime::spawn_blocking(move || playback.play_shuffled(song_ids, context_label))
        .await
        .map_err(|e| CommandError::Playback { detail: e.to_string() })??;
    Ok(())
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

fn playback_of(slot: &State<'_, PlaybackSlot>) -> Result<Arc<Playback>, CommandError> {
    slot.0.clone().map_err(|detail| CommandError::Playback { detail })
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path().app_config_dir().map_err(|e| CommandError::Storage {
        detail: e.to_string(),
    })
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
