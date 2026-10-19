//! Library items returned by the API.
//!
//! IDs are always `String`. Each item also keeps its original JSON in
//! `raw_json`, so new fields can be used later without a re-sync (spec 12.4).

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::error::SubsonicError;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Genre {
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub sort_name: Option<String>,
    pub album_count: Option<i64>,
    pub cover_art: Option<String>,
    pub artist_image_url: Option<String>,
    /// Date the item was starred (ISO 8601), absent if not a favorite.
    pub starred: Option<String>,
    pub music_brainz_id: Option<String>,
    /// OpenSubsonic: "albumartist", "artist", "composer", ...
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(skip)]
    pub raw_json: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub sort_name: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art: Option<String>,
    pub song_count: Option<i64>,
    /// Seconds.
    pub duration: Option<i64>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Date added to the server.
    pub created: Option<String>,
    pub starred: Option<String>,
    pub play_count: Option<i64>,
    /// Last played date.
    pub played: Option<String>,
    pub is_compilation: Option<bool>,
    pub music_brainz_id: Option<String>,
    #[serde(skip)]
    pub raw_json: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Song {
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub sort_name: Option<String>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub display_album_artist: Option<String>,
    pub display_composer: Option<String>,
    pub track: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub genre: Option<String>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    /// Seconds.
    pub duration: Option<i64>,
    /// kbps.
    pub bit_rate: Option<i64>,
    /// Hz.
    pub sampling_rate: Option<i64>,
    pub bit_depth: Option<i64>,
    pub channel_count: Option<i64>,
    /// File format, e.g. "flac", "mp3".
    pub suffix: Option<String>,
    pub content_type: Option<String>,
    /// Bytes.
    pub size: Option<i64>,
    pub path: Option<String>,
    pub created: Option<String>,
    pub starred: Option<String>,
    pub play_count: Option<i64>,
    pub played: Option<String>,
    pub cover_art: Option<String>,
    pub bpm: Option<i64>,
    pub music_brainz_id: Option<String>,
    #[serde(skip)]
    pub raw_json: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub comment: Option<String>,
    pub owner: Option<String>,
    pub public: Option<bool>,
    pub song_count: Option<i64>,
    pub duration: Option<i64>,
    pub created: Option<String>,
    pub changed: Option<String>,
    pub cover_art: Option<String>,
    #[serde(skip)]
    pub raw_json: String,
}

/// `getScanStatus`. `lastScan` is a Navidrome extension.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    #[serde(default)]
    pub scanning: bool,
    /// Number of songs in the library, used as the sync progress total.
    pub count: Option<i64>,
    pub last_scan: Option<String>,
}

/// `getPlayQueue`: the queue another client (or this one) saved on the server.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayQueue {
    /// Songs in playback order. Only the IDs are used.
    #[serde(default)]
    pub entry: Vec<PlayQueueEntry>,
    /// ID of the current song.
    pub current: Option<String>,
    /// Milliseconds into the current song.
    pub position: Option<i64>,
    /// When it was saved (ISO 8601, server clock).
    pub changed: Option<String>,
    /// Client name (`c` parameter) of the app that saved it.
    pub changed_by: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PlayQueueEntry {
    pub id: String,
}

/// One item of `getOpenSubsonicExtensions`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Extension {
    pub name: String,
    #[serde(default)]
    pub versions: Vec<i64>,
}

/// Artists, albums and songs from `search3` or `getStarred2`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibraryItems {
    pub artists: Vec<Artist>,
    pub albums: Vec<Album>,
    pub songs: Vec<Song>,
}

/// Shape shared by `searchResult3` and `starred2`, before typing each item.
#[derive(Debug, Default, Deserialize)]
pub(super) struct RawLibraryItems {
    #[serde(default)]
    artist: Vec<Value>,
    #[serde(default)]
    album: Vec<Value>,
    #[serde(default)]
    song: Vec<Value>,
}

impl TryFrom<RawLibraryItems> for LibraryItems {
    type Error = SubsonicError;

    fn try_from(raw: RawLibraryItems) -> Result<Self, Self::Error> {
        Ok(Self {
            artists: with_raw_json(raw.artist)?,
            albums: with_raw_json(raw.album)?,
            songs: with_raw_json(raw.song)?,
        })
    }
}

