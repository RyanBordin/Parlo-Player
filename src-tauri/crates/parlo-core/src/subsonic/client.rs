use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::auth::{generate_salt, make_token};
use crate::settings::{StreamMode, StreamQuality};
use super::error::SubsonicError;
use super::response::{ResponseBody, parse_response};
use super::secret_url::SecretUrl;
use super::types::{
    Album, ArtistInfo, Extension, LibraryItems, MusicFolder, MusicFolders, LyricsList, PlainLyrics, PlayQueue, Playlist, RawAlbumList,
    RawLibraryItems, RawPlaylists, RawSongList, ScanStatus, Song, StructuredLyrics, with_raw_json,
};

const API_VERSION: &str = "1.16.1";
const CLIENT_NAME: &str = "ParloPlayer";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Library pages (e.g. 500 songs) are much larger than a ping.
const LIBRARY_TIMEOUT: Duration = Duration::from_secs(60);
/// One file of "Save to computer" (spec 8.2): a long FLAC on a slow link.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60 * 60);
/// Largest page of `getAlbumList2` (the API maximum).
const ALBUM_LIST_PAGE: u32 = 500;
/// OpenSubsonic extension: every endpoint accepts its parameters as a POST form.
const FORM_POST: &str = "formPost";
/// OpenSubsonic extension: `getLyricsBySongId`, lyrics with timings.
const SONG_LYRICS: &str = "songLyrics";

/// One page of `search3`. A count of 0 skips that item type on Navidrome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchPage {
    pub artist_count: u32,
    pub artist_offset: u32,
    pub album_count: u32,
    pub album_offset: u32,
    pub song_count: u32,
    pub song_offset: u32,
}

impl SearchPage {
    pub fn artists(count: u32, offset: u32) -> Self {
        Self { artist_count: count, artist_offset: offset, ..Self::default() }
    }

    pub fn albums(count: u32, offset: u32) -> Self {
        Self { album_count: count, album_offset: offset, ..Self::default() }
    }

    pub fn songs(count: u32, offset: u32) -> Self {
        Self { song_count: count, song_offset: offset, ..Self::default() }
    }
}

/// What a `star` / `unstar` call is about: each kind has its own parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarItem {
    Song,
    Album,
    Artist,
}

impl StarItem {
    fn param(self) -> &'static str {
        match self {
            StarItem::Song => "id",
            StarItem::Album => "albumId",
            StarItem::Artist => "artistId",
        }
    }
}

/// Orders of `getAlbumList2` used by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlbumListType {
    /// Most played.
    Frequent,
}

impl AlbumListType {
    fn as_str(self) -> &'static str {
        match self {
            AlbumListType::Frequent => "frequent",
        }
    }
}

/// One `updatePlaylist` call: the fields to change (`None`: kept), the songs
/// to append and the positions (0-based, before the change) to remove.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaylistUpdate {
    pub name: Option<String>,
    pub comment: Option<String>,
    pub public: Option<bool>,
    pub add: Vec<String>,
    pub remove: Vec<usize>,
}

/// Server details returned by `ping`, sent to the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: bool,
}

pub struct SubsonicClient {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
    /// Names of the OpenSubsonic extensions the server announces, asked
    /// once per client.
    extensions: OnceLock<Vec<String>>,
    /// `type` of the first successful `ping` (e.g. "navidrome").
    server_type: OnceLock<Option<String>>,
}

/// How a request sends its parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    /// In the URL query (credentials included).
    Get,
    /// In a form body, so long lists fit and the URL has no credentials.
    Post,
}

