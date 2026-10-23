//! Audio playback (spec 12.1). The engine-independent parts (queue, resume,
//! listen time, ReplayGain, events) live in parlo-core, shared with Android.
//! The engine is libmpv on desktop; Android has none yet (see `unavailable`).

pub use parlo_core::player::engine::{AudioEngine, EngineError, Media};
pub use parlo_core::player::{audio, events, listen, queue, resume};

pub use audio::AudioDevice;
pub use events::{PlaybackError, PlaybackStatus, PlayerEvent, PlayerState, Track};

#[cfg(desktop)]
mod mpv;
#[cfg(desktop)]
pub use mpv::Player;

#[cfg(mobile)]
mod unavailable;
#[cfg(mobile)]
pub use unavailable::NoEngine;
