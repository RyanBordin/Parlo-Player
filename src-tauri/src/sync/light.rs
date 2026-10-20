//! Light sync: starred items and playlists, which change without a new server
//! scan (e.g. starring a song in another app).

use std::path::Path;

use crate::db::library;
use crate::playback::send_pending_scrobbles;
use crate::subsonic::{LibraryItems, SubsonicClient};

use super::{Phase, SyncDb, SyncError, SyncProgress, now};

pub(super) async fn run(
    client: &SubsonicClient,
    db: &SyncDb,
    db_path: &Path,
    full: bool,
    folders: Option<&[String]>,
    progress: &(dyn Fn(SyncProgress) + Send + Sync),
) -> Result<(), SyncError> {
    // Plays that could not be scrobbled before (spec 5.7). A failure here
    // keeps them pending and must not stop the sync.
    let _ = send_pending_scrobbles(client, db_path).await;

    progress(SyncProgress { phase: Phase::Starred, done: 0, total: None, full });
    let starred = match folders {
        None => client.get_starred2().await?,
        // Only the visible folders: a hidden one's favorites stay out.
        Some(ids) => {
            let mut all = LibraryItems::default();
            for id in ids {
                merge(&mut all, client.get_starred2_in(Some(id)).await?);
            }
            all
        }
    };

    progress(SyncProgress { phase: Phase::Playlists, done: 0, total: None, full });
    let playlists = client.get_playlists().await?;

    db.call(move |conn| library::apply_light_sync(conn, &starred, &playlists, now()))
        .await
}

/// Adds `more` to `all`, each item once (an artist can be in several folders).
fn merge(all: &mut LibraryItems, more: LibraryItems) {
    for artist in more.artists {
        if !all.artists.iter().any(|a| a.id == artist.id) {
            all.artists.push(artist);
        }
    }
    for album in more.albums {
        if !all.albums.iter().any(|a| a.id == album.id) {
            all.albums.push(album);
        }
    }
    for song in more.songs {
        if !all.songs.iter().any(|s| s.id == song.id) {
            all.songs.push(song);
        }
    }
}
