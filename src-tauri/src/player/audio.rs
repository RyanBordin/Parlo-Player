//! Audio choices of a song (spec 6.10, 12.1): its ReplayGain and the output
//! device. Pure logic, no mpv, so it is unit tested.
//!
//! ReplayGain uses the server's values (OpenSubsonic `replayGain`) when the
//! song has them: they work for converted streams too, where the file's tags
//! may be lost. The app adds the pre-amplification, limits the gain by the
//! peak to avoid clipping, and hands the result to mpv as `volume-gain`.
//! Without server values, mpv reads the file's own tags.

use serde::{Deserialize, Serialize};

use crate::settings::{AUTO_DEVICE, ReplayGainMode};

/// mpv's `volume-gain` limits, in dB.
const MIN_GAIN: f64 = -96.0;
const MAX_GAIN: f64 = 12.0;

/// A song's ReplayGain values from the server: gains in dB, linear peaks (1.0 = full scale).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReplayGainData {
    pub track_gain: Option<f64>,
    pub album_gain: Option<f64>,
    pub track_peak: Option<f64>,
    pub album_peak: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainKind {
    Track,
    Album,
}

impl GainKind {
    fn as_str(self) -> &'static str {
        match self {
            GainKind::Track => "track",
            GainKind::Album => "album",
        }
    }
}

/// How the volume of a song is set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Gain {
    /// ReplayGain off.
    Off,
    /// The server's value with the pre-amplification and the clipping limit, in dB.
    Fixed(f64),
    /// No server value: mpv reads the file's tags.
    Tags { kind: GainKind, preamp: f64, avoid_clipping: bool },
}

impl Gain {
    /// mpv options: per file when loading it, or as properties for the song
    /// playing. Every case sets `volume-gain`, so nothing is left from the
    /// previous song.
    pub fn options(&self) -> Vec<(&'static str, String)> {
        match *self {
            Gain::Off => vec![("replaygain", "no".into()), ("volume-gain", "0".into())],
            Gain::Fixed(db) => vec![("replaygain", "no".into()), ("volume-gain", format!("{db:.2}"))],
            Gain::Tags { kind, preamp, avoid_clipping } => vec![
                ("replaygain", kind.as_str().into()),
                ("replaygain-preamp", format!("{preamp:.2}")),
                // mpv: "allow the gain to clip"; avoiding clipping is "no".
                ("replaygain-clip", if avoid_clipping { "no" } else { "yes" }.into()),
                ("volume-gain", "0".into()),
            ],
        }
    }
}

/// The gain of a song. `album_in_order`: the queue is one album played in
/// order (what `Auto` picks the album gain for).
pub fn gain_for(
    mode: ReplayGainMode,
    preamp: i8,
    avoid_clipping: bool,
    album_in_order: bool,
    data: Option<&ReplayGainData>,
) -> Gain {
    let kind = match mode {
        ReplayGainMode::Off => return Gain::Off,
        ReplayGainMode::Track => GainKind::Track,
        ReplayGainMode::Album => GainKind::Album,
        ReplayGainMode::Auto if album_in_order => GainKind::Album,
        ReplayGainMode::Auto => GainKind::Track,
    };
    let preamp = f64::from(preamp);
    let tags = Gain::Tags { kind, preamp, avoid_clipping };
    let Some(data) = data else { return tags };

    // The kind asked for, else the other one.
    let track = (data.track_gain, data.track_peak);
    let album = (data.album_gain, data.album_peak);
    let (first, second) = match kind {
        GainKind::Track => (track, album),
        GainKind::Album => (album, track),
    };
    let (gain, peak) = if first.0.is_some() { first } else { second };
    let Some(gain) = gain else { return tags };

    let mut total = gain + preamp;
    if avoid_clipping && let Some(peak) = peak.filter(|p| *p > 0.0) {
        // The loudest sample, raised by `total` dB, must stay at or below 1.0.
        total = total.min(-20.0 * peak.log10());
    }
    Gain::Fixed(total.clamp(MIN_GAIN, MAX_GAIN))
}

/// An audio output, as mpv lists it (`audio-device-list`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
}

/// mpv's `audio-device-list` (JSON text).
pub fn parse_devices(json: &str) -> Vec<AudioDevice> {
    serde_json::from_str(json).unwrap_or_default()
}

