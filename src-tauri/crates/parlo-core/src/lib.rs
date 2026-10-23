//! Parlo Player logic that does not depend on Tauri or on the audio engine,
//! shared by the desktop app and the Android app: the Subsonic client, the
//! local SQLite cache, the play queue and the app settings.

pub mod db;
pub mod player;
pub mod settings;
pub mod subsonic;
pub mod time;
