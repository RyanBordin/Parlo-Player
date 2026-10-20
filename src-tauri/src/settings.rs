//! App preferences (spec 6.10): language, appearance, audio and streaming;
//! later the system ones. They live in `settings.json` in the app config
//! directory, shared by every account and kept when signing out (the login
//! is in `config.json`, which "Sign out" deletes). Preferences tied to a
//! server (scrobble, sync, visible folders) go in that library's database
//! instead (`db::settings`).
//!
//! The interface owns the meaning of the Home and sidebar lists (their IDs);
//! here they are kept as they come.

use std::fs;
use std::io;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tauri::{LogicalSize, Runtime, WebviewWindow};
use thiserror::Error;

const SETTINGS_FILE: &str = "settings.json";
/// Written first, then renamed over the real file: a crash mid-write never
/// leaves a broken file.
const TEMP_FILE: &str = "settings.json.tmp";

/// Interface scales offered, in percent (spec 6.10).
pub const SCALES: [u16; 4] = [90, 100, 110, 125];
const DEFAULT_SCALE: u16 = 100;
/// Songs of a random mix (spec 6.10, 9).
pub const MIX_SIZES: [u16; 4] = [50, 100, 200, 500];
pub const DEFAULT_MIX_SIZE: u16 = 100;
/// ReplayGain pre-amplification range, in dB.
pub const PREAMP_RANGE: (i8, i8) = (-15, 15);
/// Highest bitrates offered for converted streams, in kbps.
pub const BIT_RATES: [u16; 4] = [320, 256, 192, 128];
const DEFAULT_BIT_RATE: u16 = 192;
/// mpv's name of the system's default audio output.
pub const AUTO_DEVICE: &str = "auto";
/// Cover cache limits offered, in MB (spec 6.10, 12.4).
pub const COVER_CACHE_SIZES: [u16; 4] = [250, 500, 1024, 2048];
const DEFAULT_COVER_CACHE_MB: u16 = 500;
/// Narrowest window at 100% (`minWidth` in tauri.conf.json), in logical pixels.
const BASE_MIN_WIDTH: f64 = 1000.0;

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("settings file error: {0}")]
    Io(#[from] io::Error),

    #[error("settings could not be written: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    /// The operating system's language (src/locale.rs).
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en")]
    En,
}

/// Height of the song lists (tracklists, Home rows, Up Next).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReduceMotion {
    /// The operating system's "reduce motion" setting.
    #[default]
    System,
    On,
    Off,
}

/// ReplayGain (spec 6.10, 12.1). `Auto`: album gain when the queue is one
/// album played in order, track gain otherwise.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayGainMode {
    Off,
    Track,
    Album,
    #[default]
    Auto,
}

/// Original file, or converted by the server (`stream` with `format` and `maxBitRate`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamMode {
    #[default]
    Original,
    Converted,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamFormat {
    #[default]
    Opus,
    Mp3,
}

impl StreamFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            StreamFormat::Opus => "opus",
            StreamFormat::Mp3 => "mp3",
        }
    }
}

/// Streaming quality (spec 6.10): applies from the next song on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StreamQuality {
    #[serde(deserialize_with = "lenient")]
    pub mode: StreamMode,
    #[serde(deserialize_with = "lenient")]
    pub format: StreamFormat,
    /// kbps, one of `BIT_RATES`.
    #[serde(deserialize_with = "lenient")]
    pub max_bit_rate: u16,
}

impl Default for StreamQuality {
    fn default() -> Self {
        Self { mode: StreamMode::default(), format: StreamFormat::default(), max_bit_rate: DEFAULT_BIT_RATE }
    }
}

/// A Home section or sidebar item, in the order the user left them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutItem {
    pub id: String,
    pub visible: bool,
}

