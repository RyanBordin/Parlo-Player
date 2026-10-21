//! Performance test mode (development only): `npm run tauri:perf` builds the
//! app with the `perf-library` feature, and it opens a separate database of
//! 50 000 made-up songs instead of the real library (spec 12.3).
//!
//! Kept apart from the real data:
//! - The database lives in `{local data dir}/perf/`, not in `libraries/`.
//! - The credential store and the saved login are never read or written;
//!   signing in and out are refused.
//! - The "server" is `http://perf.invalid`: the `.invalid` domain never
//!   exists (RFC 2606), so nothing (queue saves, scrobbles, favorites) can
//!   reach a real server. The sync is off.
//!
//! The songs have no covers and no audio; the data is drawn from a fixed
//! seed, so every run measures the same library.

use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::seq::IndexedRandom;
use rand::{RngExt, SeedableRng};
use rusqlite::Connection;

use crate::db::{self, DbResult, library};
use crate::session::SavedSession;
use crate::subsonic::{Album, Artist, Genre, LibraryItems, Song};

#[cfg(all(feature = "perf-library", not(debug_assertions)))]
compile_error!("the `perf-library` feature is for development builds only");

/// Whether this build is the performance test mode.
pub const ENABLED: bool = cfg!(feature = "perf-library");

const SERVER_URL: &str = "http://perf.invalid";
const USERNAME: &str = "teste-50k";

/// Size of the made-up library.
#[derive(Debug, Clone, Copy)]
pub struct Shape {
    pub songs: usize,
    pub albums: usize,
    pub artists: usize,
}

pub const SHAPE: Shape = Shape { songs: 50_000, albums: 4_500, artists: 1_200 };
const SEED: u64 = 50_000;
/// Songs written per transaction.
const BATCH: usize = 5_000;

pub fn session() -> SavedSession {
    SavedSession { server_url: SERVER_URL.into(), username: USERNAME.into() }
}

/// `{local data dir}/perf/library-50k.db`.
pub fn db_path(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join("perf").join(format!("library-{}k.db", SHAPE.songs / 1000))
}

/// Creates the test database on first use (a few seconds); later runs open
/// it as it is. To draw it again, delete the `perf` folder.
pub fn ensure(path: &Path) -> DbResult<()> {
    let mut conn = db::open(path)?;
    let songs: i64 = conn.query_row("SELECT COUNT(*) FROM songs", [], |row| row.get(0))?;
    if songs as usize == SHAPE.songs {
        return Ok(());
    }
    drop(conn);
    // Half written (the app closed meanwhile): start over.
    for suffix in ["", "-wal", "-shm"] {
        let file = PathBuf::from(format!("{}{suffix}", path.display()));
        if file.exists() {
            fs::remove_file(file)?;
        }
    }
    conn = db::open(path)?;
    generate(&mut conn, SHAPE, SEED, crate::sync::now())
}

const SYLLABLES: &[&str] = &[
    "ma", "lú", "ré", "so", "ti", "ça", "nho", "ão", "bel", "ka", "zé", "ñu", "mi", "dó", "ra", "vé",
    "quê", "lu", "pa", "tã", "ber", "go", "fi", "nê", "ol", "sa", "rin", "co", "jú", "va", "el", "mor",
];

const GENRES: &[&str] = &[
    "Rock", "Pop", "MPB", "Samba", "Forró", "Jazz", "Blues", "Música Clássica", "Eletrônica", "Hip-Hop",
    "Reggae", "Funk", "Soul", "Bossa Nova", "Metal", "Punk", "Folk", "Country", "Axé", "Sertanejo",
    "Indie", "Ambient", "Trilha Sonora", "Gospel", "Pagode",
];

/// Bitrates in kbps; above 320 they are FLAC files.
const BIT_RATES: &[i64] = &[128, 192, 256, 320, 320, 320, 900, 1_000, 1_411];

fn word(rng: &mut StdRng) -> String {
    let count = rng.random_range(1..=3);
    let mut word: String = (0..count).map(|_| *SYLLABLES.choose(rng).unwrap()).collect();
    if let Some(first) = word.chars().next() {
        word.replace_range(..first.len_utf8(), &first.to_uppercase().to_string());
    }
    word
}

fn words(rng: &mut StdRng, max: usize) -> String {
    let count = rng.random_range(1..=max);
    (0..count).map(|_| word(rng)).collect::<Vec<_>>().join(" ")
}

