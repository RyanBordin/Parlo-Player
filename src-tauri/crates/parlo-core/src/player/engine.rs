//! The audio engine as the rest of the app sees it. Desktop plays through
//! libmpv; Android will play through Media3. The playback controller only
//! talks to this trait, so the queue and scrobble logic stay the same on both.

use thiserror::Error;

use super::audio::{AudioDevice, Gain};
use super::events::{PlayerState, Track};
use crate::subsonic::SecretUrl;

/// What the engine loads for a song: its stream and how loud it plays.
#[derive(Debug, Clone)]
pub struct Media {
    pub url: SecretUrl,
    pub gain: Gain,
}

/// An engine failure, already described in words for the logs and the interface.
#[derive(Debug, Error)]
#[error("{0}")]
pub struct EngineError(pub String);

impl From<std::io::Error> for EngineError {
    fn from(error: std::io::Error) -> Self {
        EngineError(format!("could not start the player thread: {error}"))
    }
}

/// Plays one song at a time, with the next one preloaded for gapless playback.
/// Changes arrive as `PlayerEvent`s through the callback the engine was built with.
pub trait AudioEngine: Send + Sync {
    /// Replaces whatever is playing with `track`, and preloads `next`.
    /// The song starts `start_secs` in; with `paused`, it loads but waits there.
    fn play(
        &self,
        track: Track,
        media: &Media,
        next: Option<(Track, Media)>,
        start_secs: f64,
        paused: bool,
    ) -> Result<(), EngineError>;

    /// Replaces the preloaded song (after shuffle, repeat or queue changes).
    fn set_next(&self, next: Option<(Track, Media)>) -> Result<(), EngineError>;

    /// Changes the gain of the song playing, at once (a ReplayGain setting changed).
    fn apply_gain(&self, gain: &Gain) -> Result<(), EngineError>;

    /// Sends the sound to another output without stopping the music.
    fn set_audio_device(&self, name: &str) -> Result<(), EngineError>;

    /// Exclusive output mode (WASAPI on Windows); engines without it ignore it.
    fn set_exclusive(&self, exclusive: bool) -> Result<(), EngineError>;

    fn audio_devices(&self) -> Vec<AudioDevice>;

    /// Play/pause. Does nothing when no song is loaded.
    fn toggle_pause(&self) -> Result<(), EngineError>;

    /// Jumps to `seconds` from the start of the song.
    fn seek(&self, seconds: f64) -> Result<(), EngineError>;

    /// Jumps `seconds` forward (or back, when negative).
    fn seek_by(&self, seconds: f64) -> Result<(), EngineError>;

    /// Sets the volume (0 to 100) and unmutes; returns the volume applied.
    fn set_volume(&self, volume: f64) -> Result<f64, EngineError>;

    /// Returns whether the player is now muted.
    fn toggle_mute(&self) -> Result<bool, EngineError>;

    /// Changes the details of a loaded song (current or next), e.g. its star.
    fn update_track(&self, song_id: &str, change: &dyn Fn(&mut Track));

    /// Stops playback and forgets the current song (e.g. on sign out).
    fn stop(&self) -> Result<(), EngineError>;

    fn state(&self) -> PlayerState;

    /// Seconds into the current song.
    fn position(&self) -> f64;
}