impl SubsonicClient {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, SubsonicError> {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self {
            base_url,
            username: username.into(),
            password: password.into(),
            http,
            extensions: OnceLock::new(),
            server_type: OnceLock::new(),
        })
    }

    /// Server URL without trailing slash.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// `GET /rest/ping` — checks that the server is reachable and the
    /// credentials are valid.
    pub async fn ping(&self) -> Result<ServerInfo, SubsonicError> {
        let body = self.get("ping", &[], REQUEST_TIMEOUT).await?;
        let _ = self.server_type.set(body.server_type.clone());
        Ok(ServerInfo {
            api_version: body.version,
            server_type: body.server_type,
            server_version: body.server_version,
            open_subsonic: body.open_subsonic.unwrap_or(false),
        })
    }

    /// The server's `type` (OpenSubsonic), from the first `ping`; asked now
    /// if there was none yet.
    pub async fn server_type(&self) -> Result<Option<String>, SubsonicError> {
        if let Some(kind) = self.server_type.get() {
            return Ok(kind.clone());
        }
        Ok(self.ping().await?.server_type)
    }

    /// `GET /rest/getScanStatus`.
    pub async fn get_scan_status(&self) -> Result<ScanStatus, SubsonicError> {
        let mut body = self.get("getScanStatus", &[], REQUEST_TIMEOUT).await?;
        body.take("scanStatus")
    }

    /// `GET /rest/search3` with an empty query, which OpenSubsonic servers
    /// answer with the whole library (paged).
    pub async fn search3(&self, page: SearchPage) -> Result<LibraryItems, SubsonicError> {
        self.search3_in(page, None).await
    }

    /// `search3` in one music folder only (`None`: every folder).
    pub async fn search3_in(&self, page: SearchPage, folder: Option<&str>) -> Result<LibraryItems, SubsonicError> {
        let mut params = vec![
            ("query", "\"\"".to_string()),
            ("artistCount", page.artist_count.to_string()),
            ("artistOffset", page.artist_offset.to_string()),
            ("albumCount", page.album_count.to_string()),
            ("albumOffset", page.album_offset.to_string()),
            ("songCount", page.song_count.to_string()),
            ("songOffset", page.song_offset.to_string()),
        ];
        params.extend(folder.map(|id| ("musicFolderId", id.to_string())));
        let mut body = self.get("search3", &params, LIBRARY_TIMEOUT).await?;
        body.take::<RawLibraryItems>("searchResult3")?.try_into()
    }

    /// `GET /rest/getMusicFolders` — the server's music folders (libraries).
    pub async fn get_music_folders(&self) -> Result<Vec<MusicFolder>, SubsonicError> {
        let mut body = self.get("getMusicFolders", &[], REQUEST_TIMEOUT).await?;
        Ok(body.take::<MusicFolders>("musicFolders")?.music_folder)
    }

    /// `GET /rest/getAlbumList2` — `size` albums in that order, after the
    /// first `offset` ones. Servers send at most 500 per request.
    pub async fn get_album_list2(
        &self,
        kind: AlbumListType,
        size: u32,
        offset: u32,
    ) -> Result<Vec<Album>, SubsonicError> {
        let params = [
            ("type", kind.as_str().to_string()),
            ("size", size.to_string()),
            ("offset", offset.to_string()),
        ];
        let mut body = self.get("getAlbumList2", &params, REQUEST_TIMEOUT).await?;
        with_raw_json(body.take::<RawAlbumList>("albumList2")?.album)
    }

    /// Every album of a `getAlbumList2` order, page by page.
    pub async fn get_all_album_list2(&self, kind: AlbumListType) -> Result<Vec<Album>, SubsonicError> {
        let mut albums = Vec::new();
        loop {
            let page = self.get_album_list2(kind, ALBUM_LIST_PAGE, albums.len() as u32).await?;
            let last = page.len() < ALBUM_LIST_PAGE as usize;
            albums.extend(page);
            if last {
                return Ok(albums);
            }
        }
    }

    /// `GET /rest/getStarred2` — starred artists, albums and songs.
    pub async fn get_starred2(&self) -> Result<LibraryItems, SubsonicError> {
        self.get_starred2_in(None).await
    }

    /// `getStarred2` in one music folder only (`None`: every folder).
    pub async fn get_starred2_in(&self, folder: Option<&str>) -> Result<LibraryItems, SubsonicError> {
        let params: Vec<(&str, String)> = folder.map(|id| ("musicFolderId", id.to_string())).into_iter().collect();
        let mut body = self.get("getStarred2", &params, LIBRARY_TIMEOUT).await?;
        body.take::<RawLibraryItems>("starred2")?.try_into()
    }

    /// URL of `GET /rest/stream` for a song, with a fresh salt, in the
    /// quality chosen (spec 6.10). Contains credentials.
    pub fn stream_url(&self, song_id: &str, quality: &StreamQuality) -> Result<SecretUrl, SubsonicError> {
        let mut url = reqwest::Url::parse(&format!("{}/rest/stream", self.base_url))
            .map_err(|e| SubsonicError::InvalidUrl(e.to_string()))?;
        url.query_pairs_mut()
            .append_pair("id", song_id)
            .extend_pairs(stream_params(quality))
            .extend_pairs(self.auth_params(&generate_salt()));
        Ok(SecretUrl::new(url))
    }

    /// `GET /rest/scrobble`. With `submission=false` it only says "now playing";
    /// with `true` it records plays (spec 5.7). Each play is a song ID and the
    /// time it started, in Unix milliseconds; several can go in one request.
    pub async fn scrobble(&self, plays: &[(String, i64)], submission: bool) -> Result<(), SubsonicError> {
        if plays.is_empty() {
            return Ok(());
        }
        self.get("scrobble", &scrobble_params(plays, submission), REQUEST_TIMEOUT).await?;
        Ok(())
    }

    /// `GET /rest/getPlaylists` — all playlists visible to the user.
    pub async fn get_playlists(&self) -> Result<Vec<Playlist>, SubsonicError> {
        let mut body = self.get("getPlaylists", &[], LIBRARY_TIMEOUT).await?;
        with_raw_json(body.take::<RawPlaylists>("playlists")?.playlist)
    }

    /// `star` / `unstar` songs, albums or artists (spec 2: favorites live on
    /// the server).
    pub async fn set_starred(&self, item: StarItem, ids: &[String], starred: bool) -> Result<(), SubsonicError> {
        if ids.is_empty() {
            return Ok(());
        }
        let endpoint = if starred { "star" } else { "unstar" };
        self.get(endpoint, &star_params(item, ids), REQUEST_TIMEOUT).await?;
        Ok(())
    }

    /// `createPlaylist` — a new playlist with these songs, in this order (e.g.
    /// the queue, spec 9). Sent by POST when the server allows it: the list
    /// can be long. Returns the playlist the server made, if it says.
    pub async fn create_playlist(&self, name: &str, song_ids: &[String]) -> Result<Option<Playlist>, SubsonicError> {
        let params = create_playlist_params(name, song_ids);
        let mut body = self.request_large("createPlaylist", &params, LIBRARY_TIMEOUT).await?;
        let mut playlist: Value = body.take("playlist")?;
        if playlist.is_null() {
            return Ok(None);
        }
        if let Some(fields) = playlist.as_object_mut() {
            fields.remove("entry");
        }
        Ok(with_raw_json::<Playlist>(vec![playlist])?.pop())
    }

    /// `updatePlaylist` (spec 8.1, 8.2): rename, describe, make public or
    /// private, append songs and remove positions, in one call. Sent by POST
    /// when the server allows it, as the songs can be many.
    pub async fn update_playlist(&self, playlist_id: &str, update: &PlaylistUpdate) -> Result<(), SubsonicError> {
        let params = update_playlist_params(playlist_id, update);
        self.request_large("updatePlaylist", &params, LIBRARY_TIMEOUT).await?;
        Ok(())
    }

    /// `deletePlaylist`.
    pub async fn delete_playlist(&self, playlist_id: &str) -> Result<(), SubsonicError> {
        self.get("deletePlaylist", &[("id", playlist_id.to_string())], REQUEST_TIMEOUT).await?;
        Ok(())
    }

    /// `GET /rest/getPlaylist` — a playlist and its songs in the saved order.
    pub async fn get_playlist(&self, playlist_id: &str) -> Result<(Playlist, Vec<Song>), SubsonicError> {
        let mut body = self.get("getPlaylist", &[("id", playlist_id.to_string())], LIBRARY_TIMEOUT).await?;
        let mut playlist: Value = body.take("playlist")?;
        let entries = match playlist.as_object_mut().and_then(|p| p.remove("entry")) {
            Some(Value::Array(entries)) => entries,
            _ => Vec::new(),
        };
        let playlist = with_raw_json::<Playlist>(vec![playlist])?
            .pop()
            .ok_or_else(|| SubsonicError::InvalidResponse("getPlaylist: no playlist".into()))?;
        Ok((playlist, with_raw_json(entries)?))
    }

    /// `GET /rest/getCoverArt` — the image bytes, resized by the server to
    /// `size` pixels. On failure the server answers JSON instead of an image.
    pub async fn get_cover_art(&self, cover_id: &str, size: u32) -> Result<Vec<u8>, SubsonicError> {
        self.cover_art(cover_id, Some(size)).await
    }

    /// The cover at its original size ("Save to computer" of an album, spec 8.2).
    pub async fn get_full_cover_art(&self, cover_id: &str) -> Result<Vec<u8>, SubsonicError> {
        self.cover_art(cover_id, None).await
    }

    async fn cover_art(&self, cover_id: &str, size: Option<u32>) -> Result<Vec<u8>, SubsonicError> {
        let mut params = vec![("id", cover_id.to_string())];
        params.extend(size.map(|size| ("size", size.to_string())));
        let response = self.binary("getCoverArt", &params, LIBRARY_TIMEOUT).await?;
        Ok(response.bytes().await?.to_vec())
    }

    /// `GET /rest/download` — the original file, never converted (spec 8.2).
    /// The caller reads the body chunk by chunk. A refused download (no
    /// permission) is `Api { code: 50 }` or an HTTP 401 / 403 status.
    pub async fn download(&self, song_id: &str) -> Result<reqwest::Response, SubsonicError> {
        self.binary("download", &[("id", song_id.to_string())], DOWNLOAD_TIMEOUT).await
    }

    /// An endpoint that answers a file. On failure the server answers JSON
    /// (or XML) instead: that becomes the error.
    async fn binary(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
        timeout: Duration,
    ) -> Result<reqwest::Response, SubsonicError> {
        let url = format!("{}/rest/{endpoint}", self.base_url);
        let auth = self.auth_params(&generate_salt());
        let response = self
            .http
            .get(url)
            .query(&auth)
            .query(params)
            .timeout(timeout)
            .send()
            .await?
            .error_for_status()?;

        let is_json = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.contains("json") || v.contains("xml"));
        if is_json {
            parse_response(&response.text().await?)?;
            return Err(SubsonicError::InvalidResponse(format!("{endpoint}: no file")));
        }
        Ok(response)
    }

    /// `GET /rest/getOpenSubsonicExtensions`.
    pub async fn get_open_subsonic_extensions(&self) -> Result<Vec<Extension>, SubsonicError> {
        let mut body = self.get("getOpenSubsonicExtensions", &[], REQUEST_TIMEOUT).await?;
        body.take("openSubsonicExtensions")
    }

    /// Whether the server announces an OpenSubsonic extension. A server
    /// that does not know the endpoint simply has none. A network failure is
    /// not remembered, so the next call asks again.
    async fn supports_extension(&self, name: &str) -> bool {
        if let Some(names) = self.extensions.get() {
            return names.iter().any(|n| n == name);
        }
        let names = match self.get_open_subsonic_extensions().await {
            Ok(extensions) => extensions.into_iter().map(|e| e.name).collect(),
            Err(SubsonicError::Http(e)) if !e.is_status() => return false,
            Err(_) => Vec::new(),
        };
        self.extensions.get_or_init(|| names).iter().any(|n| n == name)
    }

    /// Whether the server takes POST forms (`formPost`).
    pub async fn supports_form_post(&self) -> bool {
        self.supports_extension(FORM_POST).await
    }

    /// `getLyricsBySongId` (OpenSubsonic `songLyrics`): the song's lyrics,
    /// with timings when synced. Empty when the server has none, or does not
    /// have the extension.
    pub async fn get_lyrics_by_song_id(&self, song_id: &str) -> Result<Vec<StructuredLyrics>, SubsonicError> {
        if !self.supports_extension(SONG_LYRICS).await {
            return Ok(Vec::new());
        }
        let mut body = self.get("getLyricsBySongId", &[("id", song_id.to_string())], REQUEST_TIMEOUT).await?;
        Ok(body.take::<LyricsList>("lyricsList")?.structured_lyrics)
    }

    /// `getLyrics` — plain lyrics found by artist and title, if any.
    pub async fn get_lyrics(&self, artist: &str, title: &str) -> Result<Option<String>, SubsonicError> {
        let params = [("artist", artist.to_string()), ("title", title.to_string())];
        let mut body = self.get("getLyrics", &params, REQUEST_TIMEOUT).await?;
        let lyrics: PlainLyrics = body.take("lyrics")?;
        Ok(lyrics.value.filter(|text| !text.trim().is_empty()))
    }

    /// `getSimilarSongs2` — songs like this artist's (Last.fm on Navidrome).
    /// Navidrome also takes a song or album ID.
    pub async fn get_similar_songs2(&self, id: &str, count: u32) -> Result<Vec<Song>, SubsonicError> {
        let params = [("id", id.to_string()), ("count", count.to_string())];
        let mut body = self.get("getSimilarSongs2", &params, LIBRARY_TIMEOUT).await?;
        with_raw_json(body.take::<RawSongList>("similarSongs2")?.song)
    }

    /// `getArtistInfo2` — biography and similar artists (Last.fm on
    /// Navidrome). Only artists the server has are asked for.
    pub async fn get_artist_info2(&self, artist_id: &str) -> Result<ArtistInfo, SubsonicError> {
        let params = [("id", artist_id.to_string()), ("includeNotPresent", "false".to_string())];
        let mut body = self.get("getArtistInfo2", &params, REQUEST_TIMEOUT).await?;
        let info: Value = body.take("artistInfo2")?;
        with_raw_json::<ArtistInfo>(vec![info])?
            .pop()
            .ok_or_else(|| SubsonicError::InvalidResponse("getArtistInfo2: no info".into()))
    }

    /// `getTopSongs` — the artist's most popular songs, by name (Last.fm on
    /// Navidrome; empty without it).
    pub async fn get_top_songs(&self, artist_name: &str, count: u32) -> Result<Vec<Song>, SubsonicError> {
        let params = [("artist", artist_name.to_string()), ("count", count.to_string())];
        let mut body = self.get("getTopSongs", &params, REQUEST_TIMEOUT).await?;
        with_raw_json(body.take::<RawSongList>("topSongs")?.song)
    }

    /// `getPlayQueue` — the queue saved on the server, `None` if there is none.
    pub async fn get_play_queue(&self) -> Result<Option<PlayQueue>, SubsonicError> {
        let mut body = self.get("getPlayQueue", &[], REQUEST_TIMEOUT).await?;
        let queue: PlayQueue = body.take("playQueue")?;
        Ok((!queue.entry.is_empty()).then_some(queue))
    }

    /// `savePlayQueue` — songs in playback order, the current one and the
    /// position in milliseconds (spec 5.6). Sent by POST when the server
    /// allows it, because the queue can have thousands of songs.
    pub async fn save_play_queue(
        &self,
        song_ids: &[String],
        current: &str,
        position_ms: i64,
        timeout: Duration,
    ) -> Result<(), SubsonicError> {
        let params = save_play_queue_params(song_ids, current, position_ms);
        self.request_large("savePlayQueue", &params, timeout).await?;
        Ok(())
    }

    /// A request whose parameters can be too many for a URL: POST when the
    /// server announces `formPost`, GET otherwise.
    async fn request_large(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
        timeout: Duration,
    ) -> Result<ResponseBody, SubsonicError> {
        let method = if self.supports_form_post().await { Method::Post } else { Method::Get };
        self.request(method, endpoint, params, timeout).await
    }

    async fn get(
        &self,
        endpoint: &str,
        params: &[(&str, String)],
        timeout: Duration,
    ) -> Result<ResponseBody, SubsonicError> {
        self.request(Method::Get, endpoint, params, timeout).await
    }

    async fn request(
        &self,
        method: Method,
        endpoint: &str,
        params: &[(&str, String)],
        timeout: Duration,
    ) -> Result<ResponseBody, SubsonicError> {
        let url = format!("{}/rest/{endpoint}", self.base_url);
        let auth = self.auth_params(&generate_salt());

        let request = match method {
            Method::Get => self.http.get(url).query(&auth).query(params),
            Method::Post => {
                let form: Vec<(&str, &str)> = auth
                    .iter()
                    .chain(params)
                    .map(|(k, v)| (*k, v.as_str()))
                    .collect();
                self.http.post(url).form(&form)
            }
        };
        let text = request
            .timeout(timeout)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        parse_response(&text)
    }

    /// Query parameters sent with every request. A fresh salt must be passed
    /// each time.
    fn auth_params(&self, salt: &str) -> [(&'static str, String); 6] {
        [
            ("u", self.username.clone()),
            ("t", make_token(&self.password, salt)),
            ("s", salt.to_string()),
            ("v", API_VERSION.to_string()),
            ("c", CLIENT_NAME.to_string()),
            ("f", "json".to_string()),
        ]
    }
}

