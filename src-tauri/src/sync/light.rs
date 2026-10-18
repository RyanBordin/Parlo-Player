//! Light sync: starred items and playlists, which change without a new server
//! scan (e.g. starring a song in another app).

use crate::db::library;
use crate::subsonic::SubsonicClient;

use super::{Phase, SyncDb, SyncError, SyncProgress, now};

pub(super) async fn run(
    client: &SubsonicClient,
    db: &SyncDb,
    full: bool,
    progress: &(dyn Fn(SyncProgress) + Send + Sync),
) -> Result<(), SyncError> {
    progress(SyncProgress { phase: Phase::Starred, done: 0, total: None, full });
    let starred = client.get_starred2().await?;

    progress(SyncProgress { phase: Phase::Playlists, done: 0, total: None, full });
    let playlists = client.get_playlists().await?;

    db.call(move |conn| library::apply_light_sync(conn, &starred, &playlists, now()))
        .await
}
