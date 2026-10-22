//! Client for the Subsonic / OpenSubsonic API (target server: Navidrome).

mod auth;
mod client;
mod error;
mod response;
mod secret_url;
mod types;

pub use client::{AlbumListType, PlaylistUpdate, SearchPage, ServerInfo, StarItem, SubsonicClient};
pub use error::SubsonicError;
pub use secret_url::SecretUrl;
pub use types::{
    Album, Artist, ArtistInfo, Extension, Genre, LibraryItems, MusicFolder, LyricsLine, PlayQueue, PlayQueueEntry, Playlist, ScanStatus, SimilarArtist,
    Song, StructuredLyrics,
};