/// `id` and `time` repeat once per play, in matching order.
fn scrobble_params(plays: &[(String, i64)], submission: bool) -> Vec<(&'static str, String)> {
    let mut params = vec![("submission", submission.to_string())];
    for (id, time_ms) in plays {
        params.push(("id", id.clone()));
        params.push(("time", time_ms.to_string()));
    }
    params
}

/// `format=raw` asks for the original file, whatever the server's own
/// transcoding settings; otherwise the format and the highest bitrate.
fn stream_params(quality: &StreamQuality) -> Vec<(&'static str, String)> {
    match quality.mode {
        StreamMode::Original => vec![("format", "raw".to_string())],
        StreamMode::Converted => vec![
            ("format", quality.format.as_str().to_string()),
            ("maxBitRate", quality.max_bit_rate.to_string()),
        ],
    }
}

/// The item's parameter repeats once per ID.
fn star_params(item: StarItem, ids: &[String]) -> Vec<(&'static str, String)> {
    ids.iter().map(|id| (item.param(), id.clone())).collect()
}

/// `songId` repeats once per song, in order.
fn create_playlist_params(name: &str, song_ids: &[String]) -> Vec<(&'static str, String)> {
    let mut params = vec![("name", name.to_string())];
    params.extend(song_ids.iter().map(|id| ("songId", id.clone())));
    params
}