/// The output to use: the one chosen while it exists, else the system's
/// default (spec 6.10). The choice stays saved, so it comes back with the device.
pub fn device_to_use<'a>(wanted: &'a str, available: &[AudioDevice]) -> &'a str {
    if wanted == AUTO_DEVICE || available.iter().any(|device| device.name == wanted) {
        wanted
    } else {
        AUTO_DEVICE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(track: Option<(f64, f64)>, album: Option<(f64, f64)>) -> ReplayGainData {
        ReplayGainData {
            track_gain: track.map(|t| t.0),
            track_peak: track.map(|t| t.1),
            album_gain: album.map(|a| a.0),
            album_peak: album.map(|a| a.1),
        }
    }

    fn fixed(gain: Gain) -> f64 {
        match gain {
            Gain::Fixed(db) => (db * 100.0).round() / 100.0,
            other => panic!("expected a fixed gain, got {other:?}"),
        }
    }

    #[test]
    fn off_sets_no_gain() {
        let values = data(Some((-6.0, 0.9)), None);
        assert_eq!(gain_for(ReplayGainMode::Off, 5, true, false, Some(&values)), Gain::Off);
    }

    #[test]
    fn auto_picks_the_album_gain_only_for_an_album_in_order() {
        let values = data(Some((-6.0, 0.5)), Some((-8.0, 0.5)));
        assert_eq!(fixed(gain_for(ReplayGainMode::Auto, 0, true, true, Some(&values))), -8.0);
        assert_eq!(fixed(gain_for(ReplayGainMode::Auto, 0, true, false, Some(&values))), -6.0);
        assert_eq!(fixed(gain_for(ReplayGainMode::Album, 0, true, false, Some(&values))), -8.0);
    }

    #[test]
    fn a_missing_kind_uses_the_other_one() {
        let only_track = data(Some((-6.0, 0.5)), None);
        assert_eq!(fixed(gain_for(ReplayGainMode::Album, 0, true, true, Some(&only_track))), -6.0);
    }

    #[test]
    fn the_preamp_adds_and_the_peak_limits_it() {
        // Peak 0.5 is -6.02 dB below full scale: at most +6.02 dB.
        let quiet = data(Some((4.0, 0.5)), None);
        assert_eq!(fixed(gain_for(ReplayGainMode::Track, 0, true, false, Some(&quiet))), 4.0);
        assert_eq!(fixed(gain_for(ReplayGainMode::Track, 5, true, false, Some(&quiet))), 6.02);
        assert_eq!(fixed(gain_for(ReplayGainMode::Track, 5, false, false, Some(&quiet))), 9.0, "clipping allowed");
    }

    #[test]
    fn the_gain_stays_within_mpv_limits() {
        let silent = data(Some((30.0, 0.0)), None);
        assert_eq!(fixed(gain_for(ReplayGainMode::Track, 15, true, false, Some(&silent))), 12.0);
    }

    #[test]
    fn without_server_values_mpv_reads_the_tags() {
        let expected = Gain::Tags { kind: GainKind::Album, preamp: -3.0, avoid_clipping: true };
        assert_eq!(gain_for(ReplayGainMode::Album, -3, true, false, None), expected);
        // Navidrome sends an empty `replayGain` for files without the tags.
        assert_eq!(gain_for(ReplayGainMode::Album, -3, true, false, Some(&ReplayGainData::default())), expected);
    }

    #[test]
    fn options_always_reset_the_volume_gain() {
        let options = |gain: Gain| gain.options().into_iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>();
        assert_eq!(options(Gain::Off), ["replaygain=no", "volume-gain=0"]);
        assert_eq!(options(Gain::Fixed(-6.5)), ["replaygain=no", "volume-gain=-6.50"]);
        assert_eq!(
            options(Gain::Tags { kind: GainKind::Track, preamp: 2.0, avoid_clipping: true }),
            ["replaygain=track", "replaygain-preamp=2.00", "replaygain-clip=no", "volume-gain=0"]
        );
    }

    #[test]
    fn a_missing_device_falls_back_to_the_system_default() {
        let devices = parse_devices(
            r#"[{"name":"auto","description":"Autoselect device"},{"name":"wasapi/{1}","description":"Headset"}]"#,
        );
        assert_eq!(devices.len(), 2);
        assert_eq!(device_to_use("wasapi/{1}", &devices), "wasapi/{1}");
        assert_eq!(device_to_use("wasapi/{gone}", &devices), "auto");
        assert_eq!(device_to_use("auto", &[]), "auto");
        assert!(parse_devices("not json").is_empty());
    }
}
