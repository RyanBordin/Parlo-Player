//! Playback controller (spec 5.1, 5.1.1, 5.4, 5.6, 5.7).
//!
//! The queue lives here and is the source of truth; the player (mpv) only
//! knows the current song and the preloaded next one. The controller also
//! counts the time really listened, scrobbles plays, and saves the queue
//! (locally and on the server) so the next start can resume it.
//!
//! Player events arrive through a channel and are handled on the
//! `playback-events` thread, so a command holding the controller lock never
//! waits for its own events. Saves go to the `queue-saver` thread, which only
//! writes the latest one when several pile up.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;
use rand::{Rng, RngExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use thiserror::Error;

use crate::db::library::SongSummary;
use crate::db::{self, DbError, DbResult, history, library, saved_queue};
use crate::player::listen::ListenTracker;
use crate::player::queue::{Queue, RepeatMode, Step};
use crate::player::resume::{self, SavedQueue, ServerQueue, Source};
use crate::player::{PlaybackStatus, Player, PlayerError, PlayerEvent, Track};
use crate::state::{ActiveLibrary, AppState};
use crate::subsonic::{SecretUrl, SubsonicClient, SubsonicError};

pub const QUEUE_CHANGED_EVENT: &str = "queue-changed";
const PLAYER_STATE_EVENT: &str = "player-state";
const PLAYER_POSITION_EVENT: &str = "player-position";
const PLAYER_ERROR_EVENT: &str = "player-error";

/// Songs of a random mix of a big set (spec 9).
pub const MIX_SIZE: usize = 100;

/// The three buttons of the Albums page (spec 6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AlbumsPlayMode {
    /// The albums in the order shown, one after the other.
    InOrder,
    /// The albums in a random order, each album's songs in order.
    ShuffledAlbums,
    /// The songs of all the albums mixed, at most `MIX_SIZE`.
    Mix,
}

/// What a new queue does with the shuffle mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShuffleMode {
    /// As the user left it, from the clicked song.
    Keep,
    /// On, from a random song ("Shuffle" buttons).
    On,
    /// Off: the order given is the point (Albums page buttons).
    Off,
}

/// Pending scrobbles sent per request.
const SCROBBLE_BATCH: u32 = 100;

/// While playing, the queue (with the position) is saved this often (spec 5.6).
const SAVE_INTERVAL: Duration = Duration::from_secs(30);
/// Server timeout for a save in the background.
const SAVE_TIMEOUT: Duration = Duration::from_secs(5);
/// Server timeout for the last save, when the app closes.
const EXIT_SAVE_TIMEOUT: Duration = Duration::from_secs(3);

