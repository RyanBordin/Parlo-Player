//! Client for the Subsonic / OpenSubsonic API (target server: Navidrome).

mod auth;
mod client;
mod error;
mod response;

pub use client::{ServerInfo, SubsonicClient};
pub use error::SubsonicError;
