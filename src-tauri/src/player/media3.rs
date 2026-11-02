//! Audio playback on Android with Media3 (spec 12.1), through the
//! `parlo-audio` plugin.
//!
//! The player runs in Kotlin; this engine keeps the same small state as the
//! mpv one (status, song, position, volume) and turns the plugin's events into
//! `PlayerEvent`s on its own thread, so the queue logic is the same on both.
//!
//! No gapless yet: the next song is only remembered, and loaded when the
//! current one ends (the plugin reports that as `Ended`).

use super::audio::{AudioDevice, Gain};
use super::events::{PlaybackError, PlaybackStatus, PlayerEvent, PlayerState, Position, Track, clamp_volume};
use super::{AudioEngine, EngineError, Media};

use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;

use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_parlo_audio::{AudioEvent, AudioPlayer, AudioState, PlayRequest};

type EventSink = Arc<dyn Fn(PlayerEvent) + Send + Sync>;

/// State shared by the commands and the event thread.
struct Shared {
    status: PlaybackStatus,
    track: Option<Track>,
    /// Loaded when the current song ends.
    next: Option<(Track, Media)>,
    volume: f64,
    muted: bool,
    position: f64,
    duration: Option<f64>,
}

impl Shared {
    fn snapshot(&self) -> PlayerState {
        PlayerState { status: self.status, track: self.track.clone(), volume: self.volume, muted: self.muted }
    }

    fn start(&mut self, track: Track, status: PlaybackStatus, position: f64) {
        self.status = status;
        self.duration = track.duration;
        self.position = position;
        self.track = Some(track);
    }
}

pub struct Media3Engine {
    audio: AudioPlayer<Wry>,
    shared: Arc<Mutex<Shared>>,
    sink: EventSink,
}

impl Media3Engine {
    /// Connects to the plugin and starts the thread that handles its events.
    pub fn new(app: &AppHandle, sink: impl Fn(PlayerEvent) + Send + Sync + 'static) -> Result<Self, EngineError> {
        let audio = app
            .try_state::<AudioPlayer<Wry>>()
            .ok_or_else(|| EngineError("the parlo-audio plugin is not registered".into()))?
            .inner()
            .clone();
        let shared = Arc::new(Mutex::new(Shared {
            status: PlaybackStatus::Idle,
            track: None,
            next: None,
            volume: 100.0,
            muted: false,
            position: 0.0,
            duration: None,
        }));
        let sink: EventSink = Arc::new(sink);

        // The plugin calls back on Android's main thread, which must not wait
        // on another plugin call (loading the next song): hand events over.
        let (tx, rx) = mpsc::channel::<AudioEvent>();
        thread::Builder::new().name("media3-events".into()).spawn({
            let (audio, shared, sink) = (audio.clone(), Arc::clone(&shared), Arc::clone(&sink));
            move || {
                for event in rx {
                    handle(&audio, &shared, &sink, event);
                }
            }
        })?;
        audio
            .set_event_handler(move |event| {
                let _ = tx.send(event);
            })
            .map_err(engine_error)?;

        Ok(Self { audio, shared, sink })
    }

    fn has_track(&self) -> bool {
        lock(&self.shared).track.is_some()
    }
}

/// Loads `track` in the Kotlin player. The URL has credentials: it only goes to the plugin.
fn load(audio: &AudioPlayer<Wry>, track: &Track, media: &Media, start_secs: f64, paused: bool) -> Result<(), EngineError> {
    let request = PlayRequest {
        url: media.url.expose(),
        title: &track.title,
        artist: track.artist.as_deref(),
        start_ms: (start_secs * 1000.0) as u64,
        paused,
    };
    audio.play(&request).map_err(engine_error)
}

fn handle(audio: &AudioPlayer<Wry>, shared: &Mutex<Shared>, sink: &EventSink, event: AudioEvent) {
    let emitted = match event {
        AudioEvent::State { state: AudioState::Ended } => {
            let mut shared = lock(shared);
            match shared.next.take() {
                Some((track, media)) => {
                    shared.start(track.clone(), PlaybackStatus::Loading, 0.0);
                    let state = PlayerEvent::State(shared.snapshot());
                    drop(shared);
                    if let Err(error) = load(audio, &track, &media, 0.0, false) {
                        log::warn!("media3: could not load the next song: {error}");
                    }
                    vec![PlayerEvent::Advanced(track), state]
                }
                None => {
                    shared.status = PlaybackStatus::Idle;
                    let finished = shared.track.as_ref().map(|t| PlayerEvent::Finished { song_id: t.id.clone() });
                    let state = PlayerEvent::State(shared.snapshot());
                    finished.into_iter().chain([state]).collect()
                }
            }
        }

        AudioEvent::State { state } => {
            let status = match state {
                AudioState::Loading => PlaybackStatus::Loading,
                AudioState::Playing => PlaybackStatus::Playing,
                AudioState::Paused => PlaybackStatus::Paused,
                // Stopping is done from here (`stop`), and failures come as `Error`.
                AudioState::Idle | AudioState::Ended => return,
            };
            let mut shared = lock(shared);
            if shared.track.is_none() || shared.status == status {
                return;
            }
            shared.status = status;
            vec![PlayerEvent::State(shared.snapshot())]
        }

        AudioEvent::Position { position_ms, duration_ms } => {
            let mut shared = lock(shared);
            if shared.track.is_none() {
                return;
            }
            shared.position = position_ms as f64 / 1000.0;
            if let Some(ms) = duration_ms {
                shared.duration = Some(ms as f64 / 1000.0);
            }
            vec![PlayerEvent::Position(Position { position: shared.position, duration: shared.duration })]
        }

        AudioEvent::Seeked => vec![PlayerEvent::Seeked],

        AudioEvent::Error { message } => {
            let mut shared = lock(shared);
            if shared.status == PlaybackStatus::Idle {
                return;
            }
            shared.status = PlaybackStatus::Idle;
            let song_id = shared.track.as_ref().map(|t| t.id.clone());
            vec![
                PlayerEvent::Error(PlaybackError { song_id, detail: message }),
                PlayerEvent::State(shared.snapshot()),
            ]
        }
    };

    // Emit outside the lock.
    for event in emitted {
        sink(event);
    }
}

