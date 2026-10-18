//! Client for the Subsonic / OpenSubsonic API (target server: Navidrome).

mod auth;
mod client;
mod error;
mod response;
mod types;

pub use client::{SearchPage, ServerInfo, SubsonicClient};
pub use error::SubsonicError;
pub use types::{Album, Artist, Genre, LibraryItems, Playlist, ScanStatus, Song};
