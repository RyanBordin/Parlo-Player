//! Global search of the Topbar (spec 6.9), on the local cache (spec 7).
//!
//! - Case and accents do not matter, and every word typed must start a word
//!   of some field, in any order: "beat abb" finds "Abbey Road" by "The
//!   Beatles".
//! - Songs look at their title and every metadata of the Songs page; albums
//!   at their name, artist, year and genres; artists at their name.
//! - Artists are the ones of "All artists" on the Artists page: album
//!   artists and featured artists from the OpenSubsonic lists. A combined
//!   name such as "X feat. Y" is only a song's display text, never an artist.
//! - The FTS5 indexes (migration 9) find the items; the ranking runs here.
//!
//! Each list goes: name equal to the search, name starting with it, every
//! word in the name, found only in the other fields; then the most played
//! first, then by name. The best result is the first item of one of the
//! lists: the strongest match, and on a tie artist, then album, then song.

use std::cmp::Reverse;

use rusqlite::{Connection, params};
use serde::Serialize;

use super::DbResult;
use super::artists::{FALLBACK_COVER, IS_ALBUM_ARTIST, IS_LISTED_ARTIST};
use super::songs::{MAX_SEARCH_WORDS, normalize, song_rows};
use super::tracks::TrackListSong;

/// Songs next to the best result ("Top" tab).
pub const TOP_SONGS: usize = 4;
/// Searches kept in the recent list.
pub const MAX_RECENT_SEARCHES: usize = 10;

/// An album found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumResult {
    pub id: String,
    pub name: String,
    pub artist: Option<String>,
    pub artist_id: Option<String>,
    pub cover_art: Option<String>,
    pub starred: bool,
    pub song_count: Option<i64>,
    pub year: Option<i64>,
}

/// An artist found. Without a photo on the server, the card shows
/// `fallback_cover_art` (spec 6.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistResult {
    pub id: String,
    pub name: String,
    pub cover_art: Option<String>,
    pub fallback_cover_art: Option<String>,
    pub starred: bool,
    pub album_count: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResultKind {
    Song,
    Album,
    Artist,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    /// Every song found, best first: the queue of a click (spec 6.9).
    pub song_ids: Vec<String>,
    /// The first `TOP_SONGS` of them.
    pub top_songs: Vec<TrackListSong>,
    pub albums: Vec<AlbumResult>,
    pub artists: Vec<ArtistResult>,
    /// The list whose first item is the best result; `None` if nothing was found.
    pub best: Option<ResultKind>,
}

/// How an item's name matches the search, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Match {
    /// The same words: "abbey road" for "Abbey Road".
    Exact,
    /// The name starts with the words: "abbey" for "Abbey Road".
    Start,
    /// Every word is in the name: "road abbey".
    Name,
    /// Found only in the other fields (artist, genre, year...).
    Other,
}

impl Match {
    /// For the best result, starting the name and having its words weigh the
    /// same, so "beatles" gives the artist "The Beatles" and not the album
    /// "Beatles for Sale".
    fn strength(self) -> u8 {
        match self {
            Match::Exact => 0,
            Match::Start | Match::Name => 1,
            Match::Other => 2,
        }
    }
}

/// The words of a normalized text, as the FTS5 tokenizer splits them.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty())
}

/// The words typed, normalized; empty means "nothing to search".
fn query_words(query: &str) -> Vec<String> {
    words(&normalize(query)).take(MAX_SEARCH_WORDS).map(str::to_string).collect()
}

/// Every word as a prefix, in any order: `"beat"* "abb"*`. The words are
/// letters and digits only, so they never break out of the quotes.
fn fts_query(words: &[String]) -> String {
    words.iter().map(|w| format!("\"{w}\"*")).collect::<Vec<_>>().join(" ")
}

/// Runs for every song found (tens of thousands for one letter), so it
/// walks the words without collecting them.
fn match_of(name: &str, query: &[String]) -> Match {
    let mut name_words = words(name);
    let mut exact = true;
    let in_order = query.iter().all(|q| match name_words.next() {
        Some(n) => {
            exact &= n == q;
            n.starts_with(q.as_str())
        }
        None => false,
    });
    if in_order {
        if exact && name_words.next().is_none() { Match::Exact } else { Match::Start }
    } else if query.iter().all(|q| words(name).any(|n| n.starts_with(q.as_str()))) {
        Match::Name
    } else {
        Match::Other
    }
}

