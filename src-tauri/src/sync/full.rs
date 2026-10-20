//! Full sync: the whole library through `search3` with an empty query,
//! which OpenSubsonic servers answer with every item, paged.

use crate::db::library;
use crate::subsonic::{LibraryItems, ScanStatus, SearchPage, SubsonicClient};

use super::{Phase, SyncDb, SyncError, SyncProgress, now};

/// Items per request. About 110 requests for 50,000 songs.
const PAGE_SIZE: u32 = 500;

#[derive(Debug, Clone, Copy)]
enum Kind {
    Artists,
    Albums,
    Songs,
}

impl Kind {
    fn page(self, offset: u32) -> SearchPage {
        match self {
            Kind::Artists => SearchPage::artists(PAGE_SIZE, offset),
            Kind::Albums => SearchPage::albums(PAGE_SIZE, offset),
            Kind::Songs => SearchPage::songs(PAGE_SIZE, offset),
        }
    }

    fn phase(self) -> Phase {
        match self {
            Kind::Artists => Phase::Artists,
            Kind::Albums => Phase::Albums,
            Kind::Songs => Phase::Songs,
        }
    }

    /// Number of items of this kind and the first ID, to detect repeated pages.
    fn received(self, items: &LibraryItems) -> (u32, Option<String>) {
        let (count, first) = match self {
            Kind::Artists => (items.artists.len(), items.artists.first().map(|a| a.id.clone())),
            Kind::Albums => (items.albums.len(), items.albums.first().map(|a| a.id.clone())),
            Kind::Songs => (items.songs.len(), items.songs.first().map(|s| s.id.clone())),
        };
        (count as u32, first)
    }
}

/// `folders`: only these music folders (`None`: all). `rebuild`: the library
/// is replaced from scratch (see `library::finish_rebuild`).
pub(super) async fn run(
    client: &SubsonicClient,
    db: &SyncDb,
    scan: &ScanStatus,
    folders: Option<&[String]>,
    rebuild: bool,
    progress: &(dyn Fn(SyncProgress) + Send + Sync),
) -> Result<(), SyncError> {
    db.call(|conn| library::begin_full_sync(conn)).await?;

    if let Err(error) = download(client, db, scan, folders, progress).await {
        // The real tables were never touched: just discard what was downloaded.
        let _ = db.call(|conn| library::drop_staging(conn)).await;
        return Err(error);
    }

    progress(SyncProgress { phase: Phase::Saving, done: 0, total: None, full: true });
    let last_scan = scan.last_scan.clone();
    db.call(move |conn| {
        if rebuild {
            library::finish_rebuild(conn, last_scan.as_deref(), now())
        } else {
            library::finish_full_sync(conn, last_scan.as_deref(), now())
        }
    })
    .await
}

async fn download(
    client: &SubsonicClient,
    db: &SyncDb,
    scan: &ScanStatus,
    folders: Option<&[String]>,
    progress: &(dyn Fn(SyncProgress) + Send + Sync),
) -> Result<(), SyncError> {
    // The server's count covers every folder: no total when some are hidden.
    let song_total = if folders.is_some() { None } else { scan.count.and_then(|c| u64::try_from(c).ok()) };
    let folders: Vec<Option<&str>> = match folders {
        Some(ids) => ids.iter().map(|id| Some(id.as_str())).collect(),
        None => vec![None],
    };

    for kind in [Kind::Artists, Kind::Albums, Kind::Songs] {
        let total = matches!(kind, Kind::Songs).then_some(song_total).flatten();
        let report = |done: u32| {
            progress(SyncProgress { phase: kind.phase(), done: done.into(), total, full: true })
        };

        let mut done = 0;
        report(0);
        for &folder in &folders {
            let mut offset = 0;
            let mut previous_first: Option<String> = None;
            loop {
                let items = client.search3_in(kind.page(offset), folder).await?;
                let (received, first) = kind.received(&items);
                if received > 0 && first == previous_first {
                    return Err(SyncError::PagingNotSupported);
                }

                db.call(move |conn| library::stage(conn, &items)).await?;
                offset += received;
                done += received;
                report(done);

                if received < PAGE_SIZE {
                    break;
                }
                previous_first = first;
            }
        }
    }

    // An empty answer must not wipe the library. Hidden folders can leave a
    // visible one empty on purpose.
    let staged = db.call(|conn| library::staged_song_count(conn)).await?;
    let reported = scan.count.unwrap_or(0);
    if staged == 0 && reported > 0 && song_total.is_some() {
        return Err(SyncError::EmptyLibrary(reported));
    }
    Ok(())
}
