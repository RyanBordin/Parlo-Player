//! Audio playback with libmpv (spec 12.1).
//!
//! libmpv's client API is thread-safe: the commands (`play`, `toggle_pause`,
//! `seek`, `set_volume`) call mpv directly from the caller's thread, while a
//! dedicated thread runs mpv's event loop and turns its events into
//! `PlayerEvent`s. This module knows nothing about Tauri: events go to a
//! callback, so it can be tested on its own.

mod events;

pub use events::{
    Ended, PlaybackError, PlaybackStatus, PlayerEvent, PlayerState, Position, Track,
};

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
];

const POSITION_INTERVAL: Duration = Duration::from_millis(250);

// IDs for observed properties.
const OBSERVE_TIME_POS: u64 = 1;
const OBSERVE_DURATION: u64 = 2;
const OBSERVE_PAUSE: u64 = 3;

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
    volume: f64,
    duration: Option<f64>,
    throttle: PositionThrottle,
}

impl Shared {
    fn snapshot(&self) -> PlayerState {
        PlayerState { status: self.status, track: self.track.clone(), volume: self.volume }
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
        let volume: f64 = mpv.get_property("volume")?;

        let mpv = Arc::new(mpv);
        let shared = Arc::new(Mutex::new(Shared {
            status: PlaybackStatus::Idle,
            track: None,
            volume,
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

    /// Replaces whatever is playing with `track`, streamed from `url`.
    pub fn play(&self, track: Track, url: &SecretUrl) -> Result<(), PlayerError> {
        let title = track.title.clone();
        let state = {
            let mut shared = lock(&self.shared);
            shared.status = PlaybackStatus::Loading;
            shared.duration = track.duration;
            shared.track = Some(track);
            shared.throttle.reset();
            shared.snapshot()
        };
        (self.sink)(PlayerEvent::State(state));

        // Without this, mpv's own title would be the URL (and its credentials).
        self.mpv.set_property("force-media-title", title.as_str())?;
        self.mpv.set_property("pause", false)?;
        self.mpv.command("loadfile", &[url.expose(), "replace"])?;
        Ok(())
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
            shared.duration = None;
            shared.snapshot()
        };
        self.mpv.command("stop", &[])?;
        (self.sink)(PlayerEvent::State(state));
        Ok(())
    }

    pub fn state(&self) -> PlayerState {
        lock(&self.shared).snapshot()
    }

    fn has_track(&self) -> bool {
        lock(&self.shared).track.is_some()
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
                if shared.throttle.should_emit(Instant::now()) {
                    let duration = shared.duration;
                    vec![PlayerEvent::Position(Position { position: p.max(0.0), duration })]
                } else {
                    vec![]
                }
            }

            // After a seek, show the new position right away.
            Ok(Event::PlaybackRestart) => {
                lock(shared).throttle.reset();
                vec![]
            }

            // Stop/redirect happen when a new song replaces the current one.
            Ok(Event::EndFile(reason)) if reason == mpv_end_file_reason::Eof => {
                let mut shared = lock(shared);
                shared.status = PlaybackStatus::Idle;
                let ended = shared.track.as_ref().map(|t| Ended { song_id: t.id.clone() });
                let state = PlayerEvent::State(shared.snapshot());
                ended.map(PlayerEvent::Ended).into_iter().chain([state]).collect()
            }

            // mpv reports a failed file (network, format...) as an error here.
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

    /// Plays a 1-second synthetic tone (no network, no sound card) and checks
    /// the whole event flow. Needs the libmpv DLL, like the rest of the app.
    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn plays_a_synthetic_tone_to_the_end() {
        let (tx, rx) = mpsc::channel();
        let player = Player::new(&[("ao", "null")], move |event| {
            let _ = tx.send(event);
        })
        .expect("libmpv starts");

        let url = SecretUrl::for_test("av://lavfi:sine=frequency=440:duration=1");
        let track = Track {
            id: "tone".into(),
            title: "Tone".into(),
            artist: None,
            album: None,
            duration: None,
        };
        player.play(track, &url).unwrap();

        let mut saw_playing = false;
        let mut last_position = None;
        let mut saw_duration = false;
        loop {
            let event = rx.recv_timeout(Duration::from_secs(10)).expect("event before timeout");
            match event {
                PlayerEvent::State(s) if s.status == PlaybackStatus::Playing => saw_playing = true,
                PlayerEvent::Position(p) => {
                    last_position = Some(p.position);
                    saw_duration |= p.duration.is_some_and(|d| (d - 1.0).abs() < 0.1);
                }
                PlayerEvent::Ended(e) => {
                    assert_eq!(e.song_id, "tone");
                    break;
                }
                PlayerEvent::Error(e) => panic!("playback error: {}", e.detail),
                _ => {}
            }
        }
        assert!(saw_playing, "status should become playing");
        assert!(saw_duration, "duration should be about 1 s");
        assert!(last_position.is_some(), "positions should be reported");
        assert_eq!(player.state().status, PlaybackStatus::Idle);
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn missing_file_reports_an_error() {
        let (tx, rx) = mpsc::channel();
        let player = Player::new(&[("ao", "null")], move |event| {
            let _ = tx.send(event);
        })
        .unwrap();

        let track = Track { id: "x".into(), title: "X".into(), artist: None, album: None, duration: None };
        player.play(track, &SecretUrl::for_test("file:///definitely/missing.flac")).unwrap();

        loop {
            match rx.recv_timeout(Duration::from_secs(10)).expect("event before timeout") {
                PlayerEvent::Error(e) => {
                    assert_eq!(e.song_id.as_deref(), Some("x"));
                    assert!(!e.detail.is_empty());
                    break;
                }
                PlayerEvent::Ended(_) => panic!("a missing file must not count as played"),
                _ => {}
            }
        }
    }

    #[test]
    #[ignore = "starts libmpv; run with --ignored"]
    fn volume_is_clamped_and_reported() {
        let player = Player::new(&[("ao", "null")], |_| {}).unwrap();
        assert_eq!(player.set_volume(150.0).unwrap(), 100.0);
        assert_eq!(player.set_volume(30.0).unwrap(), 30.0);
        assert_eq!(player.state().volume, 30.0);
    }
}
