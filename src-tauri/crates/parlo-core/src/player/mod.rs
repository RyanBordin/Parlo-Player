//! Playback logic that does not depend on the audio engine: the queue, the
//! restored queue, listening time for scrobbles, ReplayGain and the player's
//! event types, and the `AudioEngine` trait that each app implements
//! (libmpv on desktop).

pub mod audio;
pub mod engine;
pub mod events;
pub mod listen;
pub mod queue;
pub mod resume;
