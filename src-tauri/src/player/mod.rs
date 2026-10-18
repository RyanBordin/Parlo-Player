//! Audio playback with libmpv (spec 12.1).
//!
//! libmpv's client API is thread-safe: the commands (`play`, `toggle_pause`,
//! `seek`, ...) call mpv directly from the caller's thread, while a dedicated
//! thread runs mpv's event loop and turns its events into `PlayerEvent`s. This
//! module knows nothing about Tauri or the queue: events go to a callback.
//!
//! Gapless: mpv's own playlist always holds the current song and, when there
//! is one, the next song of the queue. With `prefetch-playlist`, mpv opens the
//! next song before the current one ends and moves to it without silence.

mod events;
pub mod listen;
pub mod queue;
pub mod resume;

pub use events::{PlaybackError, PlaybackStatus, PlayerEvent, PlayerState, Position, Track};

use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use libmpv2::events::{Event, PropertyData};
use libmpv2::{Format, Mpv, mpv_end_file_reason, mpv_error};
use thiserror::Error;

use crate::subsonic::SecretUrl;
use events::{PositionThrottle, clamp_volume};

/// Options for an audio-only player without window, terminal output or user config.
const BASE_OPTIONS: &[(&str, &str)] = &[
    // Embedded cover art would otherwise open a video window.
    ("vid", "no"),
    ("audio-display", "no"),
    // mpv prints "Playing: <url>" to the terminal; the URL has credentials.
    ("terminal", "no"),
    // Ignore any mpv.conf / input.conf the user has installed.
    ("config", "no"),
    ("input-default-bindings", "no"),
    ("ytdl", "no"),
    // Stay alive with nothing loaded, waiting for the next song.
    ("idle", "yes"),
    ("volume-max", "100"),
    // Gapless: open the next playlist entry early and switch without a gap
    // when both songs have the same audio format (`weak`, mpv's default).
    ("prefetch-playlist", "yes"),
    ("gapless-audio", "weak"),
];

const POSITION_INTERVAL: Duration = Duration::from_millis(250);

// IDs for observed properties.
const OBSERVE_TIME_POS: u64 = 1;
const OBSERVE_DURATION: u64 = 2;
const OBSERVE_PAUSE: u64 = 3;
const OBSERVE_PLAYLIST_POS: u64 = 4;

