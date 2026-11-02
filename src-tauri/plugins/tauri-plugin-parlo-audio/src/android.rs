//! The Rust handle to the Kotlin `AudioPlugin`. Each call waits until Kotlin
//! has queued the command on the main thread, not until the player is done:
//! results arrive later as `AudioEvent`s.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::plugin::{PluginApi, PluginHandle};
use tauri::Runtime;

use crate::AudioEvent;

const PLUGIN_IDENTIFIER: &str = "com.ryanbordin.parloplayer.audio";

#[derive(Debug, thiserror::Error)]
#[error("Media3 player: {0}")]
pub struct Error(#[from] tauri::plugin::mobile::PluginInvokeError);

/// A song for the player. `url` has credentials: the Kotlin side never logs it.
#[derive(Debug, Serialize)]
pub struct Song<'a> {
    pub url: &'a str,
    /// Shown by the system media controls.
    pub title: &'a str,
    pub artist: Option<&'a str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayRequest<'a> {
    #[serde(flatten)]
    pub song: Song<'a>,
    pub start_ms: u64,
    pub paused: bool,
    /// Queued after `song`, so ExoPlayer moves to it without a gap.
    pub next: Option<Song<'a>>,
}

pub struct AudioPlayer<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Clone for AudioPlayer<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) fn init<R: Runtime, C: DeserializeOwned>(api: PluginApi<R, C>) -> Result<AudioPlayer<R>, Error> {
    Ok(AudioPlayer(api.register_android_plugin(PLUGIN_IDENTIFIER, "AudioPlugin")?))
}

impl<R: Runtime> AudioPlayer<R> {
    /// Replaces whatever is playing with this song.
    pub fn play(&self, request: &PlayRequest<'_>) -> Result<(), Error> {
        self.run("play", request)
    }

    /// Replaces the song queued after the current one (`None` removes it).
    pub fn set_next(&self, next: Option<&Song<'_>>) -> Result<(), Error> {
        self.run("setNext", json!({ "next": next }))
    }

    pub fn toggle_pause(&self) -> Result<(), Error> {
        self.run("togglePause", json!({}))
    }

    pub fn seek(&self, ms: u64) -> Result<(), Error> {
        self.run("seek", json!({ "ms": ms }))
    }

    pub fn stop(&self) -> Result<(), Error> {
        self.run("stop", json!({}))
    }

    /// 0 to 100.
    pub fn set_volume(&self, volume: f64) -> Result<(), Error> {
        self.run("setVolume", json!({ "volume": volume }))
    }

    /// Sends every player event to `handler`, from a Tauri thread.
    pub fn set_event_handler(&self, handler: impl Fn(AudioEvent) + Send + Sync + 'static) -> Result<(), Error> {
        let channel: Channel = Channel::new(move |body| {
            if let InvokeResponseBody::Json(json) = body {
                match serde_json::from_str::<AudioEvent>(&json) {
                    Ok(event) => handler(event),
                    Err(error) => log::warn!("parlo-audio: unreadable event: {error}"),
                }
            }
            Ok(())
        });
        self.run("setEventHandler", json!({ "channel": channel }))
    }

    fn run(&self, command: &str, payload: impl Serialize) -> Result<(), Error> {
        self.0.run_mobile_plugin::<serde_json::Value>(command, payload)?;
        Ok(())
    }
}
