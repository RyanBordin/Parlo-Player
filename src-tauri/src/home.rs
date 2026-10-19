//! Data of the Home screen (spec 6.1).
//!
//! Song sections ("Listen again", favorites, forgotten favorites) play their
//! whole list ("Play all"); album and artist rows show up to 20 cards and
//! open a full grid ("See more"). "Most played albums" comes from the server
//! when it can be reached (it includes plays in other apps), with the local
//! cache as the fallback; everything else comes from the local cache.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::DbResult;
use crate::db::home::{self, AlbumCard, ArtistCard};
use crate::db::library::SongSummary;
use crate::subsonic::{Album, AlbumListType};

/// Cards in a row of albums, artists or "Listen again" songs.
pub const ROW_SIZE: usize = 20;
/// Songs in a 3 × 4 song list: 4 pages of 12.
pub const TRACK_ROW_SIZE: usize = 48;

/// A card of a grid: album or artist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HomeItem {
    Album(AlbumCard),
    Artist(ArtistCard),
}

/// A song section: the songs shown, and every song of the list it stands
/// for, in order ("Play all" and a click play that whole list).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongRow {
    pub songs: Vec<SongSummary>,
    pub all_ids: Vec<String>,
}

impl SongRow {
    fn new(all: Vec<SongSummary>, shown: usize) -> Self {
        let all_ids = all.iter().map(|s| s.id.clone()).collect();
        Self { songs: all.into_iter().take(shown).collect(), all_ids }
    }
}

/// Sections read from the local cache.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeLibrary {
    pub listen_again: SongRow,
    pub starred_songs: SongRow,
    pub starred_albums: Vec<AlbumCard>,
    pub forgotten_starred: SongRow,
    pub starred_artists: Vec<ArtistCard>,
}

/// The section that comes from the server when possible.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeActivity {
    pub most_played_albums: Vec<AlbumCard>,
    /// The local cache stood in for the server.
    pub offline: bool,
}

/// The card rows that open a full grid ("See more").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HomeSection {
    MostPlayedAlbums,
    FavoriteAlbums,
    FavoriteArtists,
}

impl HomeSection {
    /// The server list this section comes from, if any.
    pub fn server_list(self) -> Option<AlbumListType> {
        match self {
            HomeSection::MostPlayedAlbums => Some(AlbumListType::Frequent),
            HomeSection::FavoriteAlbums | HomeSection::FavoriteArtists => None,
        }
    }
}

/// Every item of a section, for its grid page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionItems {
    pub items: Vec<HomeItem>,
    /// The local cache stood in for the server.
    pub offline: bool,
}

/// The local sections. `now` (Unix seconds) sets the 30 days of "Listen again".
pub fn library(conn: &Connection, now: i64) -> DbResult<HomeLibrary> {
    let row = Some(ROW_SIZE as u32);
    let recent = home::recently_played_top_songs(conn, now)?;
    // Enough to fill the row even if all the recent songs are among them.
    let top = if recent.len() < ROW_SIZE { home::most_played_songs(conn, Some(2 * ROW_SIZE as u32))? } else { Vec::new() };
    Ok(HomeLibrary {
        listen_again: SongRow::new(listen_again(recent, top), ROW_SIZE),
        starred_songs: SongRow::new(home::starred_songs(conn)?, TRACK_ROW_SIZE),
        starred_albums: home::starred_albums(conn, row)?,
        forgotten_starred: SongRow::new(home::forgotten_starred_songs(conn)?, TRACK_ROW_SIZE),
        starred_artists: home::starred_artists(conn, row)?,
    })
}

/// "Most played albums" from the server's list, or the local cache when
/// `frequent` is `None` (server not reached).
pub fn activity(conn: &Connection, frequent: Option<Vec<Album>>) -> DbResult<HomeActivity> {
    let offline = frequent.is_none();
    Ok(HomeActivity { most_played_albums: most_played(conn, frequent, Some(ROW_SIZE))?, offline })
}

/// Every item of a section. `server` is its server list (see
/// [`HomeSection::server_list`]), `None` if it could not be reached.
pub fn section(conn: &Connection, section: HomeSection, server: Option<Vec<Album>>) -> DbResult<SectionItems> {
    let offline = section.server_list().is_some() && server.is_none();
    let items = match section {
        HomeSection::MostPlayedAlbums => most_played(conn, server, None)?.into_iter().map(HomeItem::Album).collect(),
        HomeSection::FavoriteAlbums => home::starred_albums(conn, None)?.into_iter().map(HomeItem::Album).collect(),
        HomeSection::FavoriteArtists => home::starred_artists(conn, None)?.into_iter().map(HomeItem::Artist).collect(),
    };
    Ok(SectionItems { items, offline })
}

/// "Listen again" (spec 6.1): the songs of the last 30 days, most played
/// first. With fewer than a row of them, the most played songs of all time
/// fill the row, each song once.
fn listen_again(recent: Vec<SongSummary>, top: Vec<SongSummary>) -> Vec<SongSummary> {
    let mut songs = recent;
    for song in top {
        if songs.len() >= ROW_SIZE {
            break;
        }
        if !songs.iter().any(|s| s.id == song.id) {
            songs.push(song);
        }
    }
    songs
}

fn most_played(conn: &Connection, frequent: Option<Vec<Album>>, limit: Option<usize>) -> DbResult<Vec<AlbumCard>> {
    Ok(match frequent {
        Some(albums) => albums.iter().take(limit.unwrap_or(usize::MAX)).map(card_of).collect(),
        None => home::most_played_albums(conn, limit.map(|n| n as u32))?,
    })
}

