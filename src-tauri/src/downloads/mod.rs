//! "Save to computer" (spec 8.2): the original files (`download`, never
//! converted) written to a folder the user chose. Different from "Download"
//! (spec 10, v0.2), which keeps copies inside the app.
//!
//! Every save joins one queue, worked by at most two downloads at a time, so
//! playback keeps its bandwidth. The UI gets `save-progress` ("Saving 3 of
//! 12") and, when the queue is empty, `save-finished`. Each file is written
//! to a `.part` file first and renamed at the end: a cancelled or failed
//! download never leaves half a file.

pub mod naming;

use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::items::SaveSong;
use crate::subsonic::{SubsonicClient, SubsonicError};

pub const SAVE_PROGRESS_EVENT: &str = "save-progress";
pub const SAVE_FINISHED_EVENT: &str = "save-finished";
/// Files downloaded at the same time (spec 8.2).
const WORKERS: usize = 2;
/// Name of an album's cover (spec 8.2).
const COVER_FILE: &str = "cover.jpg";
/// Subsonic error code: the user may not download (spec 8.2).
const NOT_AUTHORIZED: i32 = 50;

/// What is saved, and how its files are laid out (spec 8.2).
pub enum SaveItem {
    /// One song, at the path chosen in "Save as".
    Song { song: SaveSong, path: PathBuf },
    /// "Album artist - Album (Year)/" with "NN - Title.ext" and the cover.
    Album { folder_name: String, cover_art: Option<String>, songs: Vec<SaveSong> },
    /// "Name/" with "NNN - Artist - Title.ext" and "Name.m3u8".
    Playlist { name: String, songs: Vec<SaveSong> },
    /// "Artist - Title.ext" in the folder chosen.
    Selection { songs: Vec<SaveSong> },
}

/// A file to write.
struct Job {
    kind: JobKind,
    path: PathBuf,
    client: Arc<SubsonicClient>,
    /// Save generation: a cancel makes older jobs stop.
    generation: u64,
}

enum JobKind {
    Song(String),
    /// Not counted in "N of M": only the songs are.
    Cover(String),
}

/// The files of a save, before they join the queue.
pub struct Plan {
    /// Song files to download, then extra files (cover).
    jobs: Vec<(JobKind, PathBuf)>,
    /// Songs already there with the same size.
    skipped: usize,
    /// What "Open folder" opens.
    folder: PathBuf,
}

impl Plan {
    /// Songs of the save, the skipped ones included.
    pub fn song_count(&self) -> usize {
        self.skipped + self.jobs.iter().filter(|(kind, _)| matches!(kind, JobKind::Song(_))).count()
    }
}

fn size_on_disk(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| m.len())
}

/// Lays out the files under `base` (the folder chosen, or the file chosen
/// for one song): creates the folder, writes the playlist's `.m3u8`, and
/// leaves out the files already saved.
pub fn plan(item: SaveItem, base: &Path) -> io::Result<Plan> {
    let mut taken = HashSet::new();
    let mut jobs = Vec::new();
    let mut skipped = 0;
    // Where the song's file ends up: a new name, or the one already saved.
    let mut add = |song: &SaveSong, path: PathBuf, jobs: &mut Vec<(JobKind, PathBuf)>| -> PathBuf {
        let size = song.size.and_then(|s| u64::try_from(s).ok());
        match naming::target(&path, size, &mut taken, size_on_disk) {
            Some(target) => {
                jobs.push((JobKind::Song(song.id.clone()), target.clone()));
                target
            }
            None => {
                skipped += 1;
                path
            }
        }
    };

    let folder = match item {
        SaveItem::Song { song, path } => {
            add(&song, path.clone(), &mut jobs);
            path.parent().map(Path::to_path_buf).unwrap_or_else(|| base.to_path_buf())
        }
        SaveItem::Selection { songs } => {
            for song in &songs {
                add(song, base.join(naming::song_file(song)), &mut jobs);
            }
            base.to_path_buf()
        }
        SaveItem::Album { folder_name, cover_art, songs } => {
            let folder = base.join(folder_name);
            fs::create_dir_all(&folder)?;
            let discs: HashSet<i64> = songs.iter().map(|s| s.disc_number.unwrap_or(1)).collect();
            for song in &songs {
                add(song, folder.join(naming::album_file(song, discs.len() > 1)), &mut jobs);
            }
            let cover = folder.join(COVER_FILE);
            if let Some(cover_art) = cover_art.filter(|_| size_on_disk(&cover).is_none()) {
                jobs.push((JobKind::Cover(cover_art), cover));
            }
            folder
        }
        SaveItem::Playlist { name, songs } => {
            let name = naming::clean(&name);
            let folder = base.join(&name);
            fs::create_dir_all(&folder)?;
            let mut entries = Vec::with_capacity(songs.len());
            for (index, song) in songs.iter().enumerate() {
                let path = add(song, folder.join(naming::playlist_file(index + 1, song)), &mut jobs);
                if let Some(file) = path.file_name() {
                    entries.push((song, file.to_string_lossy().into_owned()));
                }
            }
            fs::write(folder.join(format!("{name}.m3u8")), naming::m3u8(&entries))?;
            folder
        }
    };
    Ok(Plan { jobs, skipped, folder })
}

