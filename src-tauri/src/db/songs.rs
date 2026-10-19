//! The Songs page (spec 6.3): quick search and sorting run here, in SQLite,
//! not in the UI (spec 7), so a library of 50 000 songs opens and sorts fast.
//! The page asks for the IDs in their final order, then only for the lines
//! it shows.
//!
//! Each song keeps its metadata already normalized (lower case, without
//! accents): `search_text` for the search, `sort_*` for the text columns.

use std::hash::{DefaultHasher, Hash, Hasher};

use icu_normalizer::DecomposingNormalizerBorrowed;
use icu_properties::CodePointMapData;
use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};
use rusqlite::{Connection, params_from_iter};
use serde::Deserialize;

use super::DbResult;
use super::tracks::{TRACK_COLUMNS, TrackListSong, track_from_row};

/// More words than this in the search box are ignored.
const MAX_SEARCH_WORDS: usize = 16;

/// Lower case, without accents: "Beyoncé" and "beyonce" match. The same rule
/// as `normalize` in src/lib/tracklist.ts.
pub fn normalize(text: &str) -> String {
    let nfd = DecomposingNormalizerBorrowed::new_nfd();
    let categories = CodePointMapData::<GeneralCategory>::new();
    nfd.normalize_iter(text.chars())
        .filter(|&c| !GeneralCategoryGroup::Mark.contains(categories.get(c)))
        .flat_map(char::to_lowercase)
        .collect()
}

/// The fields of a song the search and the sorting look at.
pub struct SearchFields<'a> {
    pub title: &'a str,
    pub sort_name: Option<&'a str>,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub album_artist: Option<&'a str>,
    pub composer: Option<&'a str>,
    pub genre: Option<&'a str>,
    pub genres: &'a [String],
    pub year: Option<i64>,
    pub suffix: Option<&'a str>,
    pub bit_rate: Option<i64>,
    pub path: Option<&'a str>,
}

/// Values of the `search_text`, `sort_title`, `sort_artist` and `sort_album`
/// columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchColumns {
    pub search_text: String,
    pub sort_title: String,
    pub sort_artist: Option<String>,
    pub sort_album: Option<String>,
}

impl SearchColumns {
    /// The same fields as the quick search of album pages (src/lib/tracklist.ts).
    pub fn new(fields: &SearchFields) -> Self {
        let year = fields.year.map(|y| y.to_string());
        let bit_rate = fields.bit_rate.map(|b| format!("{b} kbps"));
        let text: Vec<&str> = [Some(fields.title), fields.artist, fields.album, fields.album_artist, fields.composer, fields.genre]
            .into_iter()
            .chain(fields.genres.iter().map(|g| Some(g.as_str())))
            .chain([year.as_deref(), fields.suffix, bit_rate.as_deref(), fields.path])
            .flatten()
            .filter(|f| !f.is_empty())
            .collect();
        let key = |value: Option<&str>| value.filter(|v| !v.trim().is_empty()).map(normalize);
        Self {
            search_text: normalize(&text.join("\n")),
            sort_title: normalize(fields.sort_name.filter(|s| !s.trim().is_empty()).unwrap_or(fields.title)),
            sort_artist: key(fields.artist),
            sort_album: key(fields.album),
        }
    }
}

/// Fills the search columns of the songs already in the cache (migration 4).
pub(super) fn fill_search_columns(conn: &Connection) -> DbResult<()> {
    let mut read = conn.prepare(
        "SELECT id, title, sort_name, artist, album, album_artist, composer, genre, genres, year,
                suffix, bit_rate, path
         FROM songs",
    )?;
    let mut write = conn.prepare(
        "UPDATE songs SET search_text = ?2, sort_title = ?3, sort_artist = ?4, sort_album = ?5 WHERE id = ?1",
    )?;
    let mut rows = read.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let text = |i| row.get::<_, Option<String>>(i);
        let (title, sort_name, artist, album) = (row.get::<_, String>(1)?, text(2)?, text(3)?, text(4)?);
        let (album_artist, composer, genre, path) = (text(5)?, text(6)?, text(7)?, text(12)?);
        let genres: Vec<String> = serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default();
        let suffix = text(10)?;
        let columns = SearchColumns::new(&SearchFields {
            title: &title,
            sort_name: sort_name.as_deref(),
            artist: artist.as_deref(),
            album: album.as_deref(),
            album_artist: album_artist.as_deref(),
            composer: composer.as_deref(),
            genre: genre.as_deref(),
            genres: &genres,
            year: row.get(9)?,
            suffix: suffix.as_deref(),
            bit_rate: row.get(11)?,
            path: path.as_deref(),
        });
        write.execute(rusqlite::params![
            id,
            columns.search_text,
            columns.sort_title,
            columns.sort_artist,
            columns.sort_album
        ])?;
    }
    Ok(())
}