#[derive(Debug, Error)]
pub enum PlayerError {
    #[error("mpv error: {}", describe_mpv_error(.0))]
    Mpv(#[from] libmpv2::Error),

    #[error("could not start the player thread: {0}")]
    Thread(#[from] std::io::Error),
}

type EventSink = Arc<dyn Fn(PlayerEvent) + Send + Sync>;

/// State shared by the commands and the event thread.
struct Shared {
    status: PlaybackStatus,
    track: Option<Track>,
    /// Preloaded song, second entry of mpv's playlist.
    next: Option<Track>,
    volume: f64,
    position: f64,
    duration: Option<f64>,
    throttle: PositionThrottle,
}

impl Shared {
    fn snapshot(&self) -> PlayerState {
        PlayerState { status: self.status, track: self.track.clone(), volume: self.volume }
    }

    fn start(&mut self, track: Track, status: PlaybackStatus) {
        self.status = status;
        self.duration = track.duration;
        self.position = 0.0;
        self.track = Some(track);
        self.throttle.reset();
    }
}

pub struct Player {
    mpv: Arc<Mpv>,
    shared: Arc<Mutex<Shared>>,
    sink: EventSink,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    /// Starts mpv and its event thread. `extra_options` override the defaults
    /// (tests use `ao=null` to play without a sound card).
    pub fn new(
        extra_options: &[(&str, &str)],
        sink: impl Fn(PlayerEvent) + Send + Sync + 'static,
    ) -> Result<Self, PlayerError> {
        let mpv = Mpv::with_initializer(|init| {
            for (name, value) in BASE_OPTIONS.iter().chain(extra_options) {
                init.set_property(name, *value)?;
            }
            Ok(())
        })?;
        mpv.observe_property("time-pos", Format::Double, OBSERVE_TIME_POS)?;
        mpv.observe_property("duration", Format::Double, OBSERVE_DURATION)?;
        mpv.observe_property("pause", Format::Flag, OBSERVE_PAUSE)?;
        mpv.observe_property("playlist-pos", Format::Int64, OBSERVE_PLAYLIST_POS)?;
        let volume: f64 = mpv.get_property("volume")?;

        let mpv = Arc::new(mpv);
        let shared = Arc::new(Mutex::new(Shared {
            status: PlaybackStatus::Idle,
            track: None,
            next: None,
            volume,
            position: 0.0,
            duration: None,
            throttle: PositionThrottle::new(POSITION_INTERVAL),
        }));
        let sink: EventSink = Arc::new(sink);

        let thread = thread::Builder::new().name("mpv-events".into()).spawn({
            let (mpv, shared, sink) = (Arc::clone(&mpv), Arc::clone(&shared), Arc::clone(&sink));
            move || event_loop(&mpv, &shared, &sink)
        })?;

        Ok(Self { mpv, shared, sink, thread: Some(thread) })
    }

    /// Replaces whatever is playing with `track`, and preloads `next`.
    /// The song starts `start_secs` in; with `paused`, it loads but waits there.
    pub fn play(
        &self,
        track: Track,
        url: &SecretUrl,
        next: Option<(Track, SecretUrl)>,
        start_secs: f64,
        paused: bool,
    ) -> Result<(), PlayerError> {
        let title = track.title.clone();
        let start_secs = if start_secs.is_finite() { start_secs.max(0.0) } else { 0.0 };
        let state = {
            let mut shared = lock(&self.shared);
            shared.start(track, PlaybackStatus::Loading);
            shared.position = start_secs;
            shared.next = None;
            shared.snapshot()
        };
        (self.sink)(PlayerEvent::State(state));

        self.mpv.set_property("pause", paused)?;
        self.load(url, &title, "replace", start_secs)?;
        self.set_next(next)
    }

    /// Replaces the preloaded song (after shuffle, repeat or queue changes).
    pub fn set_next(&self, next: Option<(Track, SecretUrl)>) -> Result<(), PlayerError> {
        // `playlist-clear` removes every entry except the current one.
        self.mpv.command("playlist-clear", &[])?;
        let Some((track, url)) = next else {
            lock(&self.shared).next = None;
            return Ok(());
        };
        let title = track.title.clone();
        lock(&self.shared).next = Some(track);
        self.load(&url, &title, "append", 0.0)
    }

    /// Play/pause. Does nothing when no song is loaded.
    pub fn toggle_pause(&self) -> Result<(), PlayerError> {
        if !self.has_track() {
            return Ok(());
        }
        self.mpv.command("cycle", &["pause"])?;
        Ok(())
    }

    /// Jumps to `seconds` from the start of the song.
    pub fn seek(&self, seconds: f64) -> Result<(), PlayerError> {
        if !self.has_track() || !seconds.is_finite() {
            return Ok(());
        }
        let target = format!("{:.3}", seconds.max(0.0));
        self.mpv.command("seek", &[target.as_str(), "absolute"])?;
        Ok(())
    }

    /// Sets the volume (0–100) and returns the value applied.
    pub fn set_volume(&self, volume: f64) -> Result<f64, PlayerError> {
        let volume = clamp_volume(volume);
        self.mpv.set_property("volume", volume)?;
        let state = {
            let mut shared = lock(&self.shared);
            shared.volume = volume;
            shared.snapshot()
        };
        (self.sink)(PlayerEvent::State(state));
        Ok(volume)
    }

    /// Stops playback and forgets the current song (e.g. on sign out).
    pub fn stop(&self) -> Result<(), PlayerError> {
        let state = {
            let mut shared = lock(&self.shared);
            shared.status = PlaybackStatus::Idle;
            shared.track = None;
            shared.next = None;
            shared.duration = None;
            shared.position = 0.0;
            shared.snapshot()
        };
        self.mpv.command("stop", &[])?;
        (self.sink)(PlayerEvent::State(state));
        Ok(())
    }

    pub fn state(&self) -> PlayerState {
        lock(&self.shared).snapshot()
    }

    /// Seconds played of the current song.
    pub fn position(&self) -> f64 {
        lock(&self.shared).position
    }

    fn has_track(&self) -> bool {
        lock(&self.shared).track.is_some()
    }

    /// `loadfile <url> <mode> -1 force-media-title=...[,start=...]`. Without
    /// the title, mpv's own title would be the URL (and its credentials).
    fn load(&self, url: &SecretUrl, title: &str, mode: &str, start_secs: f64) -> Result<(), PlayerError> {
        // `%N%text` quotes the value, so commas in the title are safe.
        let mut options = format!("force-media-title=%{}%{title}", title.len());
        if start_secs > 0.0 {
            options.push_str(&format!(",start={start_secs:.3}"));
        }
        self.mpv.command("loadfile", &[url.expose(), mode, "-1", options.as_str()])?;
        Ok(())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        // `quit` makes mpv send Shutdown, which ends the event thread.
        let _ = self.mpv.command("quit", &[]);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn event_loop(mpv: &Mpv, shared: &Mutex<Shared>, sink: &EventSink) {
    loop {
        // `None` means timeout or a property that became unavailable.
        let Some(event) = mpv.wait_event(1.0) else {
            continue;
        };

        let emitted = match event {
            Ok(Event::Shutdown) => break,

            Ok(Event::FileLoaded) => {
                let paused = mpv.get_property::<bool>("pause").unwrap_or(false);
                let mut shared = lock(shared);
                shared.status = if paused { PlaybackStatus::Paused } else { PlaybackStatus::Playing };
                vec![PlayerEvent::State(shared.snapshot())]
            }

            // Position 1 only happens when mpv moved on to the preloaded song.
            Ok(Event::PropertyChange {
                name: "playlist-pos",
                change: PropertyData::Int64(1),
                ..
            }) => {
                // Keep mpv's playlist at "current + next": drop the finished song.
                let _ = mpv.command("playlist-remove", &["0"]);
                let mut shared = lock(shared);
                match shared.next.take() {
                    Some(track) => {
                        let status = shared.status;
                        shared.start(track.clone(), status);
                        vec![PlayerEvent::Advanced(track), PlayerEvent::State(shared.snapshot())]
                    }
                    None => vec![],
                }
            }

            Ok(Event::PropertyChange { name: "pause", change: PropertyData::Flag(paused), .. }) => {
                let mut shared = lock(shared);
                match shared.status {
                    PlaybackStatus::Playing | PlaybackStatus::Paused => {
                        shared.status =
                            if paused { PlaybackStatus::Paused } else { PlaybackStatus::Playing };
                        vec![PlayerEvent::State(shared.snapshot())]
                    }
                    _ => vec![],
                }
            }

            Ok(Event::PropertyChange { name: "duration", change: PropertyData::Double(d), .. }) => {
                lock(shared).duration = Some(d);
                vec![]
            }

            Ok(Event::PropertyChange { name: "time-pos", change: PropertyData::Double(p), .. }) => {
                let mut shared = lock(shared);
                shared.position = p.max(0.0);
                if shared.throttle.should_emit(Instant::now()) {
                    let (position, duration) = (shared.position, shared.duration);
                    vec![PlayerEvent::Position(Position { position, duration })]
                } else {
                    vec![]
                }
            }

            Ok(Event::Seek) => vec![PlayerEvent::Seeked],

            // After a seek, show the new position right away.
            Ok(Event::PlaybackRestart) => {
                lock(shared).throttle.reset();
                vec![]
            }

            // With a preloaded song, mpv continues on its own (see playlist-pos).
            // Stop/redirect happen when a new song replaces the current one.
            Ok(Event::EndFile(reason)) if reason == mpv_end_file_reason::Eof => {
                let mut shared = lock(shared);
                if shared.next.is_some() {
                    vec![]
                } else {
                    shared.status = PlaybackStatus::Idle;
                    let finished = shared
                        .track
                        .as_ref()
                        .map(|t| PlayerEvent::Finished { song_id: t.id.clone() });
                    let state = PlayerEvent::State(shared.snapshot());
                    finished.into_iter().chain([state]).collect()
                }
            }

            // mpv reports a failed file (network, format...) as an error here.
            // If a song was preloaded, mpv moves on to it by itself.
            Err(error) => {
                let mut shared = lock(shared);
                if shared.status == PlaybackStatus::Idle {
                    vec![]
                } else {
                    shared.status = PlaybackStatus::Idle;
                    let song_id = shared.track.as_ref().map(|t| t.id.clone());
                    vec![
                        PlayerEvent::Error(PlaybackError { song_id, detail: describe_mpv_error(&error) }),
                        PlayerEvent::State(shared.snapshot()),
                    ]
                }
            }

            Ok(_) => vec![],
        };

        // Emit outside the lock.
        for event in emitted {
            sink(event);
        }
    }
}

/// Short English description of an mpv error code. Never includes the URL.
fn describe_mpv_error(error: &libmpv2::Error) -> String {
    let libmpv2::Error::Raw(code) = error else {
        return format!("{error}");
    };
    let text = match *code {
        mpv_error::LoadingFailed => "loading failed",
        mpv_error::UnknownFormat => "unrecognized file format",
        mpv_error::NothingToPlay => "no audio stream",
        mpv_error::AoInitFailed => "audio output could not be opened",
        mpv_error::PropertyUnavailable => "property unavailable",
        mpv_error::InvalidParameter => "invalid parameter",
        mpv_error::Command => "command failed",
        _ => return format!("mpv error {code}"),
    };
    text.to_string()
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn track(id: &str, title: &str) -> Track {
        Track { id: id.into(), title: title.into(), artist: None, album: None, duration: None }
    }

    /// A synthetic tone: no network and, with `ao=null`, no sound card.
    fn tone(frequency: u32) -> SecretUrl {
        SecretUrl::for_test(&format!("av://lavfi:sine=frequency={frequency}:duration=1"))
    }

    fn test_player() -> (Player, mpsc::Receiver<PlayerEvent>) {
        let (tx, rx) = mpsc::channel();
        let player = Player::new(&[("ao", "null")], move |event| {
            let _ = tx.send(event);
        })
        .expect("libmpv starts");
        (player, rx)
    }

    fn next_event(rx: &mpsc::Receiver<PlayerEvent>) -> PlayerEvent {
        rx.recv_timeout(Duration::from_secs(10)).expect("event before timeout")
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn plays_a_synthetic_tone_to_the_end() {
        let (player, rx) = test_player();
        player.play(track("tone", "Tone"), &tone(440), None, 0.0, false).unwrap();

        let (mut saw_playing, mut saw_position, mut saw_duration) = (false, false, false);
        loop {
            match next_event(&rx) {
                PlayerEvent::State(s) if s.status == PlaybackStatus::Playing => saw_playing = true,
                PlayerEvent::Position(p) => {
                    saw_position = true;
                    saw_duration |= p.duration.is_some_and(|d| (d - 1.0).abs() < 0.1);
                }
                PlayerEvent::Finished { song_id } => {
                    assert_eq!(song_id, "tone");
                    break;
                }
                PlayerEvent::Error(e) => panic!("playback error: {}", e.detail),
                _ => {}
            }
        }
        assert!(saw_playing && saw_position && saw_duration);
        assert_eq!(player.state().status, PlaybackStatus::Idle);
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn moves_to_the_preloaded_song_by_itself() {
        let (player, rx) = test_player();
        // Commas in the title must not break the per-file options.
        let next = Some((track("b", "Second, with comma"), tone(660)));
        player.play(track("a", "First"), &tone(440), next, 0.0, false).unwrap();

        let (mut advanced, mut title) = (None, String::new());
        loop {
            match next_event(&rx) {
                PlayerEvent::Advanced(t) => {
                    advanced = Some(t.id);
                    // Read while the second song plays (it is empty once idle).
                    title = player.mpv.get_property("media-title").unwrap_or_default();
                }
                PlayerEvent::Finished { song_id } => {
                    assert_eq!(song_id, "b", "only the last song finishes");
                    break;
                }
                PlayerEvent::Error(e) => panic!("playback error: {}", e.detail),
                _ => {}
            }
        }
        assert_eq!(advanced.as_deref(), Some("b"));
        assert_eq!(title, "Second, with comma");
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn loads_paused_when_asked() {
        let (player, rx) = test_player();
        player.play(track("a", "A"), &tone(440), None, 0.0, true).unwrap();
        loop {
            if let PlayerEvent::State(s) = next_event(&rx) {
                if s.status == PlaybackStatus::Paused {
                    break;
                }
                assert_ne!(s.status, PlaybackStatus::Playing);
            }
        }
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn loads_paused_at_a_saved_position() {
        let (player, rx) = test_player();
        // A 1 s tone, resumed at 0.6 s (spec 5.6: restored songs wait paused).
        player.play(track("a", "A"), &tone(440), None, 0.6, true).unwrap();
        loop {
            if let PlayerEvent::State(s) = next_event(&rx)
                && s.status == PlaybackStatus::Paused
            {
                break;
            }
        }
        let position: f64 = player.mpv.get_property("time-pos").unwrap();
        assert!((position - 0.6).abs() < 0.1, "time-pos {position}");
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn missing_file_reports_an_error() {
        let (player, rx) = test_player();
        let url = SecretUrl::for_test("file:///definitely/missing.flac");
        player.play(track("x", "X"), &url, None, 0.0, false).unwrap();

        loop {
            match next_event(&rx) {
                PlayerEvent::Error(e) => {
                    assert_eq!(e.song_id.as_deref(), Some("x"));
                    assert!(!e.detail.is_empty());
                    break;
                }
                PlayerEvent::Finished { .. } => panic!("a missing file must not count as played"),
                _ => {}
            }
        }
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn volume_is_clamped_and_reported() {
        let (player, _rx) = test_player();
        assert_eq!(player.set_volume(150.0).unwrap(), 100.0);
        assert_eq!(player.set_volume(30.0).unwrap(), 30.0);
        assert_eq!(player.state().volume, 30.0);
    }
}
