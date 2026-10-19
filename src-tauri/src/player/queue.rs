//! Play queue (spec 5.1, 5.1.1, 5.4 and 5.6). Pure logic: no mpv, no network.
//!
//! The queue keeps the context in its original order (`songs`) and a playback
//! order (`order`, indexes into `songs`). Shuffle only rearranges `order`, so
//! turning it off restores the original order.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

/// With more than this many seconds played, Previous restarts the song.
pub const RESTART_THRESHOLD_SECS: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepeatMode {
    #[default]
    Off,
    /// Repeat the whole queue.
    All,
    /// Repeat the current song when it ends by itself.
    One,
}

impl RepeatMode {
    /// Button cycle: Off → All → One → Off (spec 4.3).
    pub fn cycled(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }

    /// Name stored in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            RepeatMode::Off => "off",
            RepeatMode::All => "all",
            RepeatMode::One => "one",
        }
    }

    /// Unknown names read as `Off`.
    pub fn from_name(name: &str) -> Self {
        match name {
            "all" => RepeatMode::All,
            "one" => RepeatMode::One,
            _ => RepeatMode::Off,
        }
    }
}

/// What removing a song did to the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removed {
    /// No song at that position.
    Nothing,
    /// Another song than the current one: the current one goes on.
    Other,
    /// The current song: the one that took its place is current now.
    Current,
    /// The current song, and it was the last: the first one is current now
    /// and must wait paused (end of queue, spec 5.1.1).
    CurrentAtEnd,
    /// The last song left: the queue is empty.
    Emptied,
}

/// What the player must do after a queue action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Play the song now at this position of the playback order.
    Play(usize),
    /// Restart the current song from the beginning.
    Restart,
    /// Past the last song: the first one is now current and must stay paused.
    EndOfQueue,
    /// Nothing to do (empty queue or invalid position).
    Nothing,
}

#[derive(Debug, Clone, Default)]
pub struct Queue {
    /// Song IDs in the order the context was shown.
    songs: Vec<String>,
    /// Playback order: indexes into `songs`.
    order: Vec<usize>,
    /// Position of the current song in `order`.
    current: usize,
    shuffle: bool,
    repeat: RepeatMode,
    /// "Playing from", e.g. an album name.
    label: String,
    /// Name of the app the queue was continued from (spec 5.6), shown
    /// instead of `label`.
    continuing_from: Option<String>,
}

/// Everything needed to rebuild a queue later (spec 5.6).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueueSnapshot {
    /// Song IDs in the original order.
    pub songs: Vec<String>,
    /// Playback order: indexes into `songs`.
    pub order: Vec<usize>,
    /// Position of the current song in `order`.
    pub current: usize,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub label: String,
    pub continuing_from: Option<String>,
}

impl QueueSnapshot {
    /// Song IDs in playback order.
    pub fn ids_in_order(&self) -> impl Iterator<Item = &str> {
        self.order.iter().filter_map(|&i| self.songs.get(i).map(String::as_str))
    }

    pub fn current_id(&self) -> Option<&str> {
        self.order.get(self.current).and_then(|&i| self.songs.get(i)).map(String::as_str)
    }
}

impl Queue {
    /// Replaces the queue with a whole context, starting at `start` (an index
    /// into `songs`). Shuffle and repeat stay as they were; with shuffle on,
    /// the clicked song goes to the top and the rest is shuffled.
    pub fn replace(&mut self, songs: Vec<String>, start: usize, label: String, rng: &mut impl Rng) {
        let start = start.min(songs.len().saturating_sub(1));
        self.order = (0..songs.len()).collect();
        self.songs = songs;
        self.label = label;
        self.continuing_from = None;
        self.current = start;
        if self.shuffle {
            self.shuffle_from_current(rng);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.songs.is_empty()
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn continuing_from(&self) -> Option<&str> {
        self.continuing_from.as_deref()
    }

    pub fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            songs: self.songs.clone(),
            order: self.order.clone(),
            current: self.current,
            shuffle: self.shuffle,
            repeat: self.repeat,
            label: self.label.clone(),
            continuing_from: self.continuing_from.clone(),
        }
    }