/// An item found, with what ranks it.
struct Found<T> {
    matched: Match,
    plays: i64,
    /// Normalized name.
    name: String,
    id: String,
    item: T,
}

fn rank<T>(mut found: Vec<Found<T>>) -> Vec<Found<T>> {
    found.sort_by(|a, b| {
        (a.matched, Reverse(a.plays), &a.name, &a.id).cmp(&(b.matched, Reverse(b.plays), &b.name, &b.id))
    });
    found
}

/// Songs, albums and artists that match `query`, best first.
pub fn search(conn: &Connection, query: &str) -> DbResult<SearchResults> {
    let words = query_words(query);
    if words.is_empty() {
        return Ok(SearchResults::default());
    }
    let fts = fts_query(&words);
    let songs = rank(found_songs(conn, &fts, &words)?);
    let albums = rank(found_albums(conn, &fts, &words)?);
    let artists = rank(found_artists(conn, &fts, &words)?);

    // The order of the candidates is the order of preference on a tie.
    let best = [
        (ResultKind::Artist, artists.first().map(|f| f.matched)),
        (ResultKind::Album, albums.first().map(|f| f.matched)),
        (ResultKind::Song, songs.first().map(|f| f.matched)),
    ]
    .into_iter()
    .filter_map(|(kind, matched)| Some((kind, matched?)))
    .min_by_key(|(_, matched)| matched.strength())
    .map(|(kind, _)| kind);

    let song_ids: Vec<String> = songs.into_iter().map(|f| f.id).collect();
    let top_songs = song_rows(conn, &song_ids[..song_ids.len().min(TOP_SONGS)])?;
    Ok(SearchResults {
        song_ids,
        top_songs,
        albums: albums.into_iter().map(|f| f.item).collect(),
        artists: artists.into_iter().map(|f| f.item).collect(),
        best,
    })
}

fn found_songs(conn: &Connection, fts: &str, words: &[String]) -> DbResult<Vec<Found<()>>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.search_name, COALESCE(s.play_count, 0)
           FROM songs_fts JOIN songs s ON s.rowid = songs_fts.rowid
          WHERE songs_fts MATCH ?1",
    )?;
    let found = stmt
        .query_map([fts], |row| {
            let name: String = row.get(1)?;
            Ok(Found { matched: match_of(&name, words), plays: row.get(2)?, name, id: row.get(0)?, item: () })
        })?
        .collect::<Result<_, _>>()?;
    Ok(found)
}

