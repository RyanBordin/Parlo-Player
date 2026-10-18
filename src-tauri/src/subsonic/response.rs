//! Parsing of the `subsonic-response` JSON envelope.
//!
//! Convention: every ID coming from the API is a `String`, never a number.

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use super::error::SubsonicError;

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "subsonic-response")]
    body: ResponseBody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Failed,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub code: i32,
    pub message: Option<String>,
}

/// Fields common to every Subsonic response. `type`, `serverVersion` and
/// `openSubsonic` are OpenSubsonic extensions, absent on older servers.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBody {
    pub status: Status,
    pub version: String,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: Option<bool>,
    pub error: Option<ApiError>,
    /// Endpoint-specific data, e.g. `searchResult3` or `playlists`.
    #[serde(flatten)]
    pub payload: Map<String, Value>,
}

impl ResponseBody {
    /// Removes and parses one endpoint-specific field. A missing field gives
    /// the default value (servers omit empty results).
    pub fn take<T: DeserializeOwned + Default>(&mut self, key: &str) -> Result<T, SubsonicError> {
        match self.payload.remove(key) {
            Some(value) => serde_json::from_value(value)
                .map_err(|e| SubsonicError::InvalidResponse(format!("{key}: {e}"))),
            None => Ok(T::default()),
        }
    }
}

