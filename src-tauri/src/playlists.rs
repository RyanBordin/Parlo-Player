//! Playlist changes of the "..." menus (spec 8.1, 8.2). Pure logic: no
//! network, no database.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// A song of a playlist as the UI shows it: its ID and which of its
/// occurrences it is (a playlist can repeat a song). The UI may leave out
/// songs (hidden music folders), so its positions are not the server's; the
/// n-th occurrence of an ID is the same in both lists.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub id: String,
    /// 0 for the first time the song appears.
    pub occurrence: usize,
}

/// What to do with the songs already in the playlist (spec 8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Duplicates {
    /// "Add anyway" / "Add all".
    AddAll,
    /// "Skip" / "Add only the new ones".
    OnlyNew,
}

/// Answer of "Add to playlist".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AddOutcome {
    /// Nothing was added: `count` of the `total` songs are already there,
    /// and the user chooses (`Duplicates`).
    Duplicates { count: usize, total: usize },
    /// `count` songs were added.
    Added { count: usize },
}

/// How many of `adding` are already in the playlist.
pub fn duplicate_count(existing: &[String], adding: &[String]) -> usize {
    let existing: HashSet<&str> = existing.iter().map(String::as_str).collect();
    adding.iter().filter(|id| existing.contains(id.as_str())).count()
}

/// The songs to add, in order, after the user's choice.
pub fn songs_to_add(existing: &[String], adding: Vec<String>, choice: Duplicates) -> Vec<String> {
    match choice {
        Duplicates::AddAll => adding,
        Duplicates::OnlyNew => {
            let existing: HashSet<&str> = existing.iter().map(String::as_str).collect();
            adding.into_iter().filter(|id| !existing.contains(id.as_str())).collect()
        }
    }
}

/// The server positions of these entries, from the playlist as the server
/// has it now. Entries no longer there are left out.
pub fn server_positions(server_songs: &[String], entries: &[PlaylistEntry]) -> Vec<usize> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut positions_of: HashMap<(&str, usize), usize> = HashMap::new();
    for (position, id) in server_songs.iter().enumerate() {
        let occurrence = seen.entry(id.as_str()).or_insert(0);
        positions_of.insert((id.as_str(), *occurrence), position);
        *occurrence += 1;
    }
    let mut positions: Vec<usize> = entries
        .iter()
        .filter_map(|entry| positions_of.get(&(entry.id.as_str(), entry.occurrence)).copied())
        .collect();
    positions.sort_unstable();
    positions.dedup();
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| id.to_string()).collect()
    }

    fn entry(id: &str, occurrence: usize) -> PlaylistEntry {
        PlaylistEntry { id: id.into(), occurrence }
    }

    #[test]
    fn duplicates_are_counted_and_skipped_on_request() {
        let existing = ids(&["a", "b"]);
        let adding = ids(&["b", "c", "a", "d"]);
        assert_eq!(duplicate_count(&existing, &adding), 2);
        assert_eq!(songs_to_add(&existing, adding.clone(), Duplicates::OnlyNew), ["c", "d"]);
        assert_eq!(songs_to_add(&existing, adding, Duplicates::AddAll), ["b", "c", "a", "d"]);
    }

    #[test]
    fn entries_find_their_server_position_by_occurrence() {
        // The UI hid "x" (hidden folder): its positions are off by one.
        let server = ids(&["a", "x", "b", "a"]);
        assert_eq!(server_positions(&server, &[entry("a", 1), entry("b", 0)]), [2, 3]);
        assert_eq!(server_positions(&server, &[entry("a", 0), entry("a", 0)]), [0], "once each");
        assert!(server_positions(&server, &[entry("a", 2), entry("gone", 0)]).is_empty());
    }

    #[test]
    fn add_outcome_says_its_kind() {
        let json = serde_json::to_value(AddOutcome::Duplicates { count: 1, total: 3 }).unwrap();
        assert_eq!(json, serde_json::json!({ "kind": "duplicates", "count": 1, "total": 3 }));
    }
}
