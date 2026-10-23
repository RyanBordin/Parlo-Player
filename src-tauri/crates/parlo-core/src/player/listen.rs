//! Time really listened to a song, for the scrobble (spec 5.7). Pure logic.
//!
//! Only forward movement of the playback position counts. Seeks (reported by
//! mpv) reset the reference, and any jump bigger than `MAX_STEP_SECS` between
//! two positions is ignored too, so skipping ahead never counts as listening.

/// Scrobble once this much was listened, or half the song if that comes first.
pub const SCROBBLE_AFTER_SECS: f64 = 4.0 * 60.0;

/// Positions arrive about 4 times per second; anything much larger is a jump.
const MAX_STEP_SECS: f64 = 2.0;

#[derive(Debug, Clone, Default)]
pub struct ListenTracker {
    listened: f64,
    last_position: Option<f64>,
    submitted: bool,
}

impl ListenTracker {
    /// Seconds listened so far.
    pub fn listened(&self) -> f64 {
        self.listened
    }

    /// A new playback position. Returns `true` exactly once, when the song
    /// becomes eligible for the scrobble.
    pub fn on_position(&mut self, position: f64, duration: Option<f64>) -> bool {
        if let Some(last) = self.last_position {
            let step = position - last;
            if step > 0.0 && step <= MAX_STEP_SECS {
                self.listened += step;
            }
        }
        self.last_position = Some(position);

        if self.submitted || self.listened < threshold(duration) {
            return false;
        }
        self.submitted = true;
        true
    }

    /// The user moved the playback position: the next position starts fresh.
    pub fn on_seek(&mut self) {
        self.last_position = None;
    }
}

/// Half the song or 4 minutes, whichever is smaller. Without a known
/// duration, only the 4 minutes apply.
fn threshold(duration: Option<f64>) -> f64 {
    match duration {
        Some(d) if d > 0.0 => (d / 2.0).min(SCROBBLE_AFTER_SECS),
        _ => SCROBBLE_AFTER_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds positions every 0.25 s from `from` to `to`; returns whether any
    /// of them triggered the scrobble.
    fn play(tracker: &mut ListenTracker, from: f64, to: f64, duration: Option<f64>) -> bool {
        let mut triggered = false;
        let mut position = from;
        while position <= to + 1e-9 {
            triggered |= tracker.on_position(position, duration);
            position += 0.25;
        }
        triggered
    }

    #[test]
    fn continuous_listening_reaches_half_the_song() {
        let mut tracker = ListenTracker::default();
        assert!(!play(&mut tracker, 0.0, 99.0, Some(200.0)));
        assert!(play(&mut tracker, 99.25, 101.0, Some(200.0)));
    }

    #[test]
    fn long_songs_need_four_minutes() {
        let mut tracker = ListenTracker::default();
        let duration = Some(20.0 * 60.0);
        assert!(!play(&mut tracker, 0.0, 239.0, duration));
        assert!(play(&mut tracker, 239.25, 241.0, duration));
    }

    #[test]
    fn seeking_forward_does_not_count() {
        let mut tracker = ListenTracker::default();
        play(&mut tracker, 0.0, 10.0, Some(200.0));
        tracker.on_seek();
        // Jumped to 150 s and listened 10 s there: 20 s in total, not 160 s.
        assert!(!play(&mut tracker, 150.0, 160.0, Some(200.0)));
        assert!((tracker.listened() - 20.0).abs() < 0.01, "{}", tracker.listened());
    }

    #[test]
    fn seeking_back_does_not_subtract_or_double_count() {
        let mut tracker = ListenTracker::default();
        play(&mut tracker, 0.0, 60.0, Some(200.0));
        tracker.on_seek();
        play(&mut tracker, 30.0, 60.0, Some(200.0));
        // Listening to 30 s again counts: real time listened is 90 s.
        assert!((tracker.listened() - 90.0).abs() < 0.01);
    }

    #[test]
    fn a_jump_without_seek_event_is_ignored() {
        let mut tracker = ListenTracker::default();
        play(&mut tracker, 0.0, 10.0, Some(200.0));
        assert!(!tracker.on_position(180.0, Some(200.0)));
        assert!((tracker.listened() - 10.0).abs() < 0.01);
    }

    #[test]
    fn triggers_only_once() {
        let mut tracker = ListenTracker::default();
        assert!(play(&mut tracker, 0.0, 60.0, Some(100.0)));
        assert!(!play(&mut tracker, 60.25, 100.0, Some(100.0)));
    }

    #[test]
    fn unknown_duration_uses_four_minutes() {
        let mut tracker = ListenTracker::default();
        assert!(!play(&mut tracker, 0.0, 200.0, None));
        assert!(play(&mut tracker, 200.25, 241.0, None));
    }

    #[test]
    fn pause_adds_nothing() {
        // While paused mpv sends no positions; resuming continues from the same spot.
        let mut tracker = ListenTracker::default();
        play(&mut tracker, 0.0, 30.0, Some(200.0));
        tracker.on_position(30.0, Some(200.0));
        assert!((tracker.listened() - 30.0).abs() < 0.01);
    }
}
