//! Stand-in engine for Android until the Media3 one exists: the app opens
//! and browses the library, and every playback command fails with a clear message.

use super::audio::Gain;
use super::{AudioDevice, AudioEngine, EngineError, Media, PlaybackStatus, PlayerState, Track};

const MESSAGE: &str = "audio playback is not available on this platform yet";

#[derive(Default)]
pub struct NoEngine;

fn unavailable<T>() -> Result<T, EngineError> {
    Err(EngineError(MESSAGE.into()))
}

impl AudioEngine for NoEngine {
    fn play(&self, _: Track, _: &Media, _: Option<(Track, Media)>, _: f64, _: bool) -> Result<(), EngineError> {
        unavailable()
    }

    fn set_next(&self, _: Option<(Track, Media)>) -> Result<(), EngineError> {
        unavailable()
    }

    fn apply_gain(&self, _: &Gain) -> Result<(), EngineError> {
        unavailable()
    }

    // The output settings are applied at startup: accepted, so nothing is logged as failing.
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
        unavailable()
    }

    fn seek(&self, _: f64) -> Result<(), EngineError> {
        unavailable()
    }

    fn seek_by(&self, _: f64) -> Result<(), EngineError> {
        unavailable()
    }

    fn set_volume(&self, _: f64) -> Result<f64, EngineError> {
        unavailable()
    }

    fn toggle_mute(&self) -> Result<bool, EngineError> {
        unavailable()
    }

    fn update_track(&self, _: &str, _: &dyn Fn(&mut Track)) {}

    fn stop(&self) -> Result<(), EngineError> {
        // Nothing plays, so there is nothing to stop (sign out calls this).
        Ok(())
    }

    fn state(&self) -> PlayerState {
        PlayerState { status: PlaybackStatus::Idle, track: None, volume: 100.0, muted: false }
    }

    fn position(&self) -> f64 {
        0.0
    }
}
