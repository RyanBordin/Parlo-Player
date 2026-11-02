//! Audio playback (spec 12.1). The engine-independent parts (queue, resume,
//! listen time, ReplayGain, events) live in parlo-core, shared with Android.
//! The engine is libmpv on desktop and Media3 on Android.

pub use parlo_core::player::engine::{AudioEngine, EngineError, Media};
pub use parlo_core::player::{audio, events, listen, queue, resume};

pub use audio::AudioDevice;
pub use events::{PlaybackError, PlaybackStatus, PlayerEvent, PlayerState, Track};

#[cfg(desktop)]
mod mpv;
#[cfg(desktop)]
pub use mpv::Player;

#[cfg(target_os = "android")]
mod media3;
#[cfg(target_os = "android")]
pub use media3::Media3Engine;