/// Payload of `save-progress`: songs done (saved, skipped or failed) of all.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveProgress {
    done: usize,
    total: usize,
}

/// Payload of `save-finished`.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveFinished {
    saved: usize,
    skipped: usize,
    /// Names of the files that could not be saved.
    failed: Vec<String>,
    cancelled: bool,
    /// The server refused: the user has no download permission.
    forbidden: bool,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    /// Downloads running.
    workers: usize,
    total: usize,
    done: usize,
    result: SaveFinished,
    folder: Option<PathBuf>,
}

enum JobError {
    Cancelled,
    Forbidden,
    Failed(String),
}

impl From<SubsonicError> for JobError {
    fn from(error: SubsonicError) -> Self {
        match &error {
            SubsonicError::Api { code: NOT_AUTHORIZED, .. } => Self::Forbidden,
            SubsonicError::Http(e) if matches!(e.status().map(|s| s.as_u16()), Some(401 | 403)) => Self::Forbidden,
            // reqwest's errors come without the URL (and its token): see SubsonicError.
            _ => Self::Failed(error.to_string()),
        }
    }
}

impl From<io::Error> for JobError {
    fn from(error: io::Error) -> Self {
        Self::Failed(error.to_string())
    }
}

pub struct Saves {
    app: AppHandle,
    queue: Mutex<Queue>,
    /// Raised by Cancel; jobs of older generations stop.
    generation: AtomicU64,
    /// The folder of the last save that finished ("Open folder").
    last_folder: Mutex<Option<PathBuf>>,
}

impl Saves {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            queue: Mutex::new(Queue::default()),
            generation: AtomicU64::new(0),
            last_folder: Mutex::new(None),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Adds a save to the queue and starts downloading.
    pub fn start(self: &Arc<Self>, client: Arc<SubsonicClient>, plan: Plan) {
        let generation = self.generation.load(Ordering::SeqCst);
        let mut queue = self.lock();
        if queue.result.cancelled {
            // A new save after Cancel: the cancelled files no longer count.
            queue.result = SaveFinished::default();
            queue.total = 0;
            queue.done = 0;
        }
        let songs = plan.song_count();
        queue.total += songs;
        queue.done += plan.skipped;
        queue.result.skipped += plan.skipped;
        queue.folder = Some(plan.folder);
        queue.jobs.extend(plan.jobs.into_iter().map(|(kind, path)| Job {
            kind,
            path,
            client: Arc::clone(&client),
            generation,
        }));
        self.emit_progress(&queue);

        let wanted = WORKERS.min(queue.jobs.len()).saturating_sub(queue.workers);
        queue.workers += wanted;
        if queue.workers == 0 {
            // Everything was already saved.
            self.finish(&mut queue);
        }
        drop(queue);
        for _ in 0..wanted {
            let saves = Arc::clone(self);
            tauri::async_runtime::spawn(async move { saves.work().await });
        }
    }

    /// Cancel: the files being written stop, the others are not started.
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let mut queue = self.lock();
        queue.jobs.clear();
        queue.result.cancelled = true;
        if queue.workers == 0 && queue.total > 0 {
            self.finish(&mut queue);
        }
    }

    /// The folder of the last finished save, for "Open folder".
    pub fn last_folder(&self) -> Option<PathBuf> {
        self.last_folder.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
    }

    async fn work(self: Arc<Self>) {
        loop {
            // Taking the last job and leaving happen under one lock, so a
            // save added meanwhile always finds a worker.
            let job = {
                let mut queue = self.lock();
                match queue.jobs.pop_front() {
                    Some(job) => job,
                    None => {
                        queue.workers -= 1;
                        if queue.workers == 0 {
                            self.finish(&mut queue);
                        }
                        return;
                    }
                }
            };
            let result = self.run(&job).await;

            let mut queue = self.lock();
            if job.generation != self.generation.load(Ordering::SeqCst) {
                continue;
            }
            let name = job.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match result {
                Ok(()) if matches!(job.kind, JobKind::Song(_)) => queue.result.saved += 1,
                Ok(()) | Err(JobError::Cancelled) => {}
                Err(JobError::Forbidden) => {
                    // Every other file would be refused too.
                    queue.result.forbidden = true;
                    queue.jobs.clear();
                }
                Err(JobError::Failed(detail)) => {
                    log::warn!("saves: could not save {name}: {detail}");
                    queue.result.failed.push(name);
                }
            }
            if matches!(job.kind, JobKind::Song(_)) {
                queue.done += 1;
            }
            self.emit_progress(&queue);
        }
    }

    async fn run(&self, job: &Job) -> Result<(), JobError> {
        match &job.kind {
            JobKind::Cover(cover_id) => {
                let bytes = job.client.get_full_cover_art(cover_id).await?;
                write_whole(&job.path, &bytes)?;
                Ok(())
            }
            JobKind::Song(song_id) => {
                let mut response = job.client.download(song_id).await?;
                let part = part_path(&job.path);
                let mut file = fs::File::create(&part)?;
                let written = async {
                    while let Some(chunk) = response.chunk().await.map_err(SubsonicError::from)? {
                        if job.generation != self.generation.load(Ordering::SeqCst) {
                            return Err(JobError::Cancelled);
                        }
                        file.write_all(&chunk)?;
                    }
                    file.flush()?;
                    Ok::<(), JobError>(())
                }
                .await;
                drop(file);
                match written {
                    Ok(()) => fs::rename(&part, &job.path).map_err(|e| {
                        let _ = fs::remove_file(&part);
                        e.into()
                    }),
                    Err(error) => {
                        let _ = fs::remove_file(&part);
                        Err(error)
                    }
                }
            }
        }
    }

    fn emit_progress(&self, queue: &Queue) {
        let progress = SaveProgress { done: queue.done, total: queue.total };
        if let Err(error) = self.app.emit(SAVE_PROGRESS_EVENT, progress) {
            log::warn!("saves: could not send the progress: {error}");
        }
    }

    /// The queue is empty: tells the UI and starts again from zero.
    fn finish(&self, queue: &mut Queue) {
        let finished = std::mem::take(&mut queue.result);
        *self.last_folder.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = queue.folder.take();
        queue.total = 0;
        queue.done = 0;
        if let Err(error) = self.app.emit(SAVE_FINISHED_EVENT, finished) {
            log::warn!("saves: could not send the end: {error}");
        }
    }
}

