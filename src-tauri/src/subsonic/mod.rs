//! Client for the Subsonic / OpenSubsonic API (target server: Navidrome).

mod auth;
mod client;
mod error;
mod response;
mod secret_url;
mod types;

pub use client::{AlbumListType, SearchPage, ServerInfo, SubsonicClient};
pub use error::SubsonicError;
pub use secret_url::SecretUrl;
pub use types::{Album, Artist, Genre, LibraryItems, PlayQueue, PlayQueueEntry, Playlist, ScanStatus, Song};