/// Payload of `queue-changed`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    /// "Playing from".
    pub label: String,
    /// "Continuing from [app]" (spec 5.6), shown instead of `label`.
    pub continuing_from: Option<String>,
    /// Songs in playback order.
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error(transparent)]
    Player(#[from] PlayerError),

    #[error(transparent)]
    Db(#[from] DbError),

    #[error(transparent)]
    Server(#[from] SubsonicError),

    #[error("not signed in")]
    NotLoggedIn,

    #[error("none of these songs is in the library")]
    NoSongs,

    #[error("background task failed: {0}")]
    Task(String),
}

#[derive(Default)]
struct Inner {
    queue: Queue,
    /// Song details for every ID in the queue.
    tracks: HashMap<String, Track>,
    listen: ListenTracker,
    /// Song whose listened time is being counted.
    counting: Option<String>,
    /// Whether the player was playing at its last state change.
    playing: bool,
    /// When the queue was last sent to the saver.
    last_save: Option<Instant>,
    /// Library whose saved queue was already restored (or tried).
    restored: Option<PathBuf>,
}

/// A queue to save for a library.
struct SaveJob {
    library: ActiveLibrary,
    saved: SavedQueue,
}

pub struct Playback {
    player: Player,
    inner: Mutex<Inner>,
    app: AppHandle,
    saver: mpsc::Sender<SaveJob>,
    /// Held while a save is written, so the final save on exit cannot be
    /// overwritten by an older one still in progress.
    writing: Arc<Mutex<()>>,
}

impl Playback {
    /// Starts the player and the thread that handles its events.
    pub fn start(app: AppHandle) -> Result<Arc<Self>, PlayerError> {
        let (tx, rx) = mpsc::channel::<PlayerEvent>();
        let player = Player::new(&[], move |event| {
            let _ = tx.send(event);
        })?;

        let (saver, jobs) = mpsc::channel::<SaveJob>();
        let writing = Arc::new(Mutex::new(()));
        thread::Builder::new().name("queue-saver".into()).spawn({
            let writing = Arc::clone(&writing);
            move || {
                while let Ok(mut job) = jobs.recv() {
                    // Several saves waiting: only the latest matters.
                    while let Ok(newer) = jobs.try_recv() {
                        job = newer;
                    }
                    write_save(&job, &writing, SAVE_TIMEOUT);
                }
            }
        })?;

        let inner = Mutex::new(Inner::default());
        let playback = Arc::new(Self { player, inner, app, saver, writing });

        // A weak reference, so the thread does not keep the controller alive.
        let weak: Weak<Self> = Arc::downgrade(&playback);
        thread::Builder::new().name("playback-events".into()).spawn(move || {
            for event in rx {
                let Some(playback) = weak.upgrade() else { break };
                playback.on_player_event(event);
            }
        })?;
        Ok(playback)
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    /// Replaces the queue with a context and plays from `start_index`
    /// (spec 5.1). IDs missing from the local cache are left out.
    pub fn play_context(
        &self,
        song_ids: Vec<String>,
        start_index: usize,
        label: String,
    ) -> Result<(), PlaybackError> {
        self.play_list(song_ids, start_index, label, ShuffleMode::Keep)
    }

    /// "Shuffle" button of a list (spec 6.5): turns shuffle on and starts
    /// at a random song, with the rest shuffled after it.
    pub fn play_shuffled(&self, song_ids: Vec<String>, label: String) -> Result<(), PlaybackError> {
        self.play_list(song_ids, 0, label, ShuffleMode::On)
    }

    /// The buttons of the Albums page (spec 6.4), on the albums shown, in the
    /// order shown. Shuffle turns off: the order built is the point.
    pub fn play_albums(&self, album_ids: &[String], mode: AlbumsPlayMode, label: String) -> Result<(), PlaybackError> {
        let library = self.library()?;
        let albums = read_db(&library.db_path, |conn| db::albums::songs_of_albums(conn, album_ids))?;
        let ids = arrange_albums(albums, mode, &mut rand::rng());
        self.play_list(ids, 0, label, ShuffleMode::Off)
    }

    /// Replaces the queue with `song_ids` and plays from `start_index` (or a
    /// random song, with shuffle `On`).
    fn play_list(
        &self,
        song_ids: Vec<String>,
        start_index: usize,
        label: String,
        shuffle: ShuffleMode,
    ) -> Result<(), PlaybackError> {
        let library = self.library()?;
        let found = read_db(&library.db_path, |conn| library::song_summaries(conn, &song_ids))?;

        let (ids, start) = kept_start(song_ids, start_index, |id| found.contains_key(id));
        if ids.is_empty() {
            return Err(PlaybackError::NoSongs);
        }

        let mut rng = rand::rng();
        let mut inner = self.lock();
        inner.tracks = found.into_iter().map(|(id, song)| (id, track_of(song))).collect();
        let start = match shuffle {
            ShuffleMode::Keep => start,
            ShuffleMode::On => {
                inner.queue.set_shuffle(true, &mut rng);
                rng.random_range(0..ids.len())
            }
            ShuffleMode::Off => {
                inner.queue.set_shuffle(false, &mut rng);
                start
            }
        };
        inner.queue.replace(ids, start, label, &mut rng);
        let result = self.play_current(&mut inner, &library, 0.0, false);
        self.changed(&mut inner, &library);
        result
    }

    /// Plays a whole album from its first song (Play button on a card, spec
    /// 5.2), in disc and track order, "Playing from" the album name.
    pub fn play_album(&self, album_id: &str) -> Result<(), PlaybackError> {
        let library = self.library()?;
        let (name, ids) = read_db(&library.db_path, |conn| db::home::album_songs(conn, album_id))?;
        if ids.is_empty() {
            return Err(PlaybackError::NoSongs);
        }
        self.play_context(ids, 0, name.unwrap_or_default())
    }

    pub fn next(&self) -> Result<(), PlaybackError> {
        self.step(Queue::next)
    }

    /// Previous button: restarts the song after 3 s, otherwise goes back (spec 5.1.1).
    pub fn previous(&self) -> Result<(), PlaybackError> {
        let played = self.player.position();
        self.step(move |queue| queue.previous(played))
    }

    /// Click on a song of the queue list: only jumps to it (spec 5.1).
    pub fn jump_to(&self, index: usize) -> Result<(), PlaybackError> {
        self.step(move |queue| queue.jump_to(index))
    }

    pub fn set_shuffle(&self, enabled: bool) -> Result<(), PlaybackError> {
        self.change_mode(|queue| queue.set_shuffle(enabled, &mut rand::rng()))
    }

    /// Off → repeat queue → repeat one. Returns the new mode.
    pub fn cycle_repeat(&self) -> Result<RepeatMode, PlaybackError> {
        let mut mode = RepeatMode::Off;
        self.change_mode(|queue| {
            mode = queue.repeat().cycled();
            queue.set_repeat(mode);
        })?;
        Ok(mode)
    }

    /// The song was starred or unstarred: update the queue and the player, so
    /// the UI shows it without reloading.
    pub fn song_starred(&self, song_id: &str, starred: bool) {
        if let Some(track) = self.lock().tracks.get_mut(song_id) {
            track.starred = starred;
        }
        self.player.update_track(song_id, |track| track.starred = starred);
    }

    pub fn queue_state(&self) -> QueueState {
        queue_state(&self.lock())
    }

    /// Stops playback and empties the queue (sign out). The saved queue is
    /// kept: saving the empty one would also clear it on the server.
    pub fn stop(&self) -> Result<(), PlaybackError> {
        let mut inner = self.lock();
        *inner = Inner::default();
        self.player.stop()?;
        self.emit_queue(&inner);
        Ok(())
    }

    /// Restores the saved queue once per library, paused at the saved
    /// position (spec 5.6). Does nothing if something is already queued.
    pub async fn restore(self: Arc<Self>) -> Result<(), PlaybackError> {
        let library = self.library()?;
        {
            let mut inner = self.lock();
            if !inner.queue.is_empty() || inner.restored.as_ref() == Some(&library.db_path) {
                return Ok(());
            }
            inner.restored = Some(library.db_path.clone());
        }

        // Offline (or an old server): the local record is enough.
        let server = match library.client.get_play_queue().await {
            Ok(queue) => queue.map(ServerQueue::from),
            Err(error) => {
                eprintln!("playback: could not read the server queue: {error}");
                None
            }
        };

        tauri::async_runtime::spawn_blocking(move || self.restore_from(&library, server))
            .await
            .map_err(|e| PlaybackError::Task(e.to_string()))?
    }

    fn restore_from(&self, library: &ActiveLibrary, server: Option<ServerQueue>) -> Result<(), PlaybackError> {
        let conn = db::open(&library.db_path)?;
        let local = saved_queue::load(&conn)?;
        if let Some(saved) = &local {
            self.player.set_volume(saved.volume)?;
        }

        // Every song the records mention, to know which still exist.
        let mut ids: Vec<String> = local.iter().flat_map(|l| l.queue.songs.iter().cloned()).collect();
        ids.extend(server.iter().flat_map(|s| s.ids.iter().cloned()));
        let found = library::song_summaries(&conn, &ids)?;
        let exists = |id: &str| found.contains_key(id);

        let restored = match resume::choose(local.as_ref(), server.as_ref()) {
            Source::Local => local.and_then(|saved| resume::from_local(saved, exists)),
            Source::Server => {
                let repeat = local.map_or(RepeatMode::Off, |saved| saved.queue.repeat);
                server.and_then(|server| resume::from_server(server, repeat, exists))
            }
            Source::Nothing => None,
        };
        let Some(restored) = restored else { return Ok(()) };

        let mut inner = self.lock();
        // A song was clicked while the server answered: that one wins.
        if !inner.queue.is_empty() {
            return Ok(());
        }
        inner.tracks = found
            .into_iter()
            .filter(|(id, _)| restored.queue.songs.contains(id))
            .map(|(id, song)| (id, track_of(song)))
            .collect();
        inner.queue = Queue::from_snapshot(restored.queue);
        // Nothing changed yet: no need to save right after resuming.
        inner.last_save = Some(Instant::now());
        let result = self.play_current(&mut inner, library, restored.position, true);
        self.emit_queue(&inner);
        result
    }

    /// Last save, when the app closes. Waits for a save in progress and
    /// gives the server a few seconds at most.
    pub fn save_now(&self) {
        let Ok(library) = self.library() else { return };
        let saved = self.saved_queue(&self.lock());
        if let Some(saved) = saved {
            write_save(&SaveJob { library, saved }, &self.writing, EXIT_SAVE_TIMEOUT);
        }
    }

    /// Runs a queue action and applies the resulting step.
    fn step(&self, action: impl FnOnce(&mut Queue) -> Step) -> Result<(), PlaybackError> {
        let library = self.library()?;
        let mut inner = self.lock();
        let step = action(&mut inner.queue);
        let result = self.apply(&mut inner, &library, step);
        self.changed(&mut inner, &library);
        result
    }

    /// Shuffle and repeat change which song comes next: preload the new one.
    fn change_mode(&self, change: impl FnOnce(&mut Queue)) -> Result<(), PlaybackError> {
        let mut inner = self.lock();
        change(&mut inner.queue);
        let Ok(library) = self.library() else {
            self.emit_queue(&inner);
            return Ok(());
        };
        let result = if self.player.state().track.is_some() {
            self.preload_next(&inner, &library)
        } else {
            Ok(())
        };
        self.changed(&mut inner, &library);
        result
    }

    fn apply(&self, inner: &mut Inner, library: &ActiveLibrary, step: Step) -> Result<(), PlaybackError> {
        match step {
            Step::Play(_) => self.play_current(inner, library, 0.0, false),
            // Spec 5.1.1: past the last song, wait paused on the first one.
            Step::EndOfQueue => self.play_current(inner, library, 0.0, true),
            Step::Restart => {
                self.player.seek(0.0)?;
                self.begin_counting(inner, library, true);
                Ok(())
            }
            Step::Nothing => Ok(()),
        }
    }

    /// Loads the queue's current song `start` seconds in (and preloads the
    /// next one).
    fn play_current(
        &self,
        inner: &mut Inner,
        library: &ActiveLibrary,
        start: f64,
        paused: bool,
    ) -> Result<(), PlaybackError> {
        let Some(id) = inner.queue.current_id() else {
            return Ok(());
        };
        let track = inner.tracks[id].clone();
        let url = library.client.stream_url(id)?;
        let next = self.upcoming(inner, library)?;
        self.player.play(track, &url, next, start, paused)?;
        self.begin_counting(inner, library, !paused);
        Ok(())
    }

    fn preload_next(&self, inner: &Inner, library: &ActiveLibrary) -> Result<(), PlaybackError> {
        self.player.set_next(self.upcoming(inner, library)?)?;
        Ok(())
    }

    /// The song that plays after the current one ends, with a fresh URL.
    fn upcoming(&self, inner: &Inner, library: &ActiveLibrary) -> Result<Option<(Track, SecretUrl)>, PlaybackError> {
        let Some(id) = inner.queue.upcoming().and_then(|p| inner.queue.id_at(p)) else {
            return Ok(None);
        };
        Ok(Some((inner.tracks[id].clone(), library.client.stream_url(id)?)))
    }

    /// A new play of the current song starts: count from zero and tell the
    /// server it is playing (`submission=false`).
    fn begin_counting(&self, inner: &mut Inner, library: &ActiveLibrary, now_playing: bool) {
        inner.listen = ListenTracker::default();
        inner.counting = inner.queue.current_id().map(str::to_string);
        if let (true, Some(id)) = (now_playing, inner.counting.clone()) {
            let client = Arc::clone(&library.client);
            tauri::async_runtime::spawn(async move {
                // Only informative: a failure is not kept for later.
                let _ = client.scrobble(&[(id, now() * 1000)], false).await;
            });
        }
    }

    fn on_player_event(&self, event: PlayerEvent) {
        match event {
            PlayerEvent::State(state) => {
                let playing = state.status == PlaybackStatus::Playing;
                let paused = state.status == PlaybackStatus::Paused;
                let _ = self.app.emit(PLAYER_STATE_EVENT, state);
                // Spec 5.6: save on pause. Only a real pause: a song that
                // loads paused (resume, end of queue) was not playing before.
                let mut inner = self.lock();
                let was_playing = std::mem::replace(&mut inner.playing, playing);
                if was_playing && paused {
                    self.request_save(&mut inner);
                }
            }
            PlayerEvent::Error(error) => {
                let _ = self.app.emit(PLAYER_ERROR_EVENT, error);
            }
            PlayerEvent::Position(position) => {
                let _ = self.app.emit(PLAYER_POSITION_EVENT, position);
                let mut inner = self.lock();
                // Spec 5.6: every 30 s while playing.
                if inner.playing && inner.last_save.is_none_or(|at| at.elapsed() >= SAVE_INTERVAL) {
                    self.request_save(&mut inner);
                }
                if inner.listen.on_position(position.position, position.duration)
                    && let Some(song_id) = inner.counting.clone()
                {
                    // The play started `listened` seconds ago (pauses excluded).
                    let started = now() - inner.listen.listened() as i64;
                    drop(inner);
                    self.record_play(&song_id, started);
                }
            }
            PlayerEvent::Seeked => self.lock().listen.on_seek(),
            PlayerEvent::Advanced(track) => self.on_advanced(&track),
            PlayerEvent::Finished { .. } => {
                if let Err(error) = self.step(Queue::finish_current) {
                    self.report(&error);
                }
            }
        }
    }

    /// mpv moved to the preloaded song without a gap: move the queue too.
    fn on_advanced(&self, track: &Track) {
        let Ok(library) = self.library() else { return };
        let mut inner = self.lock();
        inner.queue.finish_current();
        let result = if inner.queue.current_id() == Some(track.id.as_str()) {
            self.begin_counting(&mut inner, &library, true);
            self.preload_next(&inner, &library)
        } else {
            // The queue changed at that very moment: load what it says now.
            self.play_current(&mut inner, &library, 0.0, false)
        };
        self.changed(&mut inner, &library);
        drop(inner);
        if let Err(error) = result {
            self.report(&error);
        }
    }

    /// Spec 5.7: history + local play count now, scrobble right after
    /// (or later, if the server cannot be reached).
    fn record_play(&self, song_id: &str, started: i64) {
        let Ok(library) = self.library() else { return };
        let recorded = db::open(&library.db_path)
            .and_then(|mut conn| history::record_play(&mut conn, song_id, started));
        if let Err(error) = recorded {
            eprintln!("playback: could not record a play: {error}");
            return;
        }
        tauri::async_runtime::spawn(async move {
            // Sends this play together with any older pending ones.
            let _ = send_pending_scrobbles(&library.client, &library.db_path).await;
        });
    }

    fn report(&self, error: &PlaybackError) {
        let payload = crate::player::PlaybackError { song_id: None, detail: error.to_string() };
        let _ = self.app.emit(PLAYER_ERROR_EVENT, payload);
    }

    fn emit_queue(&self, inner: &Inner) {
        let _ = self.app.emit(QUEUE_CHANGED_EVENT, queue_state(inner));
    }

    /// The queue or the current song changed: tell the UI and save (spec 5.6).
    fn changed(&self, inner: &mut Inner, library: &ActiveLibrary) {
        self.emit_queue(inner);
        self.save(inner, library.clone());
    }

    /// Saves for the active library (pause and timer saves).
    fn request_save(&self, inner: &mut Inner) {
        if let Ok(library) = self.library() {
            self.save(inner, library);
        }
    }

    /// Hands the current queue to the `queue-saver` thread.
    fn save(&self, inner: &mut Inner, library: ActiveLibrary) {
        if let Some(saved) = self.saved_queue(inner) {
            let _ = self.saver.send(SaveJob { library, saved });
            inner.last_save = Some(Instant::now());
        }
    }

    /// What gets saved right now; `None` for an empty queue.
    fn saved_queue(&self, inner: &Inner) -> Option<SavedQueue> {
        if inner.queue.is_empty() {
            return None;
        }
        Some(SavedQueue {
            queue: inner.queue.snapshot(),
            position: self.player.position(),
            volume: self.player.state().volume,
            saved_at_ms: now_ms(),
        })
    }

    fn library(&self) -> Result<ActiveLibrary, PlaybackError> {
        self.app.state::<Arc<AppState>>().active().ok_or(PlaybackError::NotLoggedIn)
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Sends the pending scrobbles (oldest first) and marks them as sent. Runs
/// after each counted play and at every light sync. Returns how many were sent.
pub async fn send_pending_scrobbles(client: &SubsonicClient, db_path: &Path) -> Result<usize, PlaybackError> {
    // Two sends at once could submit the same plays twice.
    static SENDING: AtomicBool = AtomicBool::new(false);
    if SENDING.swap(true, Ordering::SeqCst) {
        return Ok(0);
    }
    struct Release;
    impl Drop for Release {
        fn drop(&mut self) {
            SENDING.store(false, Ordering::SeqCst);
        }
    }
    let _release = Release;

    let path = db_path.to_path_buf();
    let pending = blocking(move || {
        let conn = db::open(&path)?;
        history::pending_scrobbles(&conn, SCROBBLE_BATCH)
    })
    .await?;
    if pending.is_empty() {
        return Ok(0);
    }

    let plays: Vec<(String, i64)> =
        pending.iter().map(|p| (p.song_id.clone(), p.played_at * 1000)).collect();
    client.scrobble(&plays, true).await?;

    let ids: Vec<i64> = pending.iter().map(|p| p.history_id).collect();
    let sent = ids.len();
    let path = db_path.to_path_buf();
    blocking(move || {
        let conn = db::open(&path)?;
        history::mark_scrobbled(&conn, &ids)
    })
    .await?;
    Ok(sent)
}

/// Writes a save locally, then on the server. A server failure only loses
/// the server copy: the local one is already written.
fn write_save(job: &SaveJob, writing: &Mutex<()>, timeout: Duration) {
    let _writing = writing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let saved = &job.saved;
    let local = db::open(&job.library.db_path).and_then(|conn| saved_queue::save(&conn, saved));
    if let Err(error) = local {
        eprintln!("playback: could not save the queue locally: {error}");
    }

    let Some(current) = saved.queue.current_id() else { return };
    let ids: Vec<String> = saved.queue.ids_in_order().map(str::to_string).collect();
    let position_ms = (saved.position * 1000.0) as i64;
    let client = &job.library.client;
    let sent = tauri::async_runtime::block_on(client.save_play_queue(&ids, current, position_ms, timeout));
    if let Err(error) = sent {
        eprintln!("playback: could not save the queue on the server: {error}");
    }
}

fn queue_state(inner: &Inner) -> QueueState {
    QueueState {
        label: inner.queue.label().to_string(),
        continuing_from: inner.queue.continuing_from().map(str::to_string),
        tracks: inner
            .queue
            .ids_in_order()
            .into_iter()
            .filter_map(|id| inner.tracks.get(id).cloned())
            .collect(),
        current_index: inner.queue.current_position(),
        shuffle: inner.queue.shuffle(),
        repeat: inner.queue.repeat(),
    }
}

/// One queue from the songs of several albums (each list in track order).
fn arrange_albums(mut albums: Vec<Vec<String>>, mode: AlbumsPlayMode, rng: &mut impl Rng) -> Vec<String> {
    match mode {
        AlbumsPlayMode::InOrder => albums.concat(),
        AlbumsPlayMode::ShuffledAlbums => {
            albums.shuffle(rng);
            albums.concat()
        }
        AlbumsPlayMode::Mix => {
            let mut songs = albums.concat();
            songs.shuffle(rng);
            songs.truncate(MIX_SIZE);
            songs
        }
    }
}

/// The songs kept (those in the library) and where the clicked one is among
/// them. Counted by position, so a song repeated in a playlist starts at the
/// copy that was clicked; a clicked song left out starts at the next one kept.
fn kept_start(song_ids: Vec<String>, start_index: usize, keep: impl Fn(&str) -> bool) -> (Vec<String>, usize) {
    let mut start = None;
    let mut kept = Vec::with_capacity(song_ids.len());
    for (index, id) in song_ids.into_iter().enumerate() {
        if !keep(&id) {
            continue;
        }
        if start.is_none() && index >= start_index {
            start = Some(kept.len());
        }
        kept.push(id);
    }
    let start = start.unwrap_or(0);
    (kept, start)
}

fn track_of(song: SongSummary) -> Track {
    Track {
        id: song.id,
        title: song.title,
        artist: song.artist,
        album: song.album,
        artist_id: song.artist_id,
        album_id: song.album_id,
        cover_art: song.cover_art,
        duration: song.duration.map(|d| d as f64),
        starred: song.starred,
    }
}

fn read_db<T>(path: &Path, read: impl FnOnce(&rusqlite::Connection) -> DbResult<T>) -> DbResult<T> {
    read(&db::open(path)?)
}

async fn blocking<T, F>(task: F) -> Result<T, PlaybackError>
where
    F: FnOnce() -> DbResult<T> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| PlaybackError::Task(e.to_string()))?
        .map_err(PlaybackError::from)
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn albums() -> Vec<Vec<String>> {
        (0..30).map(|a| (0..10).map(|t| format!("a{a}-t{t}")).collect()).collect()
    }

    #[test]
    fn play_all_follows_the_albums_shown_one_after_the_other() {
        let songs = arrange_albums(albums(), AlbumsPlayMode::InOrder, &mut StdRng::seed_from_u64(1));
        assert_eq!(songs.len(), 300);
        assert_eq!(songs[..2], ["a0-t0", "a0-t1"]);
        assert_eq!(songs[10], "a1-t0");
    }

    #[test]
    fn shuffled_albums_keep_each_album_in_order() {
        let songs = arrange_albums(albums(), AlbumsPlayMode::ShuffledAlbums, &mut StdRng::seed_from_u64(1));
        assert_eq!(songs.len(), 300);
        let order: Vec<&str> = songs.chunks(10).map(|album| album[0].split('-').next().unwrap()).collect();
        assert_ne!(order[..5], ["a0", "a1", "a2", "a3", "a4"], "the albums are shuffled");
        for album in songs.chunks(10) {
            let prefix = album[0].split('-').next().unwrap();
            let expected: Vec<String> = (0..10).map(|t| format!("{prefix}-t{t}")).collect();
            assert_eq!(album, expected, "an album's songs stay together and in order");
        }
    }

    #[test]
    fn a_mix_has_at_most_100_different_songs() {
        let songs = arrange_albums(albums(), AlbumsPlayMode::Mix, &mut StdRng::seed_from_u64(1));
        assert_eq!(songs.len(), MIX_SIZE);
        let unique: std::collections::HashSet<&String> = songs.iter().collect();
        assert_eq!(unique.len(), MIX_SIZE);

        let small = arrange_albums(vec![vec!["x".into(), "y".into()]], AlbumsPlayMode::Mix, &mut StdRng::seed_from_u64(1));
        assert_eq!(small.len(), 2, "fewer songs than the limit: all of them");
    }

    #[test]
    fn album_modes_come_from_the_ui() {
        let parse = |name: &str| serde_json::from_value::<AlbumsPlayMode>(serde_json::json!(name)).ok();
        assert_eq!(parse("inOrder"), Some(AlbumsPlayMode::InOrder));
        assert_eq!(parse("shuffledAlbums"), Some(AlbumsPlayMode::ShuffledAlbums));
        assert_eq!(parse("mix"), Some(AlbumsPlayMode::Mix));
    }

    #[test]
    fn a_repeated_song_starts_at_the_copy_that_was_clicked() {
        let (kept, start) = kept_start(ids(&["a", "b", "a", "c"]), 2, |_| true);
        assert_eq!(kept, ids(&["a", "b", "a", "c"]));
        assert_eq!(start, 2);
    }

    #[test]
    fn songs_missing_from_the_library_are_left_out() {
        let (kept, start) = kept_start(ids(&["a", "gone", "b", "c"]), 2, |id| id != "gone");
        assert_eq!(kept, ids(&["a", "b", "c"]));
        assert_eq!(start, 1, "b is the second song kept");
    }

    #[test]
    fn a_clicked_song_left_out_starts_at_the_next_one() {
        let (kept, start) = kept_start(ids(&["a", "gone", "b"]), 1, |id| id != "gone");
        assert_eq!(kept, ids(&["a", "b"]));
        assert_eq!(start, 1);

        let (_, start) = kept_start(ids(&["a", "gone"]), 1, |id| id != "gone");
        assert_eq!(start, 0, "nothing after it: the first song");
    }
}
