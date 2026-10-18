//! Light sync: starred items and playlists, which change without a new server
//! scan (e.g. starring a song in another app).

use std::path::Path;

use crate::db::library;
use crate::playback::send_pending_scrobbles;
use crate::subsonic::SubsonicClient;

use super::{Phase, SyncDb, SyncError, SyncProgress, now};

pub(super) async fn run(
    client: &SubsonicClient,
    db: &SyncDb,
    db_path: &Path,
    full: bool,
    progress: &(dyn Fn(SyncProgress) + Send + Sync),
) -> Result<(), SyncError> {
    // Plays that could not be scrobbled before (spec 5.7). A failure here
    // keeps them pending and must not stop the sync.
    let _ = send_pending_scrobbles(client, db_path).await;

    progress(SyncProgress { phase: Phase::Starred, done: 0, total: None, full });
    let starred = client.get_starred2().await?;

    progress(SyncProgress { phase: Phase::Playlists, done: 0, total: None, full });
    let playlists = client.get_playlists().await?;

    db.call(move |conn| library::apply_light_sync(conn, &starred, &playlists, now()))
        .await
}