fn found_albums(conn: &Connection, fts: &str, words: &[String]) -> DbResult<Vec<Found<AlbumResult>>> {
    let mut stmt = conn.prepare(
        "SELECT al.id, al.search_name, COALESCE(al.play_count, 0), al.name, al.artist, al.artist_id,
                al.cover_art, al.starred IS NOT NULL, al.song_count, al.year
           FROM albums_fts JOIN albums al ON al.rowid = albums_fts.rowid
          WHERE albums_fts MATCH ?1",
    )?;
    let found = stmt
        .query_map([fts], |row| {
            let name: String = row.get(1)?;
            let id: String = row.get(0)?;
            Ok(Found {
                matched: match_of(&name, words),
                plays: row.get(2)?,
                name,
                id: id.clone(),
                item: AlbumResult {
                    id,
                    name: row.get(3)?,
                    artist: row.get(4)?,
                    artist_id: row.get(5)?,
                    cover_art: row.get(6)?,
                    starred: row.get(7)?,
                    song_count: row.get(8)?,
                    year: row.get(9)?,
                },
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(found)
}

fn found_artists(conn: &Connection, fts: &str, words: &[String]) -> DbResult<Vec<Found<ArtistResult>>> {
    // Plays of the artist's songs, as on the Artists page; only for the
    // artists found.
    let mut stmt = conn.prepare(&format!(
        "SELECT ar.id, ar.search_name,
                COALESCE((SELECT SUM(COALESCE(s.play_count, 0)) FROM songs s
                           WHERE s.id IN (SELECT song_id FROM song_artists WHERE artist_id = ar.id)), 0),
                ar.name, ar.cover_art, {FALLBACK_COVER}, ar.starred IS NOT NULL,
                COALESCE(ar.album_count, (SELECT COUNT(*) FROM albums al WHERE al.artist_id = ar.id)),
                {IS_ALBUM_ARTIST} AS album_artist
           FROM artists_fts JOIN artists ar ON ar.rowid = artists_fts.rowid
          WHERE artists_fts MATCH ?1 AND {IS_LISTED_ARTIST}"
    ))?;
    let found = stmt
        .query_map([fts], |row| {
            let name: String = row.get(1)?;
            let id: String = row.get(0)?;
            Ok(Found {
                matched: match_of(&name, words),
                plays: row.get(2)?,
                name,
                id: id.clone(),
                item: ArtistResult {
                    id,
                    name: row.get(3)?,
                    cover_art: row.get(4)?,
                    fallback_cover_art: row.get(5)?,
                    starred: row.get(6)?,
                    album_count: row.get(7)?,
                },
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(found)
}

/// Values of the `search_name` and `search_text` columns of an album.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlbumSearchColumns {
    pub search_name: String,
    pub search_text: String,
}

impl AlbumSearchColumns {
    pub fn new(name: &str, artist: Option<&str>, year: Option<i64>, genre: Option<&str>, genres: &[String]) -> Self {
        let year = year.map(|y| y.to_string());
        let text: Vec<&str> = [artist, genre]
            .into_iter()
            .chain(genres.iter().map(|g| Some(g.as_str())))
            .chain([year.as_deref()])
            .flatten()
            .filter(|f| !f.is_empty())
            .collect();
        Self { search_name: normalize(name), search_text: normalize(&text.join("\n")) }
    }
}

/// Fills the search columns of the items already in the cache (migration 9;
/// migration 10 then builds the indexes from them).
pub(super) fn fill_search_columns(conn: &Connection) -> DbResult<()> {
    {
        let mut read = conn.prepare("SELECT id, title FROM songs")?;
        let mut write = conn.prepare("UPDATE songs SET search_name = ?2 WHERE id = ?1")?;
        let mut rows = read.query([])?;
        while let Some(row) = rows.next()? {
            write.execute(params![row.get::<_, String>(0)?, normalize(&row.get::<_, String>(1)?)])?;
        }
    }
    {
        let mut read = conn.prepare("SELECT id, name, artist, year, genre, genres FROM albums")?;
        let mut write = conn.prepare("UPDATE albums SET search_name = ?2, search_text = ?3 WHERE id = ?1")?;
        let mut rows = read.query([])?;
        while let Some(row) = rows.next()? {
            let genres: Vec<String> = serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default();
            let columns = AlbumSearchColumns::new(
                &row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.as_deref(),
                row.get(3)?,
                row.get::<_, Option<String>>(4)?.as_deref(),
                &genres,
            );
            write.execute(params![row.get::<_, String>(0)?, columns.search_name, columns.search_text])?;
        }
    }
    {
        let mut read = conn.prepare("SELECT id, name FROM artists")?;
        let mut write = conn.prepare("UPDATE artists SET search_name = ?2 WHERE id = ?1")?;
        let mut rows = read.query([])?;
        while let Some(row) = rows.next()? {
            write.execute(params![row.get::<_, String>(0)?, normalize(&row.get::<_, String>(1)?)])?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Recent searches
// ---------------------------------------------------------------------------

/// The last searches, newest first.
pub fn recent_searches(conn: &Connection) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT query FROM recent_searches ORDER BY searched_at DESC LIMIT ?1")?;
    let searches = stmt.query_map([MAX_RECENT_SEARCHES as i64], |row| row.get(0))?.collect::<Result<_, _>>()?;
    Ok(searches)
}

/// "Beyoncé", "beyonce" and " BEYONCE " are the same search.
fn recent_key(query: &str) -> String {
    query_words(query).join(" ")
}

/// Puts a search first in the recent list (`now` in Unix milliseconds); the
/// oldest beyond `MAX_RECENT_SEARCHES` are dropped.
pub fn add_recent_search(conn: &Connection, query: &str, now: i64) -> DbResult<()> {
    let key = recent_key(query);
    if key.is_empty() {
        return Ok(());
    }
    let shown = query.split_whitespace().collect::<Vec<_>>().join(" ");
    conn.execute(
        "INSERT INTO recent_searches (key, query, searched_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET query = excluded.query, searched_at = excluded.searched_at",
        params![key, shown, now],
    )?;
    conn.execute(
        "DELETE FROM recent_searches
          WHERE key NOT IN (SELECT key FROM recent_searches ORDER BY searched_at DESC LIMIT ?1)",
        [MAX_RECENT_SEARCHES as i64],
    )?;
    Ok(())
}

pub fn remove_recent_search(conn: &Connection, query: &str) -> DbResult<()> {
    conn.execute("DELETE FROM recent_searches WHERE key = ?1", [recent_key(query)])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::library::{apply_light_sync, begin_full_sync, finish_full_sync, stage};
    use crate::db::open_in_memory;
    use crate::subsonic::{Album, Artist, Genre, LibraryItems, Song};

    fn artist(id: &str, name: &str) -> Artist {
        Artist { id: id.into(), name: name.into(), raw_json: "{}".into(), ..Default::default() }
    }

    fn album(id: &str, name: &str, artist: &Artist) -> Album {
        Album {
            id: id.into(),
            name: name.into(),
            artist: Some(artist.name.clone()),
            artist_id: Some(artist.id.clone()),
            raw_json: "{}".into(),
            ..Default::default()
        }
    }

    fn song(id: &str, title: &str, artist: &Artist, plays: i64) -> Song {
        Song {
            id: id.into(),
            title: title.into(),
            artist: Some(artist.name.clone()),
            artist_id: Some(artist.id.clone()),
            play_count: Some(plays),
            raw_json: "{}".into(),
            ..Default::default()
        }
    }

    fn sync(conn: &mut Connection, items: LibraryItems) {
        begin_full_sync(conn).unwrap();
        stage(conn, &items).unwrap();
        finish_full_sync(conn, None, 1).unwrap();
    }

    fn library(items: LibraryItems) -> Connection {
        let mut conn = open_in_memory().unwrap();
        sync(&mut conn, items);
        conn
    }

    fn song_ids(conn: &Connection, query: &str) -> Vec<String> {
        search(conn, query).unwrap().song_ids
    }

    #[test]
    fn every_word_starts_a_word_of_some_field_in_any_order() {
        let elis = artist("elis", "Elis Regina");
        let mut aguas = song("a", "Águas de Março", &elis, 0);
        aguas.genres = vec![Genre { name: "MPB".into() }];
        let conn = library(LibraryItems {
            artists: vec![elis.clone()],
            songs: vec![aguas, song("b", "Outra", &elis, 0)],
            ..Default::default()
        });

        assert_eq!(song_ids(&conn, "MARC agu"), ["a"], "case, accents, order and prefixes");
        assert_eq!(song_ids(&conn, "reg águas"), ["a"], "the artist is a field too");
        assert_eq!(song_ids(&conn, "mpb"), ["a"], "and the genre");
        assert!(song_ids(&conn, "arco").is_empty(), "only the start of a word");
        assert!(song_ids(&conn, "aguas rock").is_empty(), "every word must be found");
        assert_eq!(search(&conn, " %& ").unwrap(), SearchResults::default());
    }

    #[test]
    fn songs_rank_by_how_the_title_matches_then_by_plays() {
        let band = artist("x", "Band");
        let lovebirds = artist("y", "Lovebirds");
        let conn = library(LibraryItems {
            artists: vec![band.clone(), lovebirds.clone()],
            songs: vec![
                song("other", "Other", &lovebirds, 90),
                song("inside", "I Don't Know How to Love", &band, 50),
                song("rock", "Lovers Rock", &band, 1),
                song("better", "Love Me Better", &band, 7),
                song("love", "Love", &band, 0),
            ],
            ..Default::default()
        });
        // Exact, then the titles starting with it (most played first), then
        // the word inside the title, then the artist's name.
        assert_eq!(song_ids(&conn, "love"), ["love", "better", "rock", "inside", "other"]);
        let results = search(&conn, "love").unwrap();
        assert_eq!(results.top_songs.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), ["love", "better", "rock", "inside"]);
    }

    #[test]
    fn the_best_result_prefers_the_strongest_match_then_artists() {
        let beatles = artist("beatles", "The Beatles");
        let lincoln = artist("lincoln", "Abbey Lincoln");
        let conn = library(LibraryItems {
            artists: vec![beatles.clone(), lincoln.clone()],
            albums: vec![album("sale", "Beatles for Sale", &beatles), album("road", "Abbey Road", &beatles)],
            songs: vec![song("love", "Love", &beatles, 0), song("lonely", "Lonely", &lincoln, 0)],
        });
        let best = |q: &str| search(&conn, q).unwrap().best;
        assert_eq!(best("beatles"), Some(ResultKind::Artist), "artist and album both have the word");
        assert_eq!(best("abbey road"), Some(ResultKind::Album), "the album's exact name");
        assert_eq!(best("abbey"), Some(ResultKind::Artist));
        assert_eq!(best("love"), Some(ResultKind::Song));
        assert_eq!(best("nothing"), None);

        let results = search(&conn, "beatles").unwrap();
        assert_eq!(results.artists[0].name, "The Beatles");
        // "Beatles for Sale" by its name first, then the other album by its artist.
        assert_eq!(results.albums.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["sale", "road"]);
    }

    #[test]
    fn artists_are_album_and_featured_artists_never_combined_names() {
        let main = artist("main", "Main");
        let guest = artist("guest", "Guest");
        let producer = artist("producer", "Main Producer");
        let mut feat = song("s1", "Duet", &main, 0);
        feat.artist = Some("Main feat. Guest".into());
        feat.raw_json = r#"{"artists":[{"id":"main"},{"id":"guest"}]}"#.into();
        let conn = library(LibraryItems {
            artists: vec![main.clone(), guest, producer],
            albums: vec![album("al", "Record", &main)],
            songs: vec![feat],
        });
        let names = |q: &str| search(&conn, q).unwrap().artists.into_iter().map(|a| a.name).collect::<Vec<_>>();
        assert_eq!(names("guest"), ["Guest"]);
        assert_eq!(names("main"), ["Main"], "the producer has only credits");
        assert!(names("feat").is_empty());
        assert_eq!(song_ids(&conn, "feat guest"), ["s1"], "the song still finds its display artist");
        assert_eq!(search(&conn, "main").unwrap().artists[0].album_count, 1);
    }

    #[test]
    fn the_indexes_follow_the_syncs() {
        let band = artist("x", "Band");
        let mut conn = library(LibraryItems {
            artists: vec![band.clone()],
            songs: vec![song("s1", "Alpha", &band, 0), song("s2", "Gamma", &band, 0)],
            ..Default::default()
        });
        assert_eq!(song_ids(&conn, "alpha"), ["s1"]);

        // Renamed, removed and added by a full sync.
        sync(&mut conn, LibraryItems {
            artists: vec![artist("x", "Banda Nova")],
            songs: vec![song("s1", "Beta", &band, 0), song("s3", "Delta", &band, 0)],
            ..Default::default()
        });
        assert!(song_ids(&conn, "alpha").is_empty());
        assert_eq!(song_ids(&conn, "beta"), ["s1"]);
        assert!(song_ids(&conn, "gamma").is_empty());
        assert_eq!(song_ids(&conn, "delta"), ["s3"]);
        assert_eq!(search(&conn, "nova").unwrap().artists.len(), 1);

        // A song starred in another app arrives with the light sync.
        let mut starred = song("s4", "Epsilon", &band, 0);
        starred.starred = Some("2026-10-01T00:00:00Z".into());
        apply_light_sync(&mut conn, &LibraryItems { songs: vec![starred], ..Default::default() }, &[], 2).unwrap();
        assert_eq!(song_ids(&conn, "epsilon"), ["s4"]);

        let check: rusqlite::Result<()> =
            conn.execute_batch("INSERT INTO songs_fts (songs_fts, rank) VALUES ('integrity-check', 1)");
        assert!(check.is_ok(), "{check:?}");
    }

    #[test]
    fn recent_searches_keep_the_last_ten_without_repeats() {
        let conn = open_in_memory().unwrap();
        for i in 0..12 {
            add_recent_search(&conn, &format!("search {i}"), i).unwrap();
        }
        let recent = recent_searches(&conn).unwrap();
        assert_eq!(recent.len(), 10);
        assert_eq!(recent[0], "search 11");
        assert_eq!(recent[9], "search 2");

        add_recent_search(&conn, "  Beyoncé   live ", 20).unwrap();
        add_recent_search(&conn, "beyonce LIVE", 21).unwrap();
        let recent = recent_searches(&conn).unwrap();
        assert_eq!(recent[0], "beyonce LIVE", "the same search moves up, as typed last");
        assert_eq!(recent.iter().filter(|r| r.to_lowercase().starts_with("beyonc")).count(), 1);

        remove_recent_search(&conn, "Beyoncé live").unwrap();
        assert_eq!(recent_searches(&conn).unwrap()[0], "search 11");
        add_recent_search(&conn, " ?! ", 30).unwrap();
        assert_eq!(recent_searches(&conn).unwrap()[0], "search 11", "nothing to search: not kept");
    }
}
