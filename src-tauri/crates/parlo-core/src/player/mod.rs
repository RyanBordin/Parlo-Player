//! Playback logic that does not depend on the audio engine: the queue, the
//! restored queue, listening time for scrobbles, ReplayGain and the player's
//! event types. The engine itself (libmpv on desktop) lives in the app.

pub mod audio;
pub mod events;
pub mod listen;
pub mod queue;
pub mod resume;