/// Where the folder chosen last is kept (spec 8.2), in the app config directory.
const LAST_FOLDER_FILE: &str = "last-save-folder.txt";

/// The folder chosen last, if it still exists.
pub fn last_used_folder(config_dir: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(config_dir.join(LAST_FOLDER_FILE)).ok()?;
    Some(PathBuf::from(text.trim())).filter(|path| path.is_dir())
}

pub fn remember_folder(config_dir: &Path, folder: &Path) -> io::Result<()> {
    fs::create_dir_all(config_dir)?;
    fs::write(config_dir.join(LAST_FOLDER_FILE), folder.to_string_lossy().as_bytes())
}

/// "song.flac" is written as "song.flac.part" first.
fn part_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    path.with_file_name(name)
}

/// A small file in one go, through a `.part` file too.
fn write_whole(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let part = part_path(path);
    fs::write(&part, bytes)?;
    fs::rename(&part, path).inspect_err(|_| {
        let _ = fs::remove_file(&part);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("parlo-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn song(id: &str, title: &str, track: i64, disc: i64, size: i64) -> SaveSong {
        SaveSong {
            id: id.into(),
            title: title.into(),
            artist: Some("Band".into()),
            track: Some(track),
            disc_number: Some(disc),
            suffix: Some("mp3".into()),
            size: Some(size),
            duration: Some(60),
            ..SaveSong::default()
        }
    }

    fn names(plan: &Plan) -> Vec<String> {
        plan.jobs.iter().map(|(_, path)| path.file_name().unwrap().to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn an_album_gets_its_folder_disc_numbers_and_cover() {
        let base = temp_dir("album");
        let item = SaveItem::Album {
            folder_name: naming::album_folder(Some("Band"), "Live", Some(2001)),
            cover_art: Some("al-1".into()),
            songs: vec![song("a", "Intro", 1, 1, 10), song("b", "Outro", 1, 2, 10)],
        };
        let plan = plan(item, &base).unwrap();
        assert_eq!(plan.folder, base.join("Band - Live (2001)"));
        assert!(plan.folder.is_dir());
        assert_eq!(names(&plan), ["1-01 - Intro.mp3", "2-01 - Outro.mp3", "cover.jpg"]);
        assert_eq!(plan.song_count(), 2);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_playlist_writes_its_m3u8_and_skips_files_already_saved() {
        let base = temp_dir("playlist");
        let folder = base.join("Road trip");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("002 - Band - Two.mp3"), b"12345").unwrap();
        let item = SaveItem::Playlist {
            name: "Road trip".into(),
            songs: vec![song("a", "One", 1, 1, 3), song("b", "Two", 2, 1, 5), song("a", "One", 1, 1, 3)],
        };
        let plan = plan(item, &base).unwrap();
        assert_eq!(names(&plan), ["001 - Band - One.mp3", "003 - Band - One.mp3"]);
        assert_eq!((plan.skipped, plan.song_count()), (1, 3));
        let m3u8 = fs::read_to_string(folder.join("Road trip.m3u8")).unwrap();
        assert!(m3u8.contains("001 - Band - One.mp3\n#EXTINF:60,Band - Two\n002 - Band - Two.mp3\n"));
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_selection_numbers_two_songs_with_one_name() {
        let base = temp_dir("selection");
        let item = SaveItem::Selection { songs: vec![song("a", "Same", 1, 1, 3), song("b", "Same", 2, 1, 4)] };
        let plan = plan(item, &base).unwrap();
        assert_eq!(names(&plan), ["Band - Same.mp3", "Band - Same (1).mp3"]);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn part_files_sit_next_to_the_file() {
        assert_eq!(part_path(Path::new("dir/a.flac")), Path::new("dir/a.flac.part"));
    }
}