/// What the Songs page sorts by (spec 6.3): its columns and the "Sort by"
/// options it shares with the Albums page (spec 6.4). `Added` is the "#"
/// column: the default order, last added first. `Random` is drawn from a
/// seed, so the order stays the same until a new draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SongSortKey {
    Added,
    Title,
    Artist,
    Album,
    Duration,
    Year,
    Genre,
    BitRate,
    PlayCount,
    /// Last played.
    Played,
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct SongSort {
    pub key: SongSortKey,
    pub direction: SortDirection,
    /// The draw of `Random` (ignored by the other keys).
    #[serde(default)]
    pub seed: u32,
}

impl SongSort {
    /// Last added first.
    pub const DEFAULT: SongSort = SongSort { key: SongSortKey::Added, direction: SortDirection::Desc, seed: 0 };

    #[cfg(test)]
    pub fn new(key: SongSortKey, direction: SortDirection) -> Self {
        Self { key, direction, seed: 0 }
    }

    /// `ORDER BY` clause. Songs without the value go last either way; equal
    /// values keep a stable, sensible order (artist, album, disc, track).
    fn order_by(self) -> String {
        let direction = match self.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };
        let first = match self.key {
            SongSortKey::Added => "julianday(created)",
            SongSortKey::Title => "sort_title",
            SongSortKey::Artist => "sort_artist",
            SongSortKey::Album => "sort_album",
            SongSortKey::Duration => "duration",
            SongSortKey::Year => "year",
            SongSortKey::Genre => "genre COLLATE NOCASE",
            SongSortKey::BitRate => "bit_rate",
            SongSortKey::PlayCount => "COALESCE(play_count, 0)",
            SongSortKey::Played => "julianday(played)",
            // Sorted after the query (see `song_ids`).
            SongSortKey::Random => "NULL",
        };
        let then = match self.key {
            SongSortKey::Added => "sort_title, sort_artist, sort_album",
            SongSortKey::Title => "sort_artist, sort_album",
            // "Most played": the last played first among equals (as on the Albums page).
            SongSortKey::PlayCount => {
                "julianday(played) DESC NULLS LAST, sort_artist, sort_album, COALESCE(disc_number, 1), COALESCE(track, 0), sort_title"
            }
            _ => "sort_artist, sort_album, COALESCE(disc_number, 1), COALESCE(track, 0), sort_title",
        };
        format!("{first} {direction} NULLS LAST, {then}, id")
    }
}

/// The words typed, normalized; empty means "every song".
fn search_words(query: &str) -> Vec<String> {
    normalize(query).split_whitespace().take(MAX_SEARCH_WORDS).map(str::to_string).collect()
}

/// IDs of the songs that match every word of `query`, in `sort` order.
pub fn song_ids(conn: &Connection, query: &str, sort: SongSort) -> DbResult<Vec<String>> {
    let words = search_words(query);
    let filter = if words.is_empty() {
        String::new()
    } else {
        let conditions: Vec<String> = (1..=words.len()).map(|i| format!("instr(search_text, ?{i}) > 0")).collect();
        format!("WHERE {}", conditions.join(" AND "))
    };
    let sql = format!("SELECT id FROM songs {filter} ORDER BY {}", sort.order_by());
    let mut stmt = conn.prepare(&sql)?;
    let mut ids: Vec<String> = stmt.query_map(params_from_iter(&words), |row| row.get(0))?.collect::<Result<_, _>>()?;
    if sort.key == SongSortKey::Random {
        // Each song gets a place from the seed and its ID, so a search keeps
        // the order of the draw.
        ids.sort_by_cached_key(|id| {
            let mut hasher = DefaultHasher::new();
            (sort.seed, id).hash(&mut hasher);
            hasher.finish()
        });
    }
    Ok(ids)
}

/// Every song of the library.
pub fn song_count(conn: &Connection) -> DbResult<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM songs", [], |row| row.get(0))?)
}