/// Parses a raw JSON body. A `failed` status becomes `SubsonicError::Api`.
pub fn parse_response(json: &str) -> Result<ResponseBody, SubsonicError> {
    let envelope: Envelope = serde_json::from_str(json)
        .map_err(|e| SubsonicError::InvalidResponse(e.to_string()))?;
    let body = envelope.body;

    match body.status {
        Status::Ok => Ok(body),
        Status::Failed => {
            let (code, message) = match body.error {
                Some(err) => (err.code, err.message.unwrap_or_default()),
                None => (0, String::from("unknown error")),
            };
            Err(SubsonicError::Api { code, message })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_navidrome_success() {
        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "type":"navidrome","serverVersion":"0.58.0 (abc1234)","openSubsonic":true}}"#;

        let body = parse_response(json).expect("should parse");
        assert_eq!(body.status, Status::Ok);
        assert_eq!(body.version, "1.16.1");
        assert_eq!(body.server_type.as_deref(), Some("navidrome"));
        assert_eq!(body.server_version.as_deref(), Some("0.58.0 (abc1234)"));
        assert_eq!(body.open_subsonic, Some(true));
    }

    #[test]
    fn takes_search3_payload() {
        use crate::subsonic::types::{LibraryItems, RawLibraryItems};

        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "searchResult3":{
                "artist":[{"id":"ar1","name":"Artist","albumCount":2}],
                "album":[{"id":"al1","name":"Album","artistId":"ar1","year":2001}],
                "song":[{"id":"s1","title":"Song","albumId":"al1","suffix":"mp3"}]}}}"#;

        let mut body = parse_response(json).unwrap();
        let items: LibraryItems = body.take::<RawLibraryItems>("searchResult3").unwrap().try_into().unwrap();
        assert_eq!(items.artists[0].album_count, Some(2));
        assert_eq!(items.albums[0].artist_id.as_deref(), Some("ar1"));
        assert_eq!(items.songs[0].suffix.as_deref(), Some("mp3"));
        assert!(items.songs[0].raw_json.contains("\"albumId\":\"al1\""));
    }

    #[test]
    fn empty_search3_result_gives_no_items() {
        use crate::subsonic::types::{LibraryItems, RawLibraryItems};

        // Navidrome sends an empty object when a page has no more items.
        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1","searchResult3":{}}}"#;
        let mut body = parse_response(json).unwrap();
        let items: LibraryItems = body.take::<RawLibraryItems>("searchResult3").unwrap().try_into().unwrap();
        assert_eq!(items, LibraryItems::default());
    }

    #[test]
    fn takes_scan_status_with_navidrome_extension() {
        use crate::subsonic::types::ScanStatus;

        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "scanStatus":{"scanning":false,"count":5120,"folderCount":3,
            "lastScan":"2026-10-17T22:10:00Z"}}}"#;
        let status: ScanStatus = parse_response(json).unwrap().take("scanStatus").unwrap();
        assert_eq!(status.count, Some(5120));
        assert_eq!(status.last_scan.as_deref(), Some("2026-10-17T22:10:00Z"));
    }

    #[test]
    fn takes_album_list2() {
        use crate::subsonic::types::{Album, RawAlbumList, with_raw_json};

        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "albumList2":{"album":[{"id":"al1","name":"One","artist":"A","artistId":"ar1",
            "coverArt":"al-al1","played":"2026-10-18T10:00:00Z","playCount":7}]}}}"#;
        let raw: RawAlbumList = parse_response(json).unwrap().take("albumList2").unwrap();
        let albums: Vec<Album> = with_raw_json(raw.album).unwrap();
        assert_eq!(albums[0].id, "al1");
        assert_eq!(albums[0].played.as_deref(), Some("2026-10-18T10:00:00Z"));
        assert_eq!(albums[0].play_count, Some(7));
    }

    #[test]
    fn takes_play_queue() {
        use crate::subsonic::types::PlayQueue;

        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "playQueue":{"entry":[{"id":"s2","title":"Two"},{"id":"s1","title":"One"}],
            "current":"s1","position":83500,"username":"alice",
            "changed":"2026-10-18T12:34:56.789Z","changedBy":"Symfonium"}}}"#;
        let queue: PlayQueue = parse_response(json).unwrap().take("playQueue").unwrap();
        let ids: Vec<&str> = queue.entry.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["s2", "s1"]);
        assert_eq!(queue.current.as_deref(), Some("s1"));
        assert_eq!(queue.position, Some(83_500));
        assert_eq!(queue.changed_by.as_deref(), Some("Symfonium"));
    }

    #[test]
    fn no_saved_play_queue_gives_an_empty_one() {
        use crate::subsonic::types::PlayQueue;

        // Navidrome omits `playQueue` when nothing was saved.
        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1"}}"#;
        let queue: PlayQueue = parse_response(json).unwrap().take("playQueue").unwrap();
        assert!(queue.entry.is_empty());
    }

    #[test]
    fn takes_opensubsonic_extensions() {
        use crate::subsonic::types::Extension;

        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1",
            "openSubsonicExtensions":[{"name":"transcodeOffset","versions":[1]},
            {"name":"formPost","versions":[1]}]}}"#;
        let extensions: Vec<Extension> =
            parse_response(json).unwrap().take("openSubsonicExtensions").unwrap();
        assert!(extensions.iter().any(|e| e.name == "formPost" && e.versions == [1]));
    }

    #[test]
    fn parses_minimal_success_without_opensubsonic_fields() {
        let json = r#"{"subsonic-response":{"status":"ok","version":"1.16.1"}}"#;

        let body = parse_response(json).expect("should parse");
        assert!(body.server_type.is_none());
        assert!(body.open_subsonic.is_none());
    }

    #[test]
    fn parses_wrong_credentials_error() {
        let json = r#"{"subsonic-response":{"status":"failed","version":"1.16.1",
            "type":"navidrome","serverVersion":"0.58.0","openSubsonic":true,
            "error":{"code":40,"message":"Wrong username or password"}}}"#;

        match parse_response(json) {
            Err(SubsonicError::Api { code, message }) => {
                assert_eq!(code, 40);
                assert_eq!(message, "Wrong username or password");
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn failed_status_without_error_object_is_still_an_error() {
        let json = r#"{"subsonic-response":{"status":"failed","version":"1.16.1"}}"#;
        assert!(matches!(
            parse_response(json),
            Err(SubsonicError::Api { code: 0, .. })
        ));
    }

    #[test]
    fn html_page_is_invalid_response() {
        let html = "<!doctype html><html><body>Not a Subsonic server</body></html>";
        assert!(matches!(
            parse_response(html),
            Err(SubsonicError::InvalidResponse(_))
        ));
    }

    #[test]
    fn json_without_envelope_is_invalid_response() {
        assert!(matches!(
            parse_response(r#"{"status":"ok"}"#),
            Err(SubsonicError::InvalidResponse(_))
        ));
    }
}
