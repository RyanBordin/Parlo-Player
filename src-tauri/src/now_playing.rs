//! Lyrics and Related tabs of Now Playing (spec 4.4).

use rusqlite::Connection;
use serde::Serialize;

use crate::db::DbResult;
use crate::db::home::AlbumCard;
use crate::db::library::SongSummary;
use crate::db::related::{self, SongContext};
use crate::home::{ROW_SIZE, SongRow, TRACK_ROW_SIZE};
use crate::subsonic::{Song, StructuredLyrics};

/// Songs asked from `getSimilarSongs2`.
pub const SIMILAR_COUNT: u32 = 50;

/// The lyrics of a song, as the Lyrics tab shows them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Lyrics {
    /// Lines with their start time; the current one is highlighted.
    Synced {
        lines: Vec<SyncedLine>,
        /// Milliseconds; positive means the lines show sooner.
        offset_ms: i64,
    },
    /// Text without timings.
    Plain { text: String },
    /// No lyrics for this song.
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncedLine {
    pub start_ms: i64,
    pub text: String,
}

/// Lyrics from `getLyricsBySongId`: the first synced ones, else the first
/// with any text (shown as plain text). `None` when there are none, so the
/// caller tries `getLyrics`.
pub fn from_structured(list: Vec<StructuredLyrics>) -> Option<Lyrics> {
    let synced = list.iter().find(|l| l.synced && l.line.iter().any(|line| line.start.is_some()));
    if let Some(lyrics) = synced {
        let mut lines: Vec<SyncedLine> = lyrics
            .line
            .iter()
            .filter_map(|line| line.start.map(|start| SyncedLine { start_ms: start, text: line.value.clone() }))
            .collect();
        lines.sort_by_key(|line| line.start_ms);
        return Some(Lyrics::Synced { lines, offset_ms: lyrics.offset });
    }
    list.iter()
        .map(|l| l.line.iter().map(|line| line.value.as_str()).collect::<Vec<_>>().join("\n"))
        .find(|text| !text.trim().is_empty())
        .map(|text| Lyrics::Plain { text })
}

/// Plain lyrics from `getLyrics`, or none.
pub fn from_plain(text: Option<String>) -> Lyrics {
    text.map_or(Lyrics::None, |text| Lyrics::Plain { text })
}

/// The four sections of the Related tab, in order. Empty ones are not shown.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Related {
    /// "You might also like" (server, `getSimilarSongs2`).
    pub similar: SongRow,
    pub artist: Option<String>,
    /// "More from [Artist]".
    pub artist_songs: SongRow,
    /// "Albums by [Artist]".
    pub artist_albums: Vec<AlbumCard>,
    pub genre: Option<String>,
    /// "Same genre ([Genre])".
    pub genre_albums: Vec<AlbumCard>,
}

/// Builds the sections around `song_id`. `similar` comes from the server
/// (empty when it has nothing, e.g. Navidrome without Last.fm).
pub fn related(conn: &Connection, song_id: &str, context: &SongContext, similar: Vec<Song>) -> DbResult<Related> {
    let similar: Vec<SongSummary> = similar.iter().filter(|s| s.id != song_id).map(summary_of).collect();
    let (artist_songs, artist_albums) = match context.artist_id.as_deref() {
        Some(artist_id) => (
            related::artist_top_songs(conn, artist_id, song_id)?,
            related::artist_albums(conn, artist_id, ROW_SIZE as u32)?,
        ),
        None => (Vec::new(), Vec::new()),
    };
    let genre_albums = match context.genre.as_deref() {
        Some(genre) => related::genre_albums(conn, genre, context.album_id.as_deref(), ROW_SIZE as u32)?,
        None => Vec::new(),
    };
    Ok(Related {
        similar: SongRow::new(similar, TRACK_ROW_SIZE),
        artist: context.artist.clone(),
        artist_songs: SongRow::new(artist_songs, TRACK_ROW_SIZE),
        artist_albums,
        genre: context.genre.clone(),
        genre_albums,
    })
}

fn summary_of(song: &Song) -> SongSummary {
    SongSummary {
        id: song.id.clone(),
        title: song.title.clone(),
        artist: song.artist.clone(),
        album: song.album.clone(),
        artist_id: song.artist_id.clone(),
        album_id: song.album_id.clone(),
        cover_art: song.cover_art.clone(),
        duration: song.duration,
        starred: song.starred.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsonic::LyricsLine;

    fn line(start: Option<i64>, value: &str) -> LyricsLine {
        LyricsLine { start, value: value.into() }
    }

    fn lyrics(synced: bool, lines: Vec<LyricsLine>) -> StructuredLyrics {
        StructuredLyrics { synced, offset: 0, line: lines }
    }

    #[test]
    fn synced_lyrics_win_and_keep_the_offset() {
        let mut synced = lyrics(true, vec![line(Some(2000), "Two"), line(Some(0), "One")]);
        synced.offset = -100;
        let plain = lyrics(false, vec![line(None, "Plain")]);
        assert_eq!(
            from_structured(vec![plain, synced]),
            Some(Lyrics::Synced {
                lines: vec![
                    SyncedLine { start_ms: 0, text: "One".into() },
                    SyncedLine { start_ms: 2000, text: "Two".into() },
                ],
                offset_ms: -100,
            })
        );
    }

    #[test]
    fn unsynced_lyrics_become_plain_text() {
        let plain = lyrics(false, vec![line(None, "First"), line(None, "Second")]);
        assert_eq!(from_structured(vec![plain]), Some(Lyrics::Plain { text: "First\nSecond".into() }));
    }

    #[test]
    fn no_structured_lyrics_lets_get_lyrics_try() {
        assert_eq!(from_structured(Vec::new()), None);
        assert_eq!(from_structured(vec![lyrics(false, vec![line(None, "  ")])]), None, "blank lines only");
        assert_eq!(from_plain(Some("Text".into())), Lyrics::Plain { text: "Text".into() });
        assert_eq!(from_plain(None), Lyrics::None);
    }

    #[test]
    fn lyrics_say_their_kind() {
        let json = serde_json::to_value(Lyrics::Synced { lines: vec![], offset_ms: 5 }).unwrap();
        assert_eq!(json, serde_json::json!({ "kind": "synced", "lines": [], "offsetMs": 5 }));
        assert_eq!(serde_json::to_value(Lyrics::None).unwrap(), serde_json::json!({ "kind": "none" }));
    }

    #[test]
    fn similar_songs_leave_out_the_song_playing() {
        let conn = crate::db::open_in_memory().unwrap();
        let context = SongContext { title: "t".into(), artist: None, artist_id: None, album_id: None, genre: None };
        let similar = vec![
            Song { id: "current".into(), ..Song::default() },
            Song { id: "other".into(), ..Song::default() },
        ];
        let sections = related(&conn, "current", &context, similar).unwrap();
        assert_eq!(sections.similar.all_ids, ["other"]);
        assert!(sections.artist_songs.songs.is_empty() && sections.genre_albums.is_empty());
    }
}