/// `getLyricsBySongId` returns `{ "lyricsList": { "structuredLyrics": [...] } }`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsList {
    #[serde(default)]
    pub structured_lyrics: Vec<StructuredLyrics>,
}

/// Lyrics of a song in one language.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredLyrics {
    #[serde(default)]
    pub synced: bool,
    /// Milliseconds; positive means the lines show sooner.
    #[serde(default)]
    pub offset: i64,
    #[serde(default)]
    pub line: Vec<LyricsLine>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct LyricsLine {
    /// Milliseconds from the start of the song (synced lyrics only).
    pub start: Option<i64>,
    #[serde(default)]
    pub value: String,
}

/// `getLyrics` returns `{ "lyrics": { "artist", "title", "value" } }`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PlainLyrics {
    pub value: Option<String>,
}

/// `getSimilarSongs2` returns `{ "similarSongs2": { "song": [...] } }`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct RawSongList {
    #[serde(default)]
    pub song: Vec<Value>,
}

/// `getAlbumList2` returns `{ "albumList2": { "album": [...] } }`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct RawAlbumList {
    #[serde(default)]
    pub album: Vec<Value>,
}

/// `getPlaylists` returns `{ "playlists": { "playlist": [...] } }`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct RawPlaylists {
    #[serde(default)]
    pub playlist: Vec<Value>,
}

pub(super) trait HasRawJson: DeserializeOwned {
    fn set_raw_json(&mut self, raw: String);
}

macro_rules! impl_has_raw_json {
    ($($t:ty),*) => {
        $(impl HasRawJson for $t {
            fn set_raw_json(&mut self, raw: String) {
                self.raw_json = raw;
            }
        })*
    };
}
impl_has_raw_json!(Artist, Album, Song, Playlist);

/// Types each JSON item and keeps its original text in `raw_json`.
pub(super) fn with_raw_json<T: HasRawJson>(values: Vec<Value>) -> Result<Vec<T>, SubsonicError> {
    values
        .into_iter()
        .map(|value| {
            let raw = value.to_string();
            let mut item: T = serde_json::from_value(value)
                .map_err(|e| SubsonicError::InvalidResponse(e.to_string()))?;
            item.set_raw_json(raw);
            Ok(item)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn song_keeps_typed_fields_and_raw_json() {
        let value = json!({
            "id": "a1b2c3", "title": "Song", "album": "Album", "albumId": "al1",
            "artist": "Artist", "artistId": "ar1", "track": 3, "discNumber": 1,
            "year": 2020, "genre": "Rock", "genres": [{"name": "Rock"}, {"name": "Indie"}],
            "duration": 215, "bitRate": 1015, "samplingRate": 44100, "bitDepth": 16,
            "suffix": "flac", "size": 27000000, "created": "2026-01-02T03:04:05Z",
            "starred": "2026-02-01T00:00:00Z", "playCount": 7, "played": "2026-10-01T10:00:00Z",
            "replayGain": {"trackGain": -6.5}
        });

        let songs: Vec<Song> = with_raw_json(vec![value]).unwrap();
        let song = &songs[0];
        assert_eq!(song.id, "a1b2c3");
        assert_eq!(song.genres.len(), 2);
        assert_eq!(song.bit_rate, Some(1015));
        assert_eq!(song.suffix.as_deref(), Some("flac"));
        assert_eq!(song.play_count, Some(7));
        // Fields without a column (e.g. replayGain) survive in the raw JSON.
        assert!(song.raw_json.contains("replayGain"));
    }

    #[test]
    fn missing_optional_fields_are_none() {
        let artists: Vec<Artist> = with_raw_json(vec![json!({"id": "ar1", "name": "A"})]).unwrap();
        assert_eq!(artists[0].starred, None);
        assert!(artists[0].roles.is_empty());
    }

    #[test]
    fn numeric_id_is_rejected() {
        // IDs must be strings; a number means a malformed response.
        let result: Result<Vec<Artist>, _> = with_raw_json(vec![json!({"id": 42, "name": "A"})]);
        assert!(result.is_err());
    }
}
