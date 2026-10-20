//! Cover art for the interface (spec 12.4), through the app's own `cover`
//! protocol: `cover://localhost/?id=<coverArt>&size=<px>` on Linux,
//! `http://cover.localhost/?id=...` on Windows.
//!
//! The backend fetches the image with `getCoverArt`, keeps it in a disk cache
//! and answers with the bytes. The interface never sees a URL with
//! credentials, and covers already loaded keep working offline.
//!
//! `&fallback=<coverArt>` is for artists (spec 6.6): Navidrome answers the
//! photo of an artist that has none with a generic image, so the app compares
//! it with that image (the one the server gives for an ID that does not
//! exist) and uses the fallback cover instead.

mod cache;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use md5::{Digest, Md5};
use tauri::http::{Response, StatusCode, Uri, header};
use tauri::{AppHandle, Manager, Runtime};

pub use cache::CoverCache;

use crate::state::AppState;
use crate::subsonic::{SubsonicClient, SubsonicError};

pub const SCHEME: &str = "cover";

/// Sizes the server is asked for: a request is rounded up to one of them, so
/// a cover is not stored in dozens of nearly equal sizes.
const SIZES: [u32; 5] = [64, 128, 256, 512, 1024];
const DEFAULT_SIZE: u32 = 256;

/// A cover ID no server has: what it returns is the generic artist image.
const MISSING_ARTIST_ID: &str = "ar-parlo-missing-placeholder";

/// The cover cache and the server's generic images, kept for the app's life.
pub struct Covers {
    cache: CoverCache,
    /// Generic artist image per library and size.
    placeholders: Mutex<HashMap<String, Arc<Vec<u8>>>>,
}

impl Covers {
    pub fn new(cache: CoverCache) -> Self {
        Self { cache, placeholders: Mutex::new(HashMap::new()) }
    }

    /// The cover cache, for its settings (spec 6.10: size, limit, clear).
    pub fn cache(&self) -> &CoverCache {
        &self.cache
    }

    /// Whether `bytes` is the server's generic artist image at this size.
    /// When the server gives no such image, nothing counts as one.
    async fn is_placeholder(&self, client: &SubsonicClient, library: &Path, size: u32, bytes: &[u8]) -> bool {
        let key = format!("{}|{size}", library.display());
        let known = self.lock_placeholders().get(&key).cloned();
        let placeholder = match known {
            Some(placeholder) => placeholder,
            None => match client.get_cover_art(MISSING_ARTIST_ID, size).await {
                Ok(image) if image_type(&image).is_some() => {
                    let image = Arc::new(image);
                    self.lock_placeholders().insert(key, Arc::clone(&image));
                    image
                }
                _ => return false,
            },
        };
        placeholder.as_slice() == bytes
    }

    /// Whether the server has a real photo for this artist cover, not its
    /// generic image (artist page banner, spec 6.7). `None` when the server
    /// cannot be reached.
    pub async fn is_artist_photo(&self, client: &SubsonicClient, library: &Path, cover_id: &str) -> Option<bool> {
        // The smallest size: only compared, never shown.
        let size = SIZES[0];
        match client.get_cover_art(cover_id, size).await {
            Ok(bytes) if image_type(&bytes).is_some() => {
                Some(!self.is_placeholder(client, library, size, &bytes).await)
            }
            Err(SubsonicError::Http(e)) if !e.is_status() => None,
            _ => Some(false),
        }
    }

    fn lock_placeholders(&self) -> MutexGuard<'_, HashMap<String, Arc<Vec<u8>>>> {
        self.placeholders.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// A parsed `cover` request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverRequest {
    pub id: String,
    pub size: u32,
    /// Shown instead when `id` gives the server's generic image or fails.
    pub fallback: Option<String>,
}

impl CoverRequest {
    /// Reads `?id=...&size=...` from the request URL.
    pub fn parse(uri: &str) -> Option<Self> {
        let url = reqwest::Url::parse(uri).ok()?;
        let mut id = None;
        let mut fallback = None;
        let mut size = DEFAULT_SIZE;
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "id" => id = Some(value.into_owned()),
                "fallback" => fallback = Some(value.into_owned()),
                "size" => size = value.parse().unwrap_or(DEFAULT_SIZE),
                _ => {}
            }
        }
        let id = id.filter(|id| !id.is_empty())?;
        let fallback = fallback.filter(|f| !f.is_empty() && *f != id);
        Some(Self { id, size: size_bucket(size), fallback })
    }
}

/// The smallest stored size that is at least `requested` (largest if none).
fn size_bucket(requested: u32) -> u32 {
    SIZES.iter().copied().find(|&s| s >= requested).unwrap_or(SIZES[SIZES.len() - 1])
}

/// File name in the cache: a hash of the library (server + user), the cover
/// ID and the size. Never built from server text, so no path tricks.
fn cache_key(library: &Path, request: &CoverRequest) -> String {
    let library = library.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    let fallback = request.fallback.as_deref().unwrap_or_default();
    let digest = Md5::digest(format!("{library}\n{}\n{}\n{fallback}", request.id, request.size).as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Image type from its first bytes; `None` if it is not an image.
fn image_type(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        _ => None,
    }
}

/// Registers the `cover` protocol and the cache it uses.
pub fn register<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.register_asynchronous_uri_scheme_protocol(SCHEME, |ctx, request, responder| {
        let app = ctx.app_handle().clone();
        let uri = request.uri().clone();
        tauri::async_runtime::spawn(async move {
            responder.respond(serve(&app, &uri).await);
        });
    })
}