/// Every app preference. A field missing from the file, or with a value this
/// version does not know, takes its default; the other fields are kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    #[serde(deserialize_with = "lenient")]
    pub language: Language,
    /// Home sections in order; empty: the default order, all shown.
    #[serde(deserialize_with = "lenient")]
    pub home_sections: Vec<LayoutItem>,
    /// Sidebar items in order (Settings excluded: always last); empty: the default.
    #[serde(deserialize_with = "lenient")]
    pub sidebar_items: Vec<LayoutItem>,
    #[serde(deserialize_with = "lenient")]
    pub density: Density,
    /// Percent, one of `SCALES`.
    #[serde(deserialize_with = "lenient")]
    pub scale: u16,
    #[serde(deserialize_with = "lenient")]
    pub reduce_motion: ReduceMotion,
    #[serde(deserialize_with = "lenient")]
    pub replay_gain: ReplayGainMode,
    /// dB, within `PREAMP_RANGE`.
    #[serde(deserialize_with = "lenient")]
    pub replay_gain_preamp: i8,
    /// Lower the gain so the loudest peak does not clip.
    #[serde(deserialize_with = "lenient")]
    pub avoid_clipping: bool,
    /// mpv's device name (`audio-device-list`), or `AUTO_DEVICE`.
    #[serde(deserialize_with = "lenient")]
    pub audio_device: String,
    /// Windows only: the app has the audio output to itself (WASAPI exclusive).
    #[serde(deserialize_with = "lenient")]
    pub exclusive_audio: bool,
    /// Songs of a random mix, one of `MIX_SIZES`.
    #[serde(deserialize_with = "lenient")]
    pub mix_size: u16,
    #[serde(deserialize_with = "lenient")]
    pub stream: StreamQuality,
    /// MB, one of `COVER_CACHE_SIZES`. The cache is shared by every account.
    #[serde(deserialize_with = "lenient")]
    pub cover_cache_mb: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::default(),
            home_sections: Vec::new(),
            sidebar_items: Vec::new(),
            density: Density::default(),
            scale: DEFAULT_SCALE,
            reduce_motion: ReduceMotion::default(),
            replay_gain: ReplayGainMode::default(),
            replay_gain_preamp: 0,
            avoid_clipping: true,
            audio_device: AUTO_DEVICE.to_string(),
            exclusive_audio: false,
            mix_size: DEFAULT_MIX_SIZE,
            stream: StreamQuality::default(),
            cover_cache_mb: DEFAULT_COVER_CACHE_MB,
        }
    }
}

impl AppSettings {
    /// The same settings with values out of range set to their default (or
    /// the nearest limit, for the pre-amplification).
    pub fn normalized(mut self) -> Self {
        if !SCALES.contains(&self.scale) {
            self.scale = DEFAULT_SCALE;
        }
        self.replay_gain_preamp = self.replay_gain_preamp.clamp(PREAMP_RANGE.0, PREAMP_RANGE.1);
        if self.audio_device.trim().is_empty() {
            self.audio_device = AUTO_DEVICE.to_string();
        }
        if !MIX_SIZES.contains(&self.mix_size) {
            self.mix_size = DEFAULT_MIX_SIZE;
        }
        if !BIT_RATES.contains(&self.stream.max_bit_rate) {
            self.stream.max_bit_rate = DEFAULT_BIT_RATE;
        }
        if !COVER_CACHE_SIZES.contains(&self.cover_cache_mb) {
            self.cover_cache_mb = DEFAULT_COVER_CACHE_MB;
        }
        self
    }

    /// The cover cache limit in bytes.
    pub fn cover_cache_bytes(&self) -> u64 {
        u64::from(self.cover_cache_mb) * 1024 * 1024
    }
}

/// Reads a field; a value of the wrong kind gives the type's default.
pub(crate) fn lenient<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    let value = Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

/// The saved settings; the defaults when there is no file or it is broken.
pub fn load(config_dir: &Path) -> AppSettings {
    let text = match fs::read_to_string(config_dir.join(SETTINGS_FILE)) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return AppSettings::default(),
        Err(error) => {
            log::warn!("settings: could not read the settings file: {error}");
            return AppSettings::default();
        }
    };
    match serde_json::from_str::<AppSettings>(&text) {
        Ok(settings) => settings.normalized(),
        Err(error) => {
            log::warn!("settings: broken settings file, using the defaults: {error}");
            AppSettings::default()
        }
    }
}

pub fn save(config_dir: &Path, settings: &AppSettings) -> Result<(), SettingsError> {
    fs::create_dir_all(config_dir)?;
    let temp = config_dir.join(TEMP_FILE);
    fs::write(&temp, serde_json::to_string_pretty(settings)?)?;
    fs::rename(temp, config_dir.join(SETTINGS_FILE))?;
    Ok(())
}

