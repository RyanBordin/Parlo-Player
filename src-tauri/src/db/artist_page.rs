//! Artist page (spec 6.7) from the local cache: the header, every song the
//! artist appears on, their albums (singles and EPs apart when the server
//! gives release types), the albums they appear on, and what
//! `getArtistInfo2` said, kept in `artist_info`.

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use super::DbResult;
use super::artists::FALLBACK_COVER;
use super::home::{ALBUM_CARD_COLUMNS, AlbumCard, ArtistCard, album_card_from_row};
use super::tracks::{TRACK_COLUMNS, TrackListSong, track_from_row};

/// Header of an artist page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistHeader {
    pub id: String,
    pub name: String,
    /// The server's photo (may be its generic image, see `ArtistAbout`).
    pub cover_art: Option<String>,
    /// The latest album's cover: the banner when there is no photo.
    pub fallback_cover_art: Option<String>,
    pub starred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistPage {
    pub artist: ArtistHeader,
    /// Every song the artist appears on (featured included): most played
    /// first; on a tie, by album, disc and track.
    pub songs: Vec<TrackListSong>,
    /// The artist's albums, newest first; singles and EPs are apart when the
    /// server gives release types (OpenSubsonic `releaseTypes`).
    pub albums: Vec<AlbumCard>,
    pub singles_and_eps: Vec<AlbumCard>,
    /// Albums of other artists with songs of this one, newest first.
    pub appears_on: Vec<AlbumCard>,
}

/// Newest first; on a tie, the last added.
const ALBUM_ORDER: &str = "ORDER BY year DESC NULLS LAST, julianday(created) DESC, name";

/// Songs where `?1` is one of the song's artists.
const ARTIST_SONGS: &str = "SELECT song_id FROM song_artists WHERE artist_id = ?1 AND role = 'artist'";

/// The page of an artist, `None` if the artist is not in the library.
pub fn artist_page(conn: &Connection, artist_id: &str) -> DbResult<Option<ArtistPage>> {
    let header = conn
        .query_row(
            &format!(
                "SELECT ar.id, ar.name, ar.cover_art, {FALLBACK_COVER}, ar.starred IS NOT NULL
                 FROM artists ar WHERE ar.id = ?1"
            ),
            [artist_id],
            |row| {
                Ok(ArtistHeader {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    cover_art: row.get(2)?,
                    fallback_cover_art: row.get(3)?,
                    starred: row.get(4)?,
                })
            },
        )
        .optional()?;
    let Some(artist) = header else { return Ok(None) };

    let mut stmt = conn.prepare(&format!("SELECT {TRACK_COLUMNS} FROM songs WHERE id IN ({ARTIST_SONGS}) {SONG_ORDER}"))?;
    let songs = stmt.query_map([artist_id], track_from_row)?.collect::<Result<_, _>>()?;

    // Main artist of the album, or one of its album artists (OpenSubsonic).
    let mut stmt = conn.prepare(&format!(
        "SELECT {ALBUM_CARD_COLUMNS},
                EXISTS (SELECT 1 FROM json_each(raw_json, '$.releaseTypes') t WHERE lower(t.value) IN ('single', 'ep'))
         FROM albums
         WHERE artist_id = ?1
            OR id IN (SELECT s.album_id FROM song_artists sa JOIN songs s ON s.id = sa.song_id
                       WHERE sa.artist_id = ?1 AND sa.role = 'albumartist')
         {ALBUM_ORDER}"
    ))?;
    let own: Vec<(AlbumCard, bool)> = stmt
        .query_map([artist_id], |row| Ok((album_card_from_row(row)?, row.get(5)?)))?
        .collect::<Result<_, _>>()?;
    let own_ids: HashSet<String> = own.iter().map(|(album, _)| album.id.clone()).collect();
    let (singles_and_eps, albums): (Vec<_>, Vec<_>) = own.into_iter().partition(|(_, single)| *single);

    let mut stmt = conn.prepare(&format!(
        "SELECT {ALBUM_CARD_COLUMNS} FROM albums
         WHERE id IN (SELECT album_id FROM songs WHERE id IN ({ARTIST_SONGS}))
         {ALBUM_ORDER}"
    ))?;
    let appears_on = stmt
        .query_map([artist_id], album_card_from_row)?
        .filter(|album| album.as_ref().map_or(true, |a| !own_ids.contains(&a.id)))
        .collect::<Result<_, _>>()?;

    Ok(Some(ArtistPage {
        artist,
        songs,
        albums: albums.into_iter().map(|(album, _)| album).collect(),
        singles_and_eps: singles_and_eps.into_iter().map(|(album, _)| album).collect(),
        appears_on,
    }))
}

