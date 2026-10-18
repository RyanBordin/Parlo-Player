//! Restoring the queue when the app opens (spec 5.6). Pure logic: no mpv,
//! no network, no database.
//!
//! Two records exist: the local one (complete) and the server's
//! `getPlayQueue` (songs, current song and position only). The local one wins
//! unless another app changed the server queue after the last local save.

use super::queue::{QueueSnapshot, RepeatMode};
use crate::subsonic::PlayQueue;

/// The queue as saved locally.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedQueue {
    pub queue: QueueSnapshot,
    /// Seconds into the current song.
    pub position: f64,
    /// 0–100.
    pub volume: f64,
    /// Unix milliseconds, local clock.
    pub saved_at_ms: i64,
}

/// The queue as the server has it.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerQueue {
    /// Songs in playback order.
    pub ids: Vec<String>,
    pub current: Option<String>,
    pub position_ms: i64,
    /// Unix milliseconds, server clock. `None` if missing or unreadable.
    pub changed_ms: Option<i64>,
    pub changed_by: String,
}

impl From<PlayQueue> for ServerQueue {
    fn from(queue: PlayQueue) -> Self {
        Self {
            ids: queue.entry.into_iter().map(|e| e.id).collect(),
            current: queue.current,
            position_ms: queue.position.unwrap_or(0),
            changed_ms: queue.changed.as_deref().and_then(parse_timestamp_ms),
            changed_by: queue.changed_by.unwrap_or_default(),
        }
    }
}

impl ServerQueue {
    /// Same songs in the same order and same current song: this is the copy
    /// of our own last save. Compared by content, not by date, so a server
    /// clock running ahead does not make our own save look newer.
    fn matches(&self, local: &SavedQueue) -> bool {
        self.current.as_deref() == local.queue.current_id()
            && self.ids.iter().map(String::as_str).eq(local.queue.ids_in_order())
    }
}

/// Which record to restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Local,
    Server,
    Nothing,
}

/// The rule of spec 5.6: the server queue wins only when it differs from the
/// local one and was changed after the last local save.
pub fn choose(local: Option<&SavedQueue>, server: Option<&ServerQueue>) -> Source {
    match (local, server) {
        (None, None) => Source::Nothing,
        (Some(_), None) => Source::Local,
        (None, Some(_)) => Source::Server,
        (Some(local), Some(server)) => {
            let newer = server.changed_ms.is_some_and(|changed| changed > local.saved_at_ms);
            if !server.matches(local) && newer { Source::Server } else { Source::Local }
        }
    }
}

/// A queue ready to load, paused, at `position` seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Restored {
    pub queue: QueueSnapshot,
    pub position: f64,
}

/// The local record, without the songs that left the library.
pub fn from_local(saved: SavedQueue, exists: impl Fn(&str) -> bool) -> Option<Restored> {
    retain_existing(saved.queue, saved.position, exists)
}

/// The server record as a queue "Continuing from [app]". Its playback order
/// becomes the original order (the API does not keep the other one), so
/// shuffle starts off; repeat is this computer's setting.
pub fn from_server(server: ServerQueue, repeat: RepeatMode, exists: impl Fn(&str) -> bool) -> Option<Restored> {
    let current = server
        .current
        .as_deref()
        .and_then(|id| server.ids.iter().position(|x| x == id))
        .unwrap_or(0);
    let queue = QueueSnapshot {
        order: (0..server.ids.len()).collect(),
        songs: server.ids,
        current,
        shuffle: false,
        repeat,
        label: String::new(),
        continuing_from: Some(server.changed_by),
    };
    retain_existing(queue, server.position_ms as f64 / 1000.0, exists)
}

/// Drops the songs that no longer exist. If the current song is gone, the
/// next one that exists becomes current, from its start (or the first song,
/// if none is left after it). `None` when no song is left.
fn retain_existing(queue: QueueSnapshot, position: f64, exists: impl Fn(&str) -> bool) -> Option<Restored> {
    // New index of each song, `None` for the removed ones.
    let mut kept = 0;
    let new_index: Vec<Option<usize>> = queue
        .songs
        .iter()
        .map(|id| {
            exists(id).then(|| {
                kept += 1;
                kept - 1
            })
        })
        .collect();
    if kept == 0 {
        return None;
    }

    let old_order: Vec<Option<usize>> = queue
        .order
        .iter()
        .map(|&i| new_index.get(i).copied().flatten())
        .collect();
    // Position in the new order of the first song still there, from `from` on.
    let new_position = |from: usize| {
        old_order.get(from..)?.iter().position(Option::is_some).map(|offset| {
            old_order[..from + offset].iter().filter(|i| i.is_some()).count()
        })
    };
    let (current, position) = match old_order.get(queue.current) {
        Some(Some(_)) => (new_position(queue.current)?, position.max(0.0)),
        _ => (new_position(queue.current + 1).unwrap_or(0), 0.0),
    };

    let songs = queue.songs.into_iter().zip(&new_index).filter_map(|(id, i)| i.map(|_| id)).collect();
    let order = old_order.into_iter().flatten().collect();
    Some(Restored { queue: QueueSnapshot { songs, order, current, ..queue }, position })
}