/// Interface scale (spec 6.10): the window's zoom, and its minimum width
/// grown in proportion, so the layout always has the room it needs.
pub fn apply_scale<R: Runtime>(window: &WebviewWindow<R>, scale: u16) -> tauri::Result<()> {
    let factor = f64::from(scale) / 100.0;
    window.set_zoom(factor)?;

    let mut min_width = BASE_MIN_WIDTH * factor;
    // Never wider than the screen.
    if let Some(monitor) = window.current_monitor()? {
        let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
        min_width = min_width.min(screen.width);
    }
    window.set_min_size(Some(LogicalSize::new(min_width, 0.0)))?;

    let size = window.inner_size()?.to_logical::<f64>(window.scale_factor()?);
    if size.width < min_width {
        window.set_size(LogicalSize::new(min_width, size.height))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("parlo-settings-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn no_file_gives_the_defaults() {
        let settings = load(&temp_dir("none"));
        assert_eq!(settings, AppSettings::default());
        assert_eq!(settings.scale, 100);
        assert_eq!(settings.language, Language::System);
    }

    #[test]
    fn saved_settings_come_back() {
        let dir = temp_dir("round-trip");
        let settings = AppSettings {
            language: Language::En,
            home_sections: vec![LayoutItem { id: "starred-songs".into(), visible: false }],
            density: Density::Compact,
            scale: 125,
            reduce_motion: ReduceMotion::On,
            ..AppSettings::default()
        };
        save(&dir, &settings).unwrap();
        assert_eq!(load(&dir), settings);
        assert!(!dir.join(TEMP_FILE).exists(), "the temporary file is renamed");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_file_uses_the_interface_names() {
        let value = serde_json::to_value(AppSettings { language: Language::PtBr, ..AppSettings::default() }).unwrap();
        assert_eq!(value["language"], "pt-BR");
        assert_eq!(value["reduceMotion"], "system");
        assert_eq!(value["density"], "comfortable");
        assert_eq!(value["homeSections"], json!([]));
    }

    #[test]
    fn a_bad_field_takes_its_default_and_keeps_the_others() {
        let text = json!({ "language": "klingon", "density": "compact", "scale": "big", "future": true }).to_string();
        let settings: AppSettings = serde_json::from_str(&text).unwrap();
        assert_eq!(settings.language, Language::System);
        assert_eq!(settings.density, Density::Compact);
        assert_eq!(settings.normalized().scale, 100);
    }

    #[test]
    fn scales_out_of_the_list_become_100() {
        let settings = AppSettings { scale: 300, ..AppSettings::default() };
        assert_eq!(settings.normalized().scale, 100);
        let settings = AppSettings { scale: 110, ..AppSettings::default() };
        assert_eq!(settings.normalized().scale, 110);
    }

    #[test]
    fn audio_and_streaming_values_are_kept_in_range() {
        let settings = AppSettings {
            replay_gain_preamp: 40,
            audio_device: "  ".into(),
            mix_size: 7,
            stream: StreamQuality { max_bit_rate: 999, ..StreamQuality::default() },
            ..AppSettings::default()
        }
        .normalized();
        assert_eq!(settings.replay_gain_preamp, 15, "the nearest limit");
        assert_eq!(settings.audio_device, AUTO_DEVICE);
        assert_eq!(settings.mix_size, 100);
        assert_eq!(settings.stream.max_bit_rate, 192);

        let low = AppSettings { replay_gain_preamp: -100, ..AppSettings::default() }.normalized();
        assert_eq!(low.replay_gain_preamp, -15);
    }

    #[test]
    fn audio_defaults_follow_the_spec() {
        let settings = AppSettings::default();
        assert_eq!(settings.replay_gain, ReplayGainMode::Auto);
        assert_eq!((settings.replay_gain_preamp, settings.avoid_clipping), (0, true));
        assert_eq!((settings.mix_size, settings.exclusive_audio), (100, false));
        assert_eq!(settings.stream.mode, StreamMode::Original);
        let value = serde_json::to_value(&settings).unwrap();
        assert_eq!(value["stream"], json!({ "mode": "original", "format": "opus", "maxBitRate": 192 }));
        assert_eq!(value["replayGain"], "auto");
    }

    #[test]
    fn a_broken_file_gives_the_defaults() {
        let dir = temp_dir("broken");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(SETTINGS_FILE), "{ not json").unwrap();
        assert_eq!(load(&dir), AppSettings::default());
        fs::remove_dir_all(dir).unwrap();
    }
}