/// Order of the artist's songs (spec 6.7).
const SONG_ORDER: &str = "ORDER BY COALESCE(play_count, 0) DESC, sort_album NULLS LAST, album_id,
    COALESCE(disc_number, 1), COALESCE(track, 0), sort_title";

/// IDs of the artist's songs, in the page's order.
pub fn song_ids(conn: &Connection, artist_id: &str) -> DbResult<Vec<String>> {
    let mut stmt = conn.prepare(&format!("SELECT id FROM songs WHERE id IN ({ARTIST_SONGS}) {SONG_ORDER}"))?;
    let ids = stmt.query_map([artist_id], |row| row.get(0))?.collect::<Result<_, _>>()?;
    Ok(ids)
}

/// What `getArtistInfo2` said about an artist, as kept in `artist_info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedInfo {
    /// Plain text.
    pub biography: Option<String>,
    /// Similar artists the server has, in its order.
    pub similar_ids: Vec<String>,
    /// Whether the photo is real (not the server's generic image); `None`: unknown.
    pub has_photo: Option<bool>,
    /// Unix seconds.
    pub fetched_at: i64,
}

/// What the artist page needs before asking the server. `None` if the
/// artist is not in the library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoInputs {
    pub name: String,
    pub cover_art: Option<String>,
    /// Some song of the artist was played on this server.
    pub has_plays: bool,
    /// The last answer kept, even if older than 7 days.
    pub cached: Option<CachedInfo>,
}