async fn serve<R: Runtime>(app: &AppHandle<R>, uri: &Uri) -> Response<Vec<u8>> {
    let Some(request) = CoverRequest::parse(&uri.to_string()) else {
        return status(StatusCode::BAD_REQUEST);
    };
    let Some(library) = app.state::<Arc<AppState>>().active() else {
        return status(StatusCode::NOT_FOUND);
    };
    let covers = Arc::clone(&app.state::<Arc<Covers>>());
    let key = cache_key(&library.db_path, &request);

    let cached = {
        let (covers, key) = (Arc::clone(&covers), key.clone());
        tauri::async_runtime::spawn_blocking(move || covers.cache.get(&key)).await.ok().flatten()
    };
    if let Some(bytes) = cached {
        return image(bytes);
    }

    let client = &library.client;
    let fetched = match (client.get_cover_art(&request.id, request.size).await, &request.fallback) {
        (Ok(bytes), Some(fallback))
            if covers.is_placeholder(client, &library.db_path, request.size, &bytes).await =>
        {
            client.get_cover_art(fallback, request.size).await.or(Ok(bytes))
        }
        (Err(_), Some(fallback)) => client.get_cover_art(fallback, request.size).await,
        (result, _) => result,
    };

    // The server error is not passed on: the interface shows the placeholder.
    let Ok(bytes) = fetched else {
        return status(StatusCode::NOT_FOUND);
    };
    if image_type(&bytes).is_none() {
        return status(StatusCode::NOT_FOUND);
    }
    let stored = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = covers.cache.put(&key, &stored) {
            log::warn!("covers: could not cache a cover: {error}");
        }
    });
    image(bytes)
}

fn image(bytes: Vec<u8>) -> Response<Vec<u8>> {
    let content_type = image_type(&bytes).unwrap_or("application/octet-stream");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        // Covers of an ID do not change: the webview can keep them in memory.
        .header(header::CACHE_CONTROL, "max-age=86400")
        .body(bytes)
        .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = code;
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests_on_windows_and_linux() {
        let windows = CoverRequest::parse("http://cover.localhost/?id=al-1%2F2&size=300").unwrap();
        assert_eq!(windows, CoverRequest { id: "al-1/2".into(), size: 512, fallback: None });
        let linux = CoverRequest::parse("cover://localhost/?id=mf-9").unwrap();
        assert_eq!(linux, CoverRequest { id: "mf-9".into(), size: DEFAULT_SIZE, fallback: None });
    }

    #[test]
    fn artist_requests_carry_a_fallback_cover() {
        let request = CoverRequest::parse("http://cover.localhost/?id=ar-1&fallback=al-9&size=128").unwrap();
        assert_eq!(request.fallback.as_deref(), Some("al-9"));
        // Empty or the same as the ID: no fallback.
        let same = CoverRequest::parse("http://cover.localhost/?id=ar-1&fallback=ar-1").unwrap();
        assert_eq!(same.fallback, None);
        let empty = CoverRequest::parse("http://cover.localhost/?id=ar-1&fallback=").unwrap();
        assert_eq!(empty.fallback, None);
    }

    #[test]
    fn requests_without_an_id_are_rejected() {
        assert_eq!(CoverRequest::parse("http://cover.localhost/?size=64"), None);
        assert_eq!(CoverRequest::parse("http://cover.localhost/?id="), None);
        assert_eq!(CoverRequest::parse("not a url"), None);
    }

    #[test]
    fn sizes_round_up_to_the_stored_ones() {
        assert_eq!(size_bucket(1), 64);
        assert_eq!(size_bucket(64), 64);
        assert_eq!(size_bucket(65), 128);
        assert_eq!(size_bucket(300), 512);
        assert_eq!(size_bucket(5000), 1024);
    }

    #[test]
    fn cache_keys_are_hex_and_differ_per_library_id_and_size() {
        let request = CoverRequest { id: "../../etc/passwd".into(), size: 256, fallback: None };
        let key = cache_key(Path::new("/x/ryan@host-1.db"), &request);
        assert_eq!(key.len(), 32);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));

        let other_library = cache_key(Path::new("/x/ana@host-1.db"), &request);
        let other_size = cache_key(Path::new("/x/ryan@host-1.db"), &CoverRequest { size: 512, ..request.clone() });
        let with_fallback = cache_key(
            Path::new("/x/ryan@host-1.db"),
            &CoverRequest { fallback: Some("al-1".into()), ..request.clone() },
        );
        assert_ne!(key, other_library);
        assert_ne!(key, other_size);
        assert_ne!(key, with_fallback);
    }

    #[test]
    fn recognizes_image_types() {
        assert_eq!(image_type(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_type(b"\x89PNG\r\n"), Some("image/png"));
        assert_eq!(image_type(b"RIFF\0\0\0\0WEBPVP8"), Some("image/webp"));
        assert_eq!(image_type(b"GIF89a"), Some("image/gif"));
        assert_eq!(image_type(br#"{"subsonic-response":{}}"#), None);
    }
}