/// `songIdToAdd` and `songIndexToRemove` repeat once per item.
fn update_playlist_params(playlist_id: &str, update: &PlaylistUpdate) -> Vec<(&'static str, String)> {
    let mut params = vec![("playlistId", playlist_id.to_string())];
    params.extend(update.name.as_ref().map(|name| ("name", name.clone())));
    params.extend(update.comment.as_ref().map(|comment| ("comment", comment.clone())));
    params.extend(update.public.map(|public| ("public", public.to_string())));
    params.extend(update.add.iter().map(|id| ("songIdToAdd", id.clone())));
    params.extend(update.remove.iter().map(|index| ("songIndexToRemove", index.to_string())));
    params
}

/// `id` repeats once per song, in playback order.
fn save_play_queue_params(song_ids: &[String], current: &str, position_ms: i64) -> Vec<(&'static str, String)> {
    let mut params: Vec<(&str, String)> = song_ids.iter().map(|id| ("id", id.clone())).collect();
    params.push(("current", current.to_string()));
    params.push(("position", position_ms.max(0).to_string()));
    params
}

// Hand-written so the password never ends up in logs or error messages.
impl fmt::Debug for SubsonicClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubsonicClient")
            .field("base_url", &self.base_url)
            .field("username", &self.username)
            .field("password", &"<hidden>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::StreamFormat;

    fn client() -> SubsonicClient {
        SubsonicClient::new("https://music.example.com/", "alice", "sesame").unwrap()
    }

    #[test]
    fn trailing_slash_is_removed_from_base_url() {
        assert_eq!(client().base_url, "https://music.example.com");
    }

    #[test]
    fn auth_params_use_given_salt() {
        let params = client().auth_params("c19b2d");
        let get = |key: &str| {
            params
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.as_str())
        };

        assert_eq!(get("u"), Some("alice"));
        assert_eq!(get("t"), Some("26719a1196d2a940705a59634eb18eab"));
        assert_eq!(get("s"), Some("c19b2d"));
        assert_eq!(get("v"), Some("1.16.1"));
        assert_eq!(get("c"), Some("ParloPlayer"));
        assert_eq!(get("f"), Some("json"));
        assert_eq!(get("p"), None, "plain password must never be sent");
    }

    #[test]
    fn stream_url_has_song_and_token_but_no_password() {
        let url = client().stream_url("song-42", &StreamQuality::default()).unwrap();
        let full = reqwest::Url::parse(url.expose()).unwrap();
        let pairs: Vec<(String, String)> = full.query_pairs().into_owned().collect();
        let get = |key: &str| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());

        assert_eq!(full.path(), "/rest/stream");
        assert_eq!(get("id"), Some("song-42"));
        assert_eq!(get("u"), Some("alice"));
        assert!(get("t").is_some() && get("s").is_some());
        assert_eq!(get("p"), None, "plain password must never be sent");
        assert!(!url.expose().contains("sesame"));
        assert!(!url.to_string().contains(get("t").unwrap()), "Display must hide the token");
        assert_eq!(get("format"), Some("raw"), "original: never converted");
    }

    #[test]
    fn converted_streams_ask_for_the_format_and_bitrate() {
        let quality = StreamQuality { mode: StreamMode::Converted, format: StreamFormat::Mp3, max_bit_rate: 256 };
        assert_eq!(
            stream_params(&quality),
            [("format", "mp3".to_string()), ("maxBitRate", "256".to_string())]
        );
        assert_eq!(stream_params(&StreamQuality::default()), [("format", "raw".to_string())]);
    }

    #[test]
    fn scrobble_params_pair_ids_with_times() {
        let plays = [("s1".to_string(), 1000), ("s2".to_string(), 2000)];
        assert_eq!(
            scrobble_params(&plays, true),
            [
                ("submission", "true".to_string()),
                ("id", "s1".to_string()),
                ("time", "1000".to_string()),
                ("id", "s2".to_string()),
                ("time", "2000".to_string()),
            ]
        );
        assert_eq!(scrobble_params(&plays[..1], false)[0], ("submission", "false".to_string()));
    }

    #[test]
    fn star_params_repeat_the_id_of_each_kind() {
        let ids = ["s1".to_string(), "s2".to_string()];
        assert_eq!(star_params(StarItem::Song, &ids), [("id", "s1".to_string()), ("id", "s2".to_string())]);
        assert_eq!(star_params(StarItem::Album, &ids[..1]), [("albumId", "s1".to_string())]);
        assert_eq!(star_params(StarItem::Artist, &ids[..1]), [("artistId", "s1".to_string())]);
    }

    #[test]
    fn create_playlist_params_keep_the_songs_in_order() {
        let ids = ["s2".to_string(), "s1".to_string(), "s2".to_string()];
        assert_eq!(
            create_playlist_params("Albums, 10/18/2026", &ids),
            [
                ("name", "Albums, 10/18/2026".to_string()),
                ("songId", "s2".to_string()),
                ("songId", "s1".to_string()),
                ("songId", "s2".to_string()),
            ]
        );
    }

    #[test]
    fn update_playlist_params_send_only_what_changes() {
        let update = PlaylistUpdate {
            name: Some("Road trip".into()),
            public: Some(false),
            add: vec!["s1".into(), "s2".into()],
            remove: vec![3, 0],
            ..PlaylistUpdate::default()
        };
        assert_eq!(
            update_playlist_params("p1", &update),
            [
                ("playlistId", "p1".to_string()),
                ("name", "Road trip".to_string()),
                ("public", "false".to_string()),
                ("songIdToAdd", "s1".to_string()),
                ("songIdToAdd", "s2".to_string()),
                ("songIndexToRemove", "3".to_string()),
                ("songIndexToRemove", "0".to_string()),
            ]
        );
        let empty_comment = PlaylistUpdate { comment: Some(String::new()), ..PlaylistUpdate::default() };
        assert_eq!(update_playlist_params("p1", &empty_comment)[1], ("comment", String::new()));
    }

    #[test]
    fn save_play_queue_params_keep_the_playback_order() {
        let ids = ["s3".to_string(), "s1".to_string(), "s2".to_string()];
        assert_eq!(
            save_play_queue_params(&ids, "s1", 42_500),
            [
                ("id", "s3".to_string()),
                ("id", "s1".to_string()),
                ("id", "s2".to_string()),
                ("current", "s1".to_string()),
                ("position", "42500".to_string()),
            ]
        );
        assert_eq!(save_play_queue_params(&ids, "s1", -5)[4], ("position", "0".to_string()));
    }

    #[test]
    fn each_stream_url_uses_a_new_salt() {
        let a = client().stream_url("s1", &StreamQuality::default()).unwrap();
        let b = client().stream_url("s1", &StreamQuality::default()).unwrap();
        assert_ne!(a.expose(), b.expose());
    }

    #[test]
    fn debug_output_hides_password() {
        let debug = format!("{:?}", client());
        assert!(!debug.contains("sesame"));
        assert!(debug.contains("<hidden>"));
    }
}