impl AudioEngine for Media3Engine {
    fn play(
        &self,
        track: Track,
        media: &Media,
        next: Option<(Track, Media)>,
        start_secs: f64,
        paused: bool,
    ) -> Result<(), EngineError> {
        let start_secs = if start_secs.is_finite() { start_secs.max(0.0) } else { 0.0 };
        let state = {
            let mut shared = lock(&self.shared);
            shared.start(track.clone(), PlaybackStatus::Loading, start_secs);
            shared.next = next;
            shared.snapshot()
        };
        (self.sink)(PlayerEvent::State(state));
        load(&self.audio, &track, media, start_secs, paused)
    }

    fn set_next(&self, next: Option<(Track, Media)>) -> Result<(), EngineError> {
        lock(&self.shared).next = next;
        Ok(())
    }

    // ReplayGain, output device and exclusive mode come later on Android.
    fn apply_gain(&self, _: &Gain) -> Result<(), EngineError> {
        Ok(())
    }

    fn set_audio_device(&self, _: &str) -> Result<(), EngineError> {
        Ok(())
    }

    fn set_exclusive(&self, _: bool) -> Result<(), EngineError> {
        Ok(())
    }

    fn audio_devices(&self) -> Vec<AudioDevice> {
        Vec::new()
    }

    fn toggle_pause(&self) -> Result<(), EngineError> {
        if !self.has_track() {
            return Ok(());
        }
        self.audio.toggle_pause().map_err(engine_error)
    }

    fn seek(&self, seconds: f64) -> Result<(), EngineError> {
        if !self.has_track() || !seconds.is_finite() {
            return Ok(());
        }
        self.audio.seek((seconds.max(0.0) * 1000.0) as u64).map_err(engine_error)
    }

    fn seek_by(&self, seconds: f64) -> Result<(), EngineError> {
        if !seconds.is_finite() {
            return Ok(());
        }
        let position = self.position();
        self.seek(position + seconds)
    }

    fn set_volume(&self, volume: f64) -> Result<f64, EngineError> {
        let volume = clamp_volume(volume);
        self.audio.set_volume(volume).map_err(engine_error)?;
        let state = {
            let mut shared = lock(&self.shared);
            shared.volume = volume;
            shared.muted = false;
            shared.snapshot()
        };
        (self.sink)(PlayerEvent::State(state));
        Ok(volume)
    }

    /// Media3 has no mute: muting sets the player's volume to 0 and keeps ours.
    fn toggle_mute(&self) -> Result<bool, EngineError> {
        let (muted, volume) = {
            let shared = lock(&self.shared);
            (!shared.muted, shared.volume)
        };
        self.audio.set_volume(if muted { 0.0 } else { volume }).map_err(engine_error)?;
        let state = {
            let mut shared = lock(&self.shared);
            shared.muted = muted;
            shared.snapshot()
        };
        (self.sink)(PlayerEvent::State(state));
        Ok(muted)
    }

    fn update_track(&self, song_id: &str, change: &dyn Fn(&mut Track)) {
        let state = {
            let mut shared = lock(&self.shared);
            if let Some((next, _)) = shared.next.as_mut().filter(|(t, _)| t.id == song_id) {
                change(next);
            }
            match shared.track.as_mut().filter(|t| t.id == song_id) {
                Some(track) => {
                    change(track);
                    Some(shared.snapshot())
                }
                None => None,
            }
        };
        if let Some(state) = state {
            (self.sink)(PlayerEvent::State(state));
        }
    }

    fn stop(&self) -> Result<(), EngineError> {
        let state = {
            let mut shared = lock(&self.shared);
            shared.status = PlaybackStatus::Idle;
            shared.track = None;
            shared.next = None;
            shared.duration = None;
            shared.position = 0.0;
            shared.snapshot()
        };
        self.audio.stop().map_err(engine_error)?;
        (self.sink)(PlayerEvent::State(state));
        Ok(())
    }

    fn state(&self) -> PlayerState {
        lock(&self.shared).snapshot()
    }

    fn position(&self) -> f64 {
        lock(&self.shared).position
    }
}

fn engine_error(error: tauri_plugin_parlo_audio::Error) -> EngineError {
    EngineError(error.to_string())
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