fn card_of(album: &Album) -> AlbumCard {
    AlbumCard {
        id: album.id.clone(),
        name: album.name.clone(),
        artist: album.artist.clone(),
        artist_id: album.artist_id.clone(),
        cover_art: album.cover_art.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use rusqlite::params;

    /// 2026-10-18T12:00:00Z.
    const NOW: i64 = 1_792_324_800;

    fn song(id: &str) -> SongSummary {
        SongSummary {
            id: id.into(),
            title: id.into(),
            artist: None,
            album: None,
            artist_id: None,
            album_id: None,
            cover_art: None,
            duration: None,
            starred: false,
        }
    }

    fn songs(prefix: &str, n: usize) -> Vec<SongSummary> {
        (0..n).map(|i| song(&format!("{prefix}{i}"))).collect()
    }

    fn ids(songs: &[SongSummary]) -> Vec<&str> {
        songs.iter().map(|s| s.id.as_str()).collect()
    }

    #[test]
    fn a_full_month_needs_no_filling() {
        let recent = songs("r", 25);
        let all = listen_again(recent, songs("t", 5));
        assert_eq!(all.len(), 25, "the whole month, not cut at 20");
        assert!(all.iter().all(|s| s.id.starts_with('r')));
    }

    #[test]
    fn a_short_month_is_filled_up_to_20_without_repeats() {
        let recent = vec![song("a"), song("b")];
        let top = vec![song("c"), song("a"), song("d")];
        assert_eq!(ids(&listen_again(recent, top)), ["a", "b", "c", "d"]);

        let filled = listen_again(songs("r", 3), songs("t", 40));
        assert_eq!(filled.len(), 20);
        assert_eq!(ids(&filled)[..4], ["r0", "r1", "r2", "t0"]);
    }

    #[test]
    fn nothing_played_leaves_listen_again_empty() {
        let conn = open_in_memory().unwrap();
        conn.execute("INSERT INTO songs (id, title, raw_json) VALUES ('s1', 's1', '{}')", []).unwrap();
        let rows = library(&conn, NOW).unwrap();
        assert!(rows.listen_again.songs.is_empty());
        assert!(rows.listen_again.all_ids.is_empty());
    }

    #[test]
    fn listen_again_from_the_database_fills_with_all_time_favorites() {
        let conn = open_in_memory().unwrap();
        let add = |id: &str, plays: i64, played: &str| {
            conn.execute(
                "INSERT INTO songs (id, title, play_count, played, raw_json) VALUES (?1, ?1, ?2, ?3, '{}')",
                params![id, plays, played],
            )
            .unwrap();
        };
        add("recent", 2, "2026-10-17T08:00:00.123456789Z");
        add("old-hit", 90, "2025-01-01T00:00:00Z");
        add("old", 1, "2025-06-01T00:00:00Z");

        let row = library(&conn, NOW).unwrap().listen_again;
        assert_eq!(ids(&row.songs), ["recent", "old-hit", "old"]);
        assert_eq!(row.all_ids, ["recent", "old-hit", "old"]);
    }

    #[test]
    fn song_rows_show_their_cards_but_keep_every_id() {
        let row = SongRow::new(songs("s", 60), TRACK_ROW_SIZE);
        assert_eq!(row.songs.len(), 48);
        assert_eq!(row.all_ids.len(), 60);
        assert_eq!(row.all_ids[59], "s59");
    }

    #[test]
    fn sections_have_every_item_without_the_row_limit() {
        let conn = open_in_memory().unwrap();
        for i in 0..25 {
            conn.execute(
                "INSERT INTO albums (id, name, starred, raw_json) VALUES (?1, ?1, ?2, '{}')",
                params![format!("al{i:02}"), format!("2026-01-{:02}T00:00:00Z", i + 1)],
            )
            .unwrap();
        }
        assert_eq!(library(&conn, NOW).unwrap().starred_albums.len(), ROW_SIZE);

        let full = section(&conn, HomeSection::FavoriteAlbums, None).unwrap();
        assert_eq!(full.items.len(), 25);
        assert!(!full.offline, "favorites never need the server");
        assert!(matches!(&full.items[0], HomeItem::Album(a) if a.id == "al24"), "last starred first");
    }

    #[test]
    fn the_server_section_uses_every_server_album_or_says_it_is_offline() {
        let conn = open_in_memory().unwrap();
        let server: Vec<Album> = (0..30).map(|i| Album { id: format!("al{i}"), ..Album::default() }).collect();
        let full = section(&conn, HomeSection::MostPlayedAlbums, Some(server)).unwrap();
        assert_eq!(full.items.len(), 30);
        assert!(!full.offline);

        let local = section(&conn, HomeSection::MostPlayedAlbums, None).unwrap();
        assert!(local.offline);
        assert!(local.items.is_empty());
    }

    #[test]
    fn section_names_come_from_the_route() {
        let parse = |name: &str| serde_json::from_value::<HomeSection>(serde_json::json!(name)).ok();
        assert_eq!(parse("most-played-albums"), Some(HomeSection::MostPlayedAlbums));
        assert_eq!(parse("favorite-artists"), Some(HomeSection::FavoriteArtists));
        assert_eq!(parse("listen-again"), None, "a song section, without a grid");
    }

    #[test]
    fn items_say_their_kind() {
        let album = AlbumCard { id: "al1".into(), name: "A".into(), artist: None, artist_id: None, cover_art: None };
        let json = serde_json::to_value(HomeItem::Album(album)).unwrap();
        assert_eq!(json["kind"], "album");
        assert_eq!(json["id"], "al1");
    }
}
