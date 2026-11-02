//! Audio playback on Android with Media3 (spec 12.1).
//!
//! The Kotlin side (`android/`) runs an ExoPlayer inside a `MediaSessionService`,
//! so the music keeps playing with the screen off and shows the system media
//! controls. This crate only forwards commands to it and turns its events
//! into `AudioEvent`s; the queue and the scrobble logic stay in the app.
//!
//! On other platforms the plugin registers nothing: desktop plays with libmpv.

use serde::Deserialize;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "android")]
pub use android::{AudioPlayer, Error, PlayRequest, Song};

/// What the Media3 player is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioState {
    Idle,
    Loading,
    Playing,
    Paused,
    /// The song reached its end.
    Ended,
}

/// An event from the Media3 player. Never carries the stream URL.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AudioEvent {
    State { state: AudioState },
    /// Sent about every 250 ms while playing, and after a seek.
    Position { position_ms: u64, duration_ms: Option<u64> },
    /// The position was moved (from the app or the system media controls).
    Seeked,
    /// ExoPlayer moved on to the queued next song, without a gap.
    Advanced,
    /// ExoPlayer could not play the song (network, format...).
    Error { message: String },
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("parlo-audio")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                _app.manage(android::init(_api)?);
            }
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_from_kotlin_are_read() {
        let parse = |json: &str| serde_json::from_str::<AudioEvent>(json).unwrap();
        assert_eq!(parse(r#"{"kind":"state","state":"playing"}"#), AudioEvent::State { state: AudioState::Playing });
        assert_eq!(
            parse(r#"{"kind":"position","positionMs":1500,"durationMs":null}"#),
            AudioEvent::Position { position_ms: 1500, duration_ms: None }
        );
        assert_eq!(parse(r#"{"kind":"seeked"}"#), AudioEvent::Seeked);
        assert_eq!(parse(r#"{"kind":"advanced"}"#), AudioEvent::Advanced);
        assert_eq!(
            parse(r#"{"kind":"error","message":"Source error"}"#),
            AudioEvent::Error { message: "Source error".into() }
        );
    }
}
