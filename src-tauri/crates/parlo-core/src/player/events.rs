//! Player event types and pure helpers (no mpv needed, so they are unit tested).

use std::time::{Duration, Instant};

use serde::Serialize;

use super::audio::{AudioDevice, ReplayGainData};

/// Song being played, with what the UI shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artist_id: Option<String>,
    pub album_id: Option<String>,
    /// ID for the `cover` protocol (spec 12.4).
    pub cover_art: Option<String>,
    /// Seconds, from the library cache (mpv reports the exact value later).
    pub duration: Option<f64>,
    pub starred: bool,
    /// The server's ReplayGain values (spec 12.1); the UI does not need them.
    #[serde(skip)]
    pub replay_gain: Option<ReplayGainData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Idle,
    Loading,
    Playing,
    Paused,
}

/// Payload of `player-state`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: PlaybackStatus,
    pub track: Option<Track>,
    /// 0–100. Kept while muted.
    pub volume: f64,
    pub muted: bool,
}

/// Payload of `player-position`. Seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub position: f64,
    pub duration: Option<f64>,
}

/// Payload of `player-error`. `detail` comes from mpv's error code and never
/// contains the stream URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackError {
    pub song_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    /// Sent to the UI as `player-state`.
    State(PlayerState),
    /// Sent to the UI as `player-position`.
    Position(Position),
    /// Sent to the UI as `player-error`.
    Error(PlaybackError),
    /// mpv moved to the preloaded next song, without a gap.
    Advanced(Track),
    /// The song ended and nothing was preloaded after it.
    Finished { song_id: String },
    /// The playback position was moved (seek), so it must not count as listened.
    Seeked,
    /// The audio outputs changed (a device was plugged in or removed).
    DevicesChanged(Vec<AudioDevice>),
}

/// Limits `player-position` events: mpv reports the position many times per
/// second, the UI only needs a few.
#[derive(Debug)]
pub struct PositionThrottle {
    interval: Duration,
    last: Option<Instant>,
}

impl PositionThrottle {
    pub fn new(interval: Duration) -> Self {
        Self { interval, last: None }
    }

    pub fn should_emit(&mut self, now: Instant) -> bool {
        let due = self.last.is_none_or(|last| now.duration_since(last) >= self.interval);
        if due {
            self.last = Some(now);
        }
        due
    }

    /// Lets the next position through right away (new song, after a seek).
    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// Volume accepted by the player: 0–100.
pub fn clamp_volume(volume: f64) -> f64 {
    if volume.is_nan() { 100.0 } else { volume.clamp(0.0, 100.0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn throttle_lets_through_a_few_positions_per_second() {
        let start = Instant::now();
        let mut throttle = PositionThrottle::new(Duration::from_millis(250));
        assert!(throttle.should_emit(start));
        assert!(!throttle.should_emit(start + Duration::from_millis(100)));
        assert!(throttle.should_emit(start + Duration::from_millis(260)));
    }

    #[test]
    fn throttle_reset_lets_the_next_position_through() {
        let start = Instant::now();
        let mut throttle = PositionThrottle::new(Duration::from_millis(250));
        assert!(throttle.should_emit(start));
        throttle.reset();
        assert!(throttle.should_emit(start + Duration::from_millis(10)));
    }

    #[test]
    fn volume_is_clamped() {
        assert_eq!(clamp_volume(150.0), 100.0);
        assert_eq!(clamp_volume(-5.0), 0.0);
        assert_eq!(clamp_volume(42.5), 42.5);
        assert_eq!(clamp_volume(f64::NAN), 100.0);
    }

    #[test]
    fn events_serialize_for_the_ui() {
        let state = PlayerState {
            status: PlaybackStatus::Playing,
            track: Some(Track {
                id: "s1".into(),
                title: "Song".into(),
                artist: Some("Artist".into()),
                cover_art: Some("al-1".into()),
                duration: Some(215.0),
                ..Track::default()
            }),
            volume: 80.0,
            muted: false,
        };
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            json!({
                "status": "playing",
                "track": {
                    "id": "s1", "title": "Song", "artist": "Artist", "album": null,
                    "artistId": null, "albumId": null, "coverArt": "al-1",
                    "duration": 215.0, "starred": false
                },
                "volume": 80.0,
                "muted": false
            })
        );
        assert_eq!(
            serde_json::to_value(PlaybackError { song_id: Some("s1".into()), detail: "x".into() })
                .unwrap(),
            json!({ "songId": "s1", "detail": "x" })
        );
    }
}