/// ISO 8601 date `days` before `now` (Unix seconds), plus `seconds`.
fn date(now: i64, days: i64, seconds: i64) -> String {
    let at = now - days * 86_400 + seconds;
    let (day, second) = (at.div_euclid(86_400), at.rem_euclid(86_400));
    let (year, month, day) = crate::time::civil_from_days(day);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second / 3_600,
        second % 3_600 / 60,
        second % 60
    )
}

/// Writes a made-up library through the same path as a full sync.
pub fn generate(conn: &mut Connection, shape: Shape, seed: u64, now: i64) -> DbResult<()> {
    let mut rng = StdRng::seed_from_u64(seed);

    let artists: Vec<Artist> = (0..shape.artists)
        .map(|i| Artist {
            id: format!("perf-ar-{i}"),
            name: words(&mut rng, 2),
            raw_json: "{}".into(),
            ..Default::default()
        })
        .collect();

    // Each album: an artist, a year, a genre and the day it was added.
    struct AlbumInfo {
        album: Album,
        artist: usize,
        added_days_ago: i64,
    }
    let mut albums: Vec<AlbumInfo> = (0..shape.albums)
        .map(|i| {
            let artist = rng.random_range(0..artists.len());
            let genre = GENRES.choose(&mut rng).unwrap().to_string();
            let added_days_ago = rng.random_range(0..5 * 365);
            AlbumInfo {
                album: Album {
                    id: format!("perf-al-{i}"),
                    name: words(&mut rng, 3),
                    artist: Some(artists[artist].name.clone()),
                    artist_id: Some(artists[artist].id.clone()),
                    year: Some(rng.random_range(1960..=2026)),
                    genre: Some(genre.clone()),
                    genres: vec![Genre { name: genre }],
                    created: Some(date(now, added_days_ago, 0)),
                    raw_json: "{}".into(),
                    ..Default::default()
                },
                artist,
                added_days_ago,
            }
        })
        .collect();

    let mut songs = Vec::with_capacity(shape.songs);
    for i in 0..shape.songs {
        // Songs spread evenly over the albums, in track order.
        let album_index = i * shape.albums / shape.songs;
        let first_of_album = album_index * shape.songs / shape.albums;
        let track = (i - first_of_album + 1) as i64;
        let info = &mut albums[album_index];
        let bit_rate = *BIT_RATES.choose(&mut rng).unwrap();
        let suffix = if bit_rate > 320 { "flac" } else { "mp3" };
        // Most songs were never played; a few were played a lot.
        let play_count = if rng.random_bool(0.6) { 0 } else { rng.random_range(1..15i64).pow(2) };
        let duration = rng.random_range(90..600);
        let title = words(&mut rng, 4);
        let artist = &artists[info.artist];
        let album = &mut info.album;
        album.song_count = Some(album.song_count.unwrap_or(0) + 1);
        album.duration = Some(album.duration.unwrap_or(0) + duration);
        album.play_count = Some(album.play_count.unwrap_or(0) + play_count);
        songs.push(Song {
            id: format!("perf-so-{i}"),
            path: Some(format!("{}/{}/{track:02} - {title}.{suffix}", artist.name, album.name)),
            title,
            album: Some(album.name.clone()),
            album_id: Some(album.id.clone()),
            artist: Some(artist.name.clone()),
            artist_id: Some(artist.id.clone()),
            display_album_artist: Some(artist.name.clone()),
            track: Some(track),
            disc_number: Some(1),
            year: album.year,
            genre: album.genre.clone(),
            genres: album.genres.clone(),
            duration: Some(duration),
            bit_rate: Some(bit_rate),
            suffix: Some(suffix.into()),
            created: Some(date(now, info.added_days_ago, track)),
            starred: rng.random_bool(0.03).then(|| date(now, rng.random_range(0..365), 0)),
            play_count: Some(play_count),
            played: (play_count > 0).then(|| date(now, rng.random_range(0..120), 0)),
            raw_json: "{}".into(),
            ..Default::default()
        });
    }

    library::begin_full_sync(conn)?;
    library::stage(conn, &LibraryItems { artists, albums: albums.into_iter().map(|a| a.album).collect(), songs: Vec::new() })?;
    for batch in songs.chunks(BATCH) {
        library::stage(conn, &LibraryItems { songs: batch.to_vec(), ..Default::default() })?;
    }
    library::finish_full_sync(conn, None, now)?;
    conn.execute("UPDATE sync_state SET last_light_sync_at = ?1 WHERE id = 1", [now])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::db::songs::{SongSort, song_ids};

    const SMALL: Shape = Shape { songs: 230, albums: 20, artists: 7 };

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn generates_the_shape_asked_and_marks_it_synced() {
        let mut conn = open_in_memory().unwrap();
        generate(&mut conn, SMALL, 1, 1_790_000_000).unwrap();
        assert_eq!((count(&conn, "songs"), count(&conn, "albums"), count(&conn, "artists")), (230, 20, 7));
        let synced: Option<i64> = conn.query_row("SELECT last_full_sync_at FROM sync_state", [], |r| r.get(0)).unwrap();
        assert_eq!(synced, Some(1_790_000_000));
        // Every song is searchable and the album counts add up.
        assert_eq!(song_ids(&conn, "", SongSort::DEFAULT).unwrap().len(), 230);
        let songs_in_albums: i64 = conn.query_row("SELECT SUM(song_count) FROM albums", [], |r| r.get(0)).unwrap();
        assert_eq!(songs_in_albums, 230);
    }

    #[test]
    fn the_same_seed_draws_the_same_library() {
        let titles = |seed| {
            let mut conn = open_in_memory().unwrap();
            generate(&mut conn, SMALL, seed, 1_790_000_000).unwrap();
            conn.prepare("SELECT title FROM songs ORDER BY id")
                .unwrap()
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        assert_eq!(titles(7), titles(7));
        assert_ne!(titles(7), titles(8));
    }

    /// Times the Songs page queries on the full test library (the UI adds
    /// the transfer and the drawing): `cargo test --release timings -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn timings_with_50k_songs() {
        use crate::db::songs::{SongSortKey, SortDirection, song_rows};
        use std::time::Instant;

        let mut conn = open_in_memory().unwrap();
        let start = Instant::now();
        generate(&mut conn, SHAPE, SEED, 1_790_000_000).unwrap();
        println!("generate: {:?}", start.elapsed());

        let time = |label: &str, query: &str, sort: SongSort| {
            let start = Instant::now();
            let ids = song_ids(&conn, query, sort).unwrap();
            println!("{label}: {} songs in {:?}", ids.len(), start.elapsed());
            ids
        };
        let ids = time("open (recently added)", "", SongSort::DEFAULT);
        time("search \"ma\"", "ma", SongSort::DEFAULT);
        time("search \"lu re 320\"", "lu re 320", SongSort::DEFAULT);
        for key in [SongSortKey::Title, SongSortKey::Artist, SongSortKey::Album, SongSortKey::PlayCount, SongSortKey::Genre, SongSortKey::Played, SongSortKey::Random] {
            time(&format!("sort {key:?}"), "", SongSort::new(key, SortDirection::Asc));
        }
        let start = Instant::now();
        let rows = song_rows(&conn, &ids[20_000..20_100]).unwrap();
        println!("100 lines: {} in {:?}", rows.len(), start.elapsed());

        let start = Instant::now();
        let artists = crate::db::artists::artist_list(&conn).unwrap();
        println!("artists page: {} artists in {:?}", artists.len(), start.elapsed());

        // Global search (spec 6.9): each letter typed, then longer searches.
        for query in ["m", "ma", "mar", "ma lu", "lu re 320", "zé ma mor", "rock 1999"] {
            let start = Instant::now();
            let results = crate::db::search::search(&conn, query).unwrap();
            println!(
                "global search \"{query}\": {} songs, {} albums, {} artists in {:?}",
                results.song_ids.len(),
                results.albums.len(),
                results.artists.len(),
                start.elapsed()
            );
        }
    }

    #[test]
    fn dates_are_iso_8601() {
        assert_eq!(date(0, 0, 0), "1970-01-01T00:00:00Z");
        assert_eq!(date(86_400 * 2, 1, 3_661), "1970-01-02T01:01:01Z");
    }

    #[test]
    fn the_test_database_is_outside_the_real_libraries() {
        let path = db_path(Path::new("data"));
        assert_eq!(path, Path::new("data").join("perf").join("library-50k.db"));
        assert!(!path.starts_with(Path::new("data").join("libraries")));
        assert!(session().server_url.ends_with(".invalid"));
    }
}