    /// Rebuilds a saved queue. A playback order that does not match the songs
    /// (e.g. a damaged record) falls back to the original order, shuffle off.
    pub fn from_snapshot(snapshot: QueueSnapshot) -> Self {
        let n = snapshot.songs.len();
        let valid_order = snapshot.order.len() == n && {
            let mut seen = vec![false; n];
            snapshot.order.iter().all(|&i| i < n && !std::mem::replace(&mut seen[i], true))
        };
        let (order, shuffle) = if valid_order {
            (snapshot.order, snapshot.shuffle)
        } else {
            ((0..n).collect(), false)
        };
        Self {
            songs: snapshot.songs,
            order,
            current: snapshot.current.min(n.saturating_sub(1)),
            shuffle,
            repeat: snapshot.repeat,
            label: snapshot.label,
            continuing_from: snapshot.continuing_from,
        }
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    /// Position of the current song in the playback order.
    pub fn current_position(&self) -> Option<usize> {
        (!self.is_empty()).then_some(self.current)
    }

    pub fn current_id(&self) -> Option<&str> {
        self.id_at(self.current)
    }

    /// Song ID at a position of the playback order.
    pub fn id_at(&self, position: usize) -> Option<&str> {
        self.order.get(position).map(|&i| self.songs[i].as_str())
    }

    /// Song IDs in playback order, for the UI.
    pub fn ids_in_order(&self) -> Vec<&str> {
        self.order.iter().map(|&i| self.songs[i].as_str()).collect()
    }

    /// Position that plays when the current song ends by itself, preloaded
    /// for gapless playback. `None` at the end of the queue with repeat off.
    pub fn upcoming(&self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        match self.repeat {
            RepeatMode::One => Some(self.current),
            RepeatMode::All => Some((self.current + 1) % self.order.len()),
            RepeatMode::Off => (self.current + 1 < self.order.len()).then_some(self.current + 1),
        }
    }

    /// The current song ended by itself.
    pub fn finish_current(&mut self) -> Step {
        match self.upcoming() {
            Some(position) => {
                self.current = position;
                Step::Play(position)
            }
            None if self.is_empty() => Step::Nothing,
            None => self.end_of_queue(),
        }
    }

    /// Next button. "Repeat one" does not apply here (spec 5.1.1).
    pub fn next(&mut self) -> Step {
        if self.is_empty() {
            return Step::Nothing;
        }
        if self.current + 1 < self.order.len() {
            self.current += 1;
            Step::Play(self.current)
        } else if self.repeat == RepeatMode::All {
            self.current = 0;
            Step::Play(0)
        } else {
            self.end_of_queue()
        }
    }

    /// Previous button. `played_secs` is how far the current song has played.
    pub fn previous(&mut self, played_secs: f64) -> Step {
        if self.is_empty() {
            return Step::Nothing;
        }
        if played_secs > RESTART_THRESHOLD_SECS {
            Step::Restart
        } else if self.current > 0 {
            self.current -= 1;
            Step::Play(self.current)
        } else if self.repeat == RepeatMode::All && self.order.len() > 1 {
            self.current = self.order.len() - 1;
            Step::Play(self.current)
        } else {
            Step::Restart
        }
    }

    /// Jumps to a position of the playback order (click in the queue list).
    pub fn jump_to(&mut self, position: usize) -> Step {
        if position >= self.order.len() {
            return Step::Nothing;
        }
        self.current = position;
        Step::Play(position)
    }

    /// Turns shuffle on (current song on top, all others shuffled after it)
    /// or off (original order, current song kept).
    pub fn set_shuffle(&mut self, enabled: bool, rng: &mut impl Rng) {
        if enabled == self.shuffle {
            return;
        }
        self.shuffle = enabled;
        if self.is_empty() {
            return;
        }
        if enabled {
            self.shuffle_from_current(rng);
        } else {
            let current_song = self.order[self.current];
            self.order = (0..self.songs.len()).collect();
            self.current = current_song;
        }
    }

    /// Moves the song at `from` to `to` (positions of the playback order),
    /// e.g. dragged in Up Next. Without shuffle, the original order changes
    /// too; with shuffle, only the shuffled one (turning shuffle off restores
    /// the original order, spec 5.4). Returns false if nothing moved.
    pub fn move_song(&mut self, from: usize, to: usize) -> bool {
        let len = self.order.len();
        if from >= len || to >= len || from == to {
            return false;
        }
        let song = self.order.remove(from);
        self.order.insert(to, song);
        self.current = if self.current == from {
            to
        } else if from < self.current && to >= self.current {
            self.current - 1
        } else if from > self.current && to <= self.current {
            self.current + 1
        } else {
            self.current
        };
        if !self.shuffle {
            self.songs = self.order.iter().map(|&i| self.songs[i].clone()).collect();
            self.order = (0..len).collect();
        }
        true
    }

    /// Removes the song at `position` of the playback order (Up Next).
    pub fn remove_song(&mut self, position: usize) -> Removed {
        if position >= self.order.len() {
            return Removed::Nothing;
        }
        let song = self.order.remove(position);
        self.songs.remove(song);
        for index in &mut self.order {
            if *index > song {
                *index -= 1;
            }
        }
        if self.order.is_empty() {
            self.current = 0;
            return Removed::Emptied;
        }
        match position.cmp(&self.current) {
            std::cmp::Ordering::Less => {
                self.current -= 1;
                Removed::Other
            }
            std::cmp::Ordering::Greater => Removed::Other,
            std::cmp::Ordering::Equal if position < self.order.len() => Removed::Current,
            std::cmp::Ordering::Equal => {
                self.current = 0;
                Removed::CurrentAtEnd
            }
        }
    }

    pub fn set_repeat(&mut self, repeat: RepeatMode) {
        self.repeat = repeat;
    }

    fn shuffle_from_current(&mut self, rng: &mut impl Rng) {
        let current_song = self.order[self.current];
        let mut rest: Vec<usize> = (0..self.songs.len()).filter(|&i| i != current_song).collect();
        rest.shuffle(rng);
        self.order = std::iter::once(current_song).chain(rest).collect();
        self.current = 0;
    }

    fn end_of_queue(&mut self) -> Step {
        self.current = 0;
        Step::EndOfQueue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    fn ids(n: usize) -> Vec<String> {
        (1..=n).map(|i| format!("s{i}")).collect()
    }

    fn queue(n: usize, start: usize) -> Queue {
        let mut queue = Queue::default();
        queue.replace(ids(n), start, "Album".into(), &mut rng());
        queue
    }

    #[test]
    fn replace_starts_at_the_clicked_song_with_the_whole_context() {
        let queue = queue(10, 4);
        assert_eq!(queue.current_id(), Some("s5"));
        assert_eq!(queue.ids_in_order().len(), 10);
        assert_eq!(queue.label(), "Album");
    }

    #[test]
    fn next_and_previous_move_through_the_context() {
        // Spec 5.1: clicking the 5th song, Previous goes to the 4th.
        let mut queue = queue(10, 4);
        assert_eq!(queue.previous(1.0), Step::Play(3));
        assert_eq!(queue.current_id(), Some("s4"));
        assert_eq!(queue.next(), Step::Play(4));
        assert_eq!(queue.current_id(), Some("s5"));
    }

    #[test]
    fn previous_after_three_seconds_restarts_the_song() {
        let mut queue = queue(10, 4);
        assert_eq!(queue.previous(3.5), Step::Restart);
        assert_eq!(queue.current_id(), Some("s5"));
        assert_eq!(queue.previous(3.0), Step::Play(3), "exactly 3 s goes back");
    }

    #[test]
    fn previous_on_the_first_song_restarts_unless_repeating_the_queue() {
        let mut queue = queue(3, 0);
        assert_eq!(queue.previous(0.5), Step::Restart);
        assert_eq!(queue.current_id(), Some("s1"));

        queue.set_repeat(RepeatMode::All);
        assert_eq!(queue.previous(0.5), Step::Play(2));
        assert_eq!(queue.current_id(), Some("s3"));
    }

    #[test]
    fn next_on_the_last_song_goes_back_to_the_first_paused() {
        let mut queue = queue(3, 2);
        assert_eq!(queue.next(), Step::EndOfQueue);
        assert_eq!(queue.current_id(), Some("s1"));
    }

    #[test]
    fn next_on_the_last_song_with_repeat_all_keeps_playing() {
        let mut queue = queue(3, 2);
        queue.set_repeat(RepeatMode::All);
        assert_eq!(queue.next(), Step::Play(0));
    }

    #[test]
    fn next_ignores_repeat_one() {
        let mut queue = queue(3, 0);
        queue.set_repeat(RepeatMode::One);
        assert_eq!(queue.next(), Step::Play(1));
        assert_eq!(queue.current_id(), Some("s2"));
    }

    #[test]
    fn natural_end_follows_the_repeat_mode() {
        let mut off = queue(3, 2);
        assert_eq!(off.upcoming(), None);
        assert_eq!(off.finish_current(), Step::EndOfQueue);
        assert_eq!(off.current_id(), Some("s1"));

        let mut all = queue(3, 2);
        all.set_repeat(RepeatMode::All);
        assert_eq!(all.upcoming(), Some(0));
        assert_eq!(all.finish_current(), Step::Play(0));

        let mut one = queue(3, 1);
        one.set_repeat(RepeatMode::One);
        assert_eq!(one.upcoming(), Some(1));
        assert_eq!(one.finish_current(), Step::Play(1));
        assert_eq!(one.current_id(), Some("s2"));
    }

    #[test]
    fn upcoming_is_the_next_song_in_the_middle_of_the_queue() {
        let queue = queue(5, 1);
        assert_eq!(queue.upcoming(), Some(2));
    }

    #[test]
    fn repeat_cycles_off_all_one() {
        assert_eq!(RepeatMode::Off.cycled(), RepeatMode::All);
        assert_eq!(RepeatMode::All.cycled(), RepeatMode::One);
        assert_eq!(RepeatMode::One.cycled(), RepeatMode::Off);
    }

    #[test]
    fn shuffle_puts_the_current_song_on_top_and_keeps_every_song_once() {
        let mut queue = queue(20, 7);
        queue.set_shuffle(true, &mut rng());

        assert_eq!(queue.current_position(), Some(0));
        assert_eq!(queue.current_id(), Some("s8"));
        let mut shuffled: Vec<&str> = queue.ids_in_order();
        assert_ne!(shuffled, ids(20).iter().map(String::as_str).collect::<Vec<_>>());
        shuffled.sort_unstable();
        let mut expected = ids(20);
        expected.sort_unstable();
        assert_eq!(shuffled, expected, "same songs, no duplicates");
    }

    #[test]
    fn shuffle_off_restores_the_original_order_keeping_the_current_song() {
        let mut queue = queue(20, 7);
        queue.set_shuffle(true, &mut rng());
        queue.next();
        queue.next();
        let playing = queue.current_id().unwrap().to_string();

        queue.set_shuffle(false, &mut rng());
        assert_eq!(queue.ids_in_order(), ids(20).iter().map(String::as_str).collect::<Vec<_>>());
        assert_eq!(queue.current_id(), Some(playing.as_str()));
        let original_index: usize = playing[1..].parse::<usize>().unwrap() - 1;
        assert_eq!(queue.current_position(), Some(original_index));
    }

    #[test]
    fn new_context_with_shuffle_on_starts_with_the_clicked_song() {
        let mut queue = queue(5, 0);
        queue.set_shuffle(true, &mut rng());
        queue.replace(ids(8), 5, "Other".into(), &mut rng());
        assert!(queue.shuffle());
        assert_eq!(queue.current_position(), Some(0));
        assert_eq!(queue.current_id(), Some("s6"));
    }

    #[test]
    fn empty_queue_does_nothing() {
        let mut queue = Queue::default();
        assert_eq!(queue.next(), Step::Nothing);
        assert_eq!(queue.previous(0.0), Step::Nothing);
        assert_eq!(queue.finish_current(), Step::Nothing);
        assert_eq!(queue.upcoming(), None);
        assert_eq!(queue.current_id(), None);
        queue.set_shuffle(true, &mut rng());
        assert!(queue.shuffle());
    }

    #[test]
    fn snapshot_rebuilds_the_same_queue() {
        let mut original = queue(10, 3);
        original.set_shuffle(true, &mut rng());
        original.set_repeat(RepeatMode::All);
        original.next();

        let rebuilt = Queue::from_snapshot(original.snapshot());
        assert_eq!(rebuilt.ids_in_order(), original.ids_in_order());
        assert_eq!(rebuilt.current_id(), original.current_id());
        assert!(rebuilt.shuffle());
        assert_eq!(rebuilt.repeat(), RepeatMode::All);
        assert_eq!(rebuilt.label(), "Album");

        // Shuffle off still restores the original order after a rebuild.
        let mut rebuilt = rebuilt;
        rebuilt.set_shuffle(false, &mut rng());
        assert_eq!(rebuilt.ids_in_order(), ids(10).iter().map(String::as_str).collect::<Vec<_>>());
    }

    #[test]
    fn snapshot_with_a_broken_order_falls_back_to_the_original_order() {
        let snapshot = QueueSnapshot {
            songs: ids(3),
            order: vec![0, 0, 2],
            current: 9,
            shuffle: true,
            ..QueueSnapshot::default()
        };
        let queue = Queue::from_snapshot(snapshot);
        assert_eq!(queue.ids_in_order(), ["s1", "s2", "s3"]);
        assert!(!queue.shuffle());
        assert_eq!(queue.current_id(), Some("s3"), "current is clamped to the last song");
    }

    #[test]
    fn a_new_context_clears_continuing_from() {
        let mut queue = Queue::from_snapshot(QueueSnapshot {
            songs: ids(2),
            order: vec![0, 1],
            continuing_from: Some("Symfonium".into()),
            ..QueueSnapshot::default()
        });
        assert_eq!(queue.continuing_from(), Some("Symfonium"));
        queue.replace(ids(3), 0, "Album".into(), &mut rng());
        assert_eq!(queue.continuing_from(), None);
    }

    #[test]
    fn repeat_names_round_trip() {
        for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
            assert_eq!(RepeatMode::from_name(mode.as_str()), mode);
        }
        assert_eq!(RepeatMode::from_name("bogus"), RepeatMode::Off);
    }

    #[test]
    fn moving_without_shuffle_changes_the_original_order() {
        let mut queue = queue(5, 2); // current s3
        assert!(queue.move_song(4, 0));
        assert_eq!(queue.ids_in_order(), ["s5", "s1", "s2", "s3", "s4"]);
        assert_eq!(queue.current_id(), Some("s3"), "the current song follows");
        assert_eq!(queue.current_position(), Some(3));

        // Shuffle on and off again keeps the moved order.
        queue.set_shuffle(true, &mut rng());
        queue.set_shuffle(false, &mut rng());
        assert_eq!(queue.ids_in_order(), ["s5", "s1", "s2", "s3", "s4"]);
    }

    #[test]
    fn moving_the_current_song_keeps_it_current() {
        let mut queue = queue(5, 1);
        assert!(queue.move_song(1, 3));
        assert_eq!(queue.ids_in_order(), ["s1", "s3", "s4", "s2", "s5"]);
        assert_eq!(queue.current_position(), Some(3));
        assert_eq!(queue.current_id(), Some("s2"));

        assert!(queue.move_song(0, 4), "a song from before the current one");
        assert_eq!(queue.current_id(), Some("s2"));
        assert_eq!(queue.current_position(), Some(2));
        assert!(!queue.move_song(2, 2) && !queue.move_song(9, 0), "nothing to move");
    }

    #[test]
    fn moving_with_shuffle_only_changes_the_shuffled_order() {
        let mut queue = queue(5, 0);
        queue.set_shuffle(true, &mut rng());
        let before = queue.ids_in_order().into_iter().map(String::from).collect::<Vec<_>>();
        assert!(queue.move_song(4, 1));
        let after = queue.ids_in_order();
        assert_eq!(after[1], before[4]);

        queue.set_shuffle(false, &mut rng());
        assert_eq!(queue.ids_in_order(), ["s1", "s2", "s3", "s4", "s5"], "original order restored");
    }

    #[test]
    fn removing_songs_around_the_current_one() {
        let mut queue = queue(5, 2); // current s3
        assert_eq!(queue.remove_song(0), Removed::Other);
        assert_eq!(queue.current_id(), Some("s3"));
        assert_eq!(queue.current_position(), Some(1));
        assert_eq!(queue.remove_song(3), Removed::Other);
        assert_eq!(queue.ids_in_order(), ["s2", "s3", "s4"]);
        assert_eq!(queue.remove_song(7), Removed::Nothing);
    }

    #[test]
    fn removing_the_current_song_moves_on() {
        let mut queue = queue(3, 1);
        assert_eq!(queue.remove_song(1), Removed::Current);
        assert_eq!(queue.current_id(), Some("s3"), "the next one took its place");

        assert_eq!(queue.remove_song(1), Removed::CurrentAtEnd);
        assert_eq!(queue.current_id(), Some("s1"), "it was the last: back to the first");

        assert_eq!(queue.remove_song(0), Removed::Emptied);
        assert!(queue.is_empty());
    }

    #[test]
    fn removing_with_shuffle_keeps_both_orders_in_step() {
        let mut queue = queue(6, 0);
        queue.set_shuffle(true, &mut rng());
        let removed = queue.ids_in_order()[3].to_string();
        queue.remove_song(3);
        assert!(!queue.ids_in_order().contains(&removed.as_str()));
        queue.set_shuffle(false, &mut rng());
        let expected: Vec<String> = ids(6).into_iter().filter(|id| *id != removed).collect();
        assert_eq!(queue.ids_in_order(), expected.iter().map(String::as_str).collect::<Vec<_>>());
    }

    #[test]
    fn jump_to_an_invalid_position_does_nothing() {
        let mut queue = queue(3, 0);
        assert_eq!(queue.jump_to(5), Step::Nothing);
        assert_eq!(queue.jump_to(2), Step::Play(2));
        assert_eq!(queue.current_id(), Some("s3"));
    }
}