/// The lines of these songs, in the same order. IDs no longer in the cache
/// (removed by a sync meanwhile) are left out.
pub fn song_rows(conn: &Connection, ids: &[String]) -> DbResult<Vec<TrackListSong>> {
    let ids = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    // `json_each.key` is the position in the list.
    let columns = TRACK_COLUMNS.split(',').map(|c| format!("s.{}", c.trim())).collect::<Vec<_>>().join(", ");
    let sql = format!("SELECT {columns} FROM json_each(?1) AS j JOIN songs AS s ON s.id = j.value ORDER BY j.key");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([ids], track_from_row)?.collect::<Result<_, _>>()?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::library::{begin_full_sync, finish_full_sync, stage};
    use crate::db::open_in_memory;
    use crate::subsonic::{Genre, LibraryItems, Song};

    fn song(id: &str, title: &str, artist: &str, album: &str, created: &str) -> Song {
        Song {
            id: id.into(),
            title: title.into(),
            artist: Some(artist.into()),
            album: Some(album.into()),
            created: Some(created.into()),
            raw_json: "{}".into(),
            ..Default::default()
        }
    }

    fn library(songs: Vec<Song>) -> Connection {
        let mut conn = open_in_memory().unwrap();
        begin_full_sync(&conn).unwrap();
        stage(&mut conn, &LibraryItems { songs, ..Default::default() }).unwrap();
        finish_full_sync(&mut conn, None, 1).unwrap();
        conn
    }

    fn sorted(key: SongSortKey, direction: SortDirection) -> SongSort {
        SongSort::new(key, direction)
    }

    #[test]
    fn normalize_removes_accents_and_case() {
        assert_eq!(normalize("Beyoncé ÁGUA Ñandú"), "beyonce agua nandu");
        assert_eq!(normalize("Straße"), "straße");
        assert_eq!(normalize("Ελληνικά"), "ελληνικα");
    }

    #[test]
    fn search_columns_join_every_field_normalized() {
        let genres = ["MPB".to_string(), "Bossa Nova".to_string()];
        let columns = SearchColumns::new(&SearchFields {
            title: "Águas de Março",
            sort_name: None,
            artist: Some("Elis Regina"),
            album: Some("Elis & Tom"),
            album_artist: None,
            composer: Some("Tom Jobim"),
            genre: Some("MPB"),
            genres: &genres,
            year: Some(1974),
            suffix: Some("flac"),
            bit_rate: Some(1411),
            path: Some("Elis/Elis & Tom/01.flac"),
        });
        assert_eq!(
            columns.search_text,
            "aguas de marco\nelis regina\nelis & tom\ntom jobim\nmpb\nmpb\nbossa nova\n1974\nflac\n1411 kbps\nelis/elis & tom/01.flac"
        );
        assert_eq!(columns.sort_title, "aguas de marco");
        assert_eq!(columns.sort_artist.as_deref(), Some("elis regina"));
    }

    #[test]
    fn a_sync_fills_the_search_columns() {
        let mut s = song("s1", "Canção", "Zé", "Álbum", "2026-01-01T00:00:00Z");
        s.genres = vec![Genre { name: "Forró".into() }];
        let conn = library(vec![s]);
        let (text, artist): (String, String) = conn
            .query_row("SELECT search_text, sort_artist FROM songs WHERE id = 's1'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert!(text.contains("cancao") && text.contains("forro"), "{text}");
        assert_eq!(artist, "ze");
    }

    #[test]
    fn every_word_must_match_without_accents() {
        let conn = library(vec![
            song("a", "Águas de Março", "Elis Regina", "Elis & Tom", "2026-01-01T00:00:00Z"),
            song("b", "Garota de Ipanema", "Tom Jobim", "Getz/Gilberto", "2026-01-02T00:00:00Z"),
            song("c", "Marcha", "Outro", "Disco", "2026-01-03T00:00:00Z"),
        ]);
        let ids = |q: &str| song_ids(&conn, q, SongSort::DEFAULT).unwrap();
        assert_eq!(ids("marco"), ["a"]);
        assert_eq!(ids("TOM"), ["b", "a"]);
        assert_eq!(ids("tom elis"), ["a"]);
        assert_eq!(ids("  "), ["c", "b", "a"]);
        assert!(ids("nada disso").is_empty());
        // `%` and `_` are plain characters, not patterns.
        assert!(ids("%").is_empty());
    }

    #[test]
    fn sorts_by_each_column_with_missing_values_last() {
        let mut a = song("a", "Bravo", "Élis", "Zeta", "2026-01-01T00:00:00Z");
        a.year = Some(1999);
        a.play_count = Some(3);
        let mut b = song("b", "alfa", "Ana", "Ômega", "2026-07-21T17:10:32.999752282Z");
        b.year = Some(2010);
        let mut c = song("c", "Charlie", "", "", "2026-03-01T00:00:00Z");
        c.artist = None;
        c.album = None;
        c.play_count = Some(10);
        c.played = Some("2026-10-01T00:00:00Z".into());
        let conn = library(vec![a, b, c]);
        let ids = |key, direction| song_ids(&conn, "", sorted(key, direction)).unwrap();

        use SongSortKey::*;
        use SortDirection::*;
        assert_eq!(ids(Added, Desc), ["b", "c", "a"]);
        assert_eq!(ids(Added, Asc), ["a", "c", "b"]);
        assert_eq!(ids(Title, Asc), ["b", "a", "c"]);
        assert_eq!(ids(Artist, Asc), ["b", "a", "c"]);
        assert_eq!(ids(Artist, Desc), ["a", "b", "c"]);
        assert_eq!(ids(Album, Asc), ["b", "a", "c"]);
        assert_eq!(ids(Year, Desc), ["b", "a", "c"]);
        assert_eq!(ids(PlayCount, Desc), ["c", "a", "b"]);
        // Never played: by artist after the ones played.
        assert_eq!(ids(Played, Desc), ["c", "b", "a"]);
    }

    #[test]
    fn most_played_ties_go_last_played_first() {
        let mut a = song("a", "A", "X", "Y", "2026-01-01T00:00:00Z");
        a.play_count = Some(5);
        a.played = Some("2026-01-01T00:00:00Z".into());
        let mut b = song("b", "B", "X", "Y", "2026-01-01T00:00:00Z");
        b.play_count = Some(5);
        b.played = Some("2026-09-01T00:00:00Z".into());
        let conn = library(vec![a, b]);
        let ids = song_ids(&conn, "", sorted(SongSortKey::PlayCount, SortDirection::Desc)).unwrap();
        assert_eq!(ids, ["b", "a"]);
    }

    #[test]
    fn random_order_stays_with_the_seed_and_the_search() {
        let songs = (0..40).map(|i| song(&format!("s{i}"), &format!("T{i} {}", if i % 2 == 0 { "alfa" } else { "beta" }), "X", "Y", "2026-01-01T00:00:00Z"));
        let conn = library(songs.collect());
        let random = |seed, query: &str| {
            song_ids(&conn, query, SongSort { key: SongSortKey::Random, direction: SortDirection::Asc, seed }).unwrap()
        };
        let all = random(1, "");
        assert_eq!(all.len(), 40);
        assert_eq!(all, random(1, ""), "the same seed, the same order");
        assert_ne!(all, random(2, ""), "a new draw, a new order");
        assert_ne!(all, song_ids(&conn, "", SongSort::DEFAULT).unwrap());
        // Searching keeps the songs found in the order of the draw.
        let found = random(1, "alfa");
        let expected: Vec<String> = all.iter().filter(|id| id[1..].parse::<u32>().unwrap() % 2 == 0).cloned().collect();
        assert_eq!(found, expected);
    }

    #[test]
    fn rows_come_in_the_order_asked() {
        let conn = library(vec![
            song("a", "A", "X", "Y", "2026-01-01T00:00:00Z"),
            song("b", "B", "X", "Y", "2026-01-02T00:00:00Z"),
        ]);
        let ids = ["b".to_string(), "gone".to_string(), "a".to_string()];
        let rows = song_rows(&conn, &ids).unwrap();
        assert_eq!(rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), ["b", "a"]);
        assert_eq!(song_count(&conn).unwrap(), 2);
    }

    #[test]
    fn the_migration_fills_songs_already_cached() {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO songs (id, title, artist, raw_json) VALUES ('s1', 'Pé de Moleque', 'João', '{}')",
            [],
        )
        .unwrap();
        fill_search_columns(&conn).unwrap();
        assert_eq!(song_ids(&conn, "joao pe", SongSort::DEFAULT).unwrap(), ["s1"]);
    }
}