pub fn info_inputs(conn: &Connection, artist_id: &str) -> DbResult<Option<InfoInputs>> {
    let artist = conn
        .query_row(
            &format!(
                "SELECT name, cover_art,
                        EXISTS (SELECT 1 FROM songs WHERE id IN ({ARTIST_SONGS}) AND play_count > 0)
                 FROM artists WHERE id = ?1"
            ),
            [artist_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((name, cover_art, has_plays)) = artist else { return Ok(None) };
    Ok(Some(InfoInputs { name, cover_art, has_plays, cached: cached_info(conn, artist_id)? }))
}

fn cached_info(conn: &Connection, artist_id: &str) -> DbResult<Option<CachedInfo>> {
    let info = conn
        .query_row(
            "SELECT biography, similar_ids, has_photo, fetched_at FROM artist_info WHERE artist_id = ?1",
            [artist_id],
            |row| {
                let similar: String = row.get(1)?;
                Ok(CachedInfo {
                    biography: row.get(0)?,
                    similar_ids: serde_json::from_str(&similar).unwrap_or_default(),
                    has_photo: row.get(2)?,
                    fetched_at: row.get(3)?,
                })
            },
        )
        .optional()?;
    Ok(info)
}

/// Keeps a new answer of `getArtistInfo2`, replacing the old one.
pub fn save_info(conn: &Connection, artist_id: &str, info: &CachedInfo, raw_json: &str) -> DbResult<()> {
    let similar = serde_json::to_string(&info.similar_ids).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT OR REPLACE INTO artist_info (artist_id, biography, similar_ids, has_photo, fetched_at, raw_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![artist_id, info.biography, similar, info.has_photo, info.fetched_at, raw_json],
    )?;
    Ok(())
}

/// Cards of the artists in `ids` that are in the library, in that order,
/// without `except` (the page's own artist).
pub fn artist_cards(conn: &Connection, ids: &[String], except: &str) -> DbResult<Vec<ArtistCard>> {
    let ids = serde_json::to_string(ids).unwrap_or_else(|_| "[]".into());
    let mut stmt = conn.prepare(&format!(
        "SELECT ar.id, ar.name, ar.cover_art, {FALLBACK_COVER}
         FROM json_each(?1) j JOIN artists ar ON ar.id = j.value
         WHERE ar.id <> ?2
         ORDER BY j.key"
    ))?;
    let cards = stmt
        .query_map(params![ids, except], |row| {
            Ok(ArtistCard {
                id: row.get(0)?,
                name: row.get(1)?,
                cover_art: row.get(2)?,
                fallback_cover_art: row.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::artists::fill_song_artists;
    use crate::db::open_in_memory;

    fn artist(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO artists (id, name, cover_art, raw_json) VALUES (?1, 'Name ' || ?1, 'ar-' || ?1, '{}')",
            [id],
        )
        .unwrap();
    }

    fn album(conn: &Connection, id: &str, artist_id: &str, year: i64, raw: &str) {
        conn.execute(
            "INSERT INTO albums (id, name, artist_id, cover_art, year, raw_json) VALUES (?1, ?1, ?2, 'al-' || ?1, ?3, ?4)",
            params![id, artist_id, year, raw],
        )
        .unwrap();
    }

    /// A song of `album_id` by the artists in `artists` (OpenSubsonic list).
    fn song(conn: &Connection, id: &str, album_id: &str, track: i64, plays: i64, artists: &[&str]) {
        let list: Vec<String> = artists.iter().map(|a| format!(r#"{{"id":"{a}"}}"#)).collect();
        conn.execute(
            "INSERT INTO songs (id, title, album_id, sort_album, track, play_count, sort_title, raw_json)
             VALUES (?1, ?1, ?2, ?2, ?3, ?4, ?1, ?5)",
            params![id, album_id, track, plays, format!(r#"{{"artists":[{}]}}"#, list.join(","))],
        )
        .unwrap();
    }

    fn ids<T>(items: &[T], id: impl Fn(&T) -> &str) -> Vec<&str> {
        items.iter().map(id).collect()
    }

    #[test]
    fn songs_are_most_played_first_then_by_album_and_track() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "band");
        song(&conn, "b2", "b-album", 2, 0, &["band"]);
        song(&conn, "hit", "z-album", 9, 7, &["band"]);
        song(&conn, "b1", "b-album", 1, 0, &["band"]);
        song(&conn, "a1", "a-album", 1, 0, &["band", "guest"]);
        song(&conn, "other", "a-album", 2, 50, &["guest"]);
        fill_song_artists(&conn).unwrap();

        let page = artist_page(&conn, "band").unwrap().unwrap();
        assert_eq!(ids(&page.songs, |s| &s.id), ["hit", "a1", "b1", "b2"]);
        assert_eq!(song_ids(&conn, "band").unwrap(), ["hit", "a1", "b1", "b2"]);
        assert_eq!(page.artist.name, "Name band");
        assert_eq!(artist_page(&conn, "unknown").unwrap(), None);
    }

    #[test]
    fn albums_are_split_by_release_type_and_others_are_appears_on() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "band");
        album(&conn, "lp-old", "band", 2001, r#"{"releaseTypes":["Album"]}"#);
        album(&conn, "lp-new", "band", 2020, r#"{"releaseTypes":["Album","Live"]}"#);
        album(&conn, "single", "band", 2021, r#"{"releaseTypes":["Single"]}"#);
        album(&conn, "ep", "band", 2019, r#"{"releaseTypes":["EP"]}"#);
        album(&conn, "untyped", "band", 2010, "{}");
        album(&conn, "friend", "friend", 2015, "{}");
        song(&conn, "feat", "friend", 1, 0, &["friend", "band"]);
        song(&conn, "own", "lp-new", 1, 0, &["band"]);
        fill_song_artists(&conn).unwrap();

        let page = artist_page(&conn, "band").unwrap().unwrap();
        assert_eq!(ids(&page.albums, |a| &a.id), ["lp-new", "untyped", "lp-old"]);
        assert_eq!(ids(&page.singles_and_eps, |a| &a.id), ["single", "ep"]);
        assert_eq!(ids(&page.appears_on, |a| &a.id), ["friend"], "not the band's own album");
        assert_eq!(page.artist.fallback_cover_art.as_deref(), Some("al-single"), "latest album");
    }

    #[test]
    fn without_release_types_every_album_is_in_one_section() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "band");
        album(&conn, "a", "band", 2001, "{}");
        album(&conn, "b", "band", 2002, "{}");
        let page = artist_page(&conn, "band").unwrap().unwrap();
        assert_eq!(ids(&page.albums, |a| &a.id), ["b", "a"]);
        assert!(page.singles_and_eps.is_empty() && page.appears_on.is_empty() && page.songs.is_empty());
    }

    #[test]
    fn album_artists_of_opensubsonic_count_as_own_albums() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "duo-half");
        album(&conn, "duet", "duo", 2020, "{}");
        conn.execute(
            r#"INSERT INTO songs (id, title, album_id, raw_json)
               VALUES ('s1', 's1', 'duet', '{"artists":[{"id":"duo-half"}],"albumArtists":[{"id":"duo"},{"id":"duo-half"}]}')"#,
            [],
        )
        .unwrap();
        fill_song_artists(&conn).unwrap();
        let page = artist_page(&conn, "duo-half").unwrap().unwrap();
        assert_eq!(ids(&page.albums, |a| &a.id), ["duet"]);
        assert!(page.appears_on.is_empty());
    }

    #[test]
    fn info_is_kept_and_replaced() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "band");
        song(&conn, "s1", "al", 1, 0, &["band"]);
        fill_song_artists(&conn).unwrap();

        let inputs = info_inputs(&conn, "band").unwrap().unwrap();
        assert_eq!(inputs.cover_art.as_deref(), Some("ar-band"));
        assert!(!inputs.has_plays);
        assert_eq!(inputs.cached, None);
        assert_eq!(info_inputs(&conn, "unknown").unwrap(), None);

        let info = CachedInfo {
            biography: Some("A band.".into()),
            similar_ids: vec!["b".into(), "a".into()],
            has_photo: None,
            fetched_at: 100,
        };
        save_info(&conn, "band", &info, "{}").unwrap();
        assert_eq!(info_inputs(&conn, "band").unwrap().unwrap().cached, Some(info.clone()));

        let newer = CachedInfo { has_photo: Some(true), fetched_at: 200, ..info };
        save_info(&conn, "band", &newer, "{}").unwrap();
        assert_eq!(info_inputs(&conn, "band").unwrap().unwrap().cached, Some(newer));
    }

    #[test]
    fn a_played_song_counts_as_plays() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "band");
        song(&conn, "s1", "al", 1, 3, &["band"]);
        fill_song_artists(&conn).unwrap();
        assert!(info_inputs(&conn, "band").unwrap().unwrap().has_plays);
    }

    #[test]
    fn similar_cards_keep_only_library_artists_in_order() {
        let conn = open_in_memory().unwrap();
        for id in ["band", "x", "y"] {
            artist(&conn, id);
        }
        let similar = ["y", "gone", "band", "x"].map(String::from);
        let cards = artist_cards(&conn, &similar, "band").unwrap();
        assert_eq!(ids(&cards, |c| &c.id), ["y", "x"]);
        assert_eq!(cards[0].cover_art.as_deref(), Some("ar-y"));
    }
}
