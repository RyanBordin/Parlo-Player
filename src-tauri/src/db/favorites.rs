//! Reads for the Favorites page (spec 6.2): everything starred, last starred
//! first. `starred` is the date the server (or this app) recorded the star;
//! `julianday` compares it whatever its format (fractions of a second, time
//! zone offset).

use std::collections::HashMap;

use rusqlite::Connection;
use serde::Serialize;

use super::DbResult;
use super::home::{self, AlbumCard, ArtistCard};
use super::tracks::{TRACK_COLUMNS, TrackListSong, track_from_row};

/// Items of the "Recent" row.
pub const RECENT_SIZE: u32 = 20;

/// A card of the "Recent" row: albums, artists and songs mixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FavoriteItem {
    Album(AlbumCard),
    Artist(ArtistCard),
    Song(TrackListSong),
}

/// Everything the Favorites page shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Favorites {
    /// The last items starred, of any kind.
    pub recent: Vec<FavoriteItem>,
    pub albums: Vec<AlbumCard>,
    pub artists: Vec<ArtistCard>,
    pub songs: Vec<TrackListSong>,
}

pub fn favorites(conn: &Connection) -> DbResult<Favorites> {
    let albums = home::starred_albums(conn, None)?;
    let artists = home::starred_artists(conn, None)?;
    let songs = starred_tracks(conn)?;
    let recent = recent(conn, &albums, &artists, &songs)?;
    Ok(Favorites { recent, albums, artists, songs })
}

/// Starred songs with every tracklist field, last starred first.
pub fn starred_tracks(conn: &Connection) -> DbResult<Vec<TrackListSong>> {
    let sql = format!(
        "SELECT {TRACK_COLUMNS} FROM songs WHERE starred IS NOT NULL
         ORDER BY julianday(starred) DESC, title"
    );
    let mut stmt = conn.prepare(&sql)?;
    let songs = stmt.query_map([], track_from_row)?.collect::<Result<_, _>>()?;
    Ok(songs)
}

/// The last starred items of the three kinds, taken from the lists already read.
fn recent(
    conn: &Connection,
    albums: &[AlbumCard],
    artists: &[ArtistCard],
    songs: &[TrackListSong],
) -> DbResult<Vec<FavoriteItem>> {
    let mut stmt = conn.prepare(
        "SELECT kind, id FROM (
             SELECT 'album' AS kind, id, starred FROM albums WHERE starred IS NOT NULL
             UNION ALL SELECT 'artist', id, starred FROM artists WHERE starred IS NOT NULL
             UNION ALL SELECT 'song', id, starred FROM songs WHERE starred IS NOT NULL
         )
         ORDER BY julianday(starred) DESC, kind, id LIMIT ?1",
    )?;
    let order: Vec<(String, String)> = stmt
        .query_map([RECENT_SIZE], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;

    let albums: HashMap<&str, &AlbumCard> = albums.iter().map(|a| (a.id.as_str(), a)).collect();
    let artists: HashMap<&str, &ArtistCard> = artists.iter().map(|a| (a.id.as_str(), a)).collect();
    let songs: HashMap<&str, &TrackListSong> = songs.iter().map(|s| (s.id.as_str(), s)).collect();
    Ok(order
        .iter()
        .filter_map(|(kind, id)| match kind.as_str() {
            "album" => albums.get(id.as_str()).map(|a| FavoriteItem::Album((*a).clone())),
            "artist" => artists.get(id.as_str()).map(|a| FavoriteItem::Artist((*a).clone())),
            _ => songs.get(id.as_str()).map(|s| FavoriteItem::Song((*s).clone())),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use rusqlite::params;

    fn add(conn: &Connection, table: &str, id: &str, starred: Option<&str>) {
        let name = if table == "songs" { "title" } else { "name" };
        conn.execute(
            &format!("INSERT INTO {table} (id, {name}, starred, raw_json) VALUES (?1, ?1, ?2, '{{}}')"),
            params![id, starred],
        )
        .unwrap();
    }

    fn kinds_and_ids(items: &[FavoriteItem]) -> Vec<(&str, &str)> {
        items
            .iter()
            .map(|item| match item {
                FavoriteItem::Album(a) => ("album", a.id.as_str()),
                FavoriteItem::Artist(a) => ("artist", a.id.as_str()),
                FavoriteItem::Song(s) => ("song", s.id.as_str()),
            })
            .collect()
    }

    #[test]
    fn songs_follow_the_star_date_in_any_format() {
        let conn = open_in_memory().unwrap();
        // 07:00 at -03:00 is 10:00 UTC: the latest, although it sorts first as text.
        add(&conn, "songs", "offset", Some("2026-10-17T07:00:00-03:00"));
        add(&conn, "songs", "nanos", Some("2026-10-17T08:00:00.123456789Z"));
        add(&conn, "songs", "app", Some("2026-10-17T09:00:00Z"));
        add(&conn, "songs", "not", None);

        let ids: Vec<String> = starred_tracks(&conn).unwrap().into_iter().map(|s| s.id).collect();
        assert_eq!(ids, ["offset", "app", "nanos"]);
    }

    #[test]
    fn recent_mixes_kinds_by_date() {
        let conn = open_in_memory().unwrap();
        add(&conn, "albums", "al1", Some("2026-10-01T00:00:00Z"));
        add(&conn, "artists", "ar1", Some("2026-10-03T00:00:00Z"));
        add(&conn, "songs", "s1", Some("2026-10-02T00:00:00Z"));
        add(&conn, "songs", "s2", None);

        let all = favorites(&conn).unwrap();
        assert_eq!(kinds_and_ids(&all.recent), [("artist", "ar1"), ("song", "s1"), ("album", "al1")]);
        assert_eq!(all.songs.len(), 1, "only starred songs");
        assert_eq!(all.albums.len(), 1);
        assert_eq!(all.artists.len(), 1);
    }

    #[test]
    fn recent_keeps_the_last_20_but_the_lists_keep_everything() {
        let conn = open_in_memory().unwrap();
        for i in 0..15 {
            add(&conn, "songs", &format!("s{i:02}"), Some(&format!("2026-01-{:02}T00:00:00Z", i + 1)));
            add(&conn, "albums", &format!("al{i:02}"), Some(&format!("2026-02-{:02}T00:00:00Z", i + 1)));
        }
        let all = favorites(&conn).unwrap();
        assert_eq!(all.recent.len(), RECENT_SIZE as usize);
        assert_eq!(kinds_and_ids(&all.recent)[0], ("album", "al14"), "last starred first");
        assert_eq!(all.songs.len(), 15);
        assert_eq!(all.albums.len(), 15);
    }

    #[test]
    fn items_say_their_kind() {
        let conn = open_in_memory().unwrap();
        add(&conn, "songs", "s1", Some("2026-10-01T00:00:00Z"));
        let json = serde_json::to_value(&favorites(&conn).unwrap().recent[0]).unwrap();
        assert_eq!(json["kind"], "song");
        assert_eq!(json["id"], "s1");
        assert_eq!(json["starred"], true);
    }
}