/// Reads an ISO 8601 / RFC 3339 date-time (`2026-10-18T12:34:56.789Z`,
/// `...-03:00`) as Unix milliseconds. Only the first 3 fraction digits count.
pub fn parse_timestamp_ms(text: &str) -> Option<i64> {
    let (date, time) = text.trim().split_once(['T', 't', ' '])?;

    let mut parts = date.splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;

    let (clock, offset_secs) = match time.strip_suffix(['Z', 'z']) {
        Some(clock) => (clock, 0),
        None => {
            let at = time.rfind(['+', '-'])?;
            let (clock, zone) = time.split_at(at);
            let (hours, minutes) = zone[1..].split_once(':')?;
            let secs = hours.parse::<i64>().ok()? * 3600 + minutes.parse::<i64>().ok()? * 60;
            (clock, if zone.starts_with('-') { -secs } else { secs })
        }
    };

    let (hms, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    let mut parts = hms.splitn(3, ':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let second: i64 = parts.next()?.parse().ok()?;
    if !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let millis: i64 = format!("{fraction:0<3}")[..3].parse().ok()?;

    let valid = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second <= 60;
    if !valid {
        return None;
    }

    let secs = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    Some((secs - offset_secs) * 1000 + millis)
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsonic::PlayQueueEntry;

    const SAVED_AT: i64 = 1_792_326_896_000;

    fn ids(n: usize) -> Vec<String> {
        (1..=n).map(|i| format!("s{i}")).collect()
    }

    /// Local record: songs s1..sn in order, current at `current`.
    fn local(n: usize, current: usize) -> SavedQueue {
        SavedQueue {
            queue: QueueSnapshot {
                songs: ids(n),
                order: (0..n).collect(),
                current,
                shuffle: false,
                repeat: RepeatMode::All,
                label: "Album".into(),
                continuing_from: None,
            },
            position: 42.0,
            volume: 70.0,
            saved_at_ms: SAVED_AT,
        }
    }

    fn server(ids: Vec<String>, current: &str, changed_ms: Option<i64>) -> ServerQueue {
        ServerQueue {
            ids,
            current: Some(current.into()),
            position_ms: 12_500,
            changed_ms,
            changed_by: "Symfonium".into(),
        }
    }

    fn all_exist(_: &str) -> bool {
        true
    }

    // --- Which queue wins ---

    #[test]
    fn nothing_saved_restores_nothing() {
        assert_eq!(choose(None, None), Source::Nothing);
    }

    #[test]
    fn only_local_or_server_offline_uses_local() {
        assert_eq!(choose(Some(&local(3, 0)), None), Source::Local);
    }

    #[test]
    fn only_server_uses_server() {
        let server = server(ids(3), "s1", Some(SAVED_AT));
        assert_eq!(choose(None, Some(&server)), Source::Server);
    }

    #[test]
    fn our_own_save_on_the_server_keeps_the_complete_local_state() {
        let local = local(3, 1);
        let same = server(ids(3), "s2", Some(SAVED_AT + 500));
        assert_eq!(choose(Some(&local), Some(&same)), Source::Local);
    }

    #[test]
    fn a_server_clock_running_ahead_does_not_beat_our_own_save() {
        let local = local(3, 1);
        let same = server(ids(3), "s2", Some(SAVED_AT + 10 * 60 * 1000));
        assert_eq!(choose(Some(&local), Some(&same)), Source::Local);
    }

    #[test]
    fn another_app_changing_the_queue_later_wins() {
        let local = local(3, 1);
        let other = server(vec!["x1".into(), "x2".into()], "x2", Some(SAVED_AT + 1));
        assert_eq!(choose(Some(&local), Some(&other)), Source::Server);

        // Same songs, but the other app moved to another song.
        let moved = server(ids(3), "s3", Some(SAVED_AT + 1));
        assert_eq!(choose(Some(&local), Some(&moved)), Source::Server);
    }

    #[test]
    fn an_older_server_queue_loses() {
        let local = local(3, 1);
        let older = server(vec!["x1".into()], "x1", Some(SAVED_AT - 1));
        assert_eq!(choose(Some(&local), Some(&older)), Source::Local);
    }

    #[test]
    fn an_unreadable_server_date_keeps_local() {
        let local = local(3, 1);
        let other = server(vec!["x1".into()], "x1", None);
        assert_eq!(choose(Some(&local), Some(&other)), Source::Local);
    }

    #[test]
    fn server_queue_is_built_from_the_api_fields() {
        let api = PlayQueue {
            entry: vec![PlayQueueEntry { id: "a".into() }, PlayQueueEntry { id: "b".into() }],
            current: Some("b".into()),
            position: Some(1500),
            changed: Some("2026-10-18T12:34:56.789Z".into()),
            changed_by: Some("Feishin".into()),
        };
        let server = ServerQueue::from(api);
        assert_eq!(server.ids, ["a", "b"]);
        assert_eq!(server.changed_ms, Some(1_792_326_896_789));
        assert_eq!(server.changed_by, "Feishin");
    }

    // --- Restoring with missing songs ---

    #[test]
    fn local_restore_keeps_everything_when_all_songs_exist() {
        let restored = from_local(local(5, 2), all_exist).unwrap();
        assert_eq!(restored.queue, local(5, 2).queue);
        assert_eq!(restored.position, 42.0);
    }

    #[test]
    fn missing_songs_are_left_out_and_the_current_one_stays() {
        // s2 and s4 are gone; the current song is s5.
        let restored = from_local(local(6, 4), |id| id != "s2" && id != "s4").unwrap();
        assert_eq!(restored.queue.songs, ["s1", "s3", "s5", "s6"]);
        assert_eq!(restored.queue.order, [0, 1, 2, 3]);
        assert_eq!(restored.queue.current_id(), Some("s5"));
        assert_eq!(restored.position, 42.0, "same song, same position");
    }

    #[test]
    fn a_missing_current_song_moves_to_the_next_one_from_the_start() {
        let restored = from_local(local(4, 1), |id| id != "s2").unwrap();
        assert_eq!(restored.queue.current_id(), Some("s3"));
        assert_eq!(restored.position, 0.0);
    }

    #[test]
    fn a_missing_last_song_goes_back_to_the_first() {
        let restored = from_local(local(3, 2), |id| id != "s3").unwrap();
        assert_eq!(restored.queue.current_id(), Some("s1"));
        assert_eq!(restored.position, 0.0);
    }

    #[test]
    fn no_song_left_restores_nothing() {
        assert_eq!(from_local(local(3, 0), |_| false), None);
    }

    #[test]
    fn a_shuffled_queue_keeps_its_order_without_the_missing_songs() {
        let mut saved = local(5, 1);
        // Playback order s4, s2, s5, s1, s3; current s2 (position 1).
        saved.queue.order = vec![3, 1, 4, 0, 2];
        saved.queue.shuffle = true;

        let restored = from_local(saved, |id| id != "s4" && id != "s1").unwrap();
        let in_order: Vec<&str> = restored.queue.ids_in_order().collect();
        assert_eq!(in_order, ["s2", "s5", "s3"]);
        assert_eq!(restored.queue.songs, ["s2", "s3", "s5"], "original order kept");
        assert_eq!(restored.queue.current_id(), Some("s2"));
        assert!(restored.queue.shuffle);
    }

    #[test]
    fn server_restore_continues_from_the_other_app() {
        let server = server(vec!["a".into(), "b".into(), "c".into()], "b", Some(SAVED_AT));
        let restored = from_server(server, RepeatMode::One, |id| id != "a").unwrap();
        assert_eq!(restored.queue.ids_in_order().collect::<Vec<_>>(), ["b", "c"]);
        assert_eq!(restored.queue.current_id(), Some("b"));
        assert_eq!(restored.position, 12.5);
        assert!(!restored.queue.shuffle);
        assert_eq!(restored.queue.repeat, RepeatMode::One);
        assert_eq!(restored.queue.continuing_from.as_deref(), Some("Symfonium"));
    }

    #[test]
    fn server_restore_with_an_unknown_current_song_starts_at_the_first() {
        let server = server(ids(3), "gone", Some(SAVED_AT));
        let restored = from_server(server, RepeatMode::Off, all_exist).unwrap();
        assert_eq!(restored.queue.current_id(), Some("s1"));
    }

    // --- Dates ---

    #[test]
    fn parses_utc_dates_with_milliseconds() {
        assert_eq!(parse_timestamp_ms("2026-10-18T12:34:56.789Z"), Some(1_792_326_896_789));
        assert_eq!(parse_timestamp_ms("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_timestamp_ms("1969-12-31T23:59:59Z"), Some(-1000));
    }

    #[test]
    fn parses_offsets_and_long_fractions() {
        assert_eq!(parse_timestamp_ms("2026-10-18T09:34:56-03:00"), Some(1_792_326_896_000));
        // Go writes up to 9 fraction digits.
        assert_eq!(parse_timestamp_ms("2026-10-18T12:34:56.789123456Z"), Some(1_792_326_896_789));
        assert_eq!(parse_timestamp_ms("2026-10-18T12:34:56.5Z"), Some(1_792_326_896_500));
    }

    #[test]
    fn rejects_text_that_is_not_a_date() {
        for text in ["", "yesterday", "2026-10-18", "2026-13-01T00:00:00Z", "2026-10-18T12:34:56.x1Z"] {
            assert_eq!(parse_timestamp_ms(text), None, "{text}");
        }
    }
}
