//! Data of the Home screen (spec 6.1): six rows of up to 20 cards.
//!
//! "Listen again" and "Most played albums" come from the server when it can
//! be reached (they include plays in other apps), with the local cache as
//! the fallback; the favorites rows come from the local cache (light sync).

use rusqlite::Connection;
use serde::Serialize;

use crate::db::DbResult;
use crate::db::home::{self, AlbumCard, ArtistCard};
use crate::db::library::SongSummary;
use crate::subsonic::Album;
use crate::time::parse_timestamp_ms;

/// Cards per row (spec 6.1).
pub const ROW_SIZE: usize = 20;

/// A card of "Listen again", which mixes albums and songs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ListenAgainItem {
    Album(AlbumCard),
    Song(SongSummary),
}

/// A row of songs: the cards shown, and every song of the list it stands
/// for, in order (a click plays the whole list from that song).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongRow {
    pub songs: Vec<SongSummary>,
    pub all_ids: Vec<String>,
}

impl SongRow {
    fn new(all: Vec<SongSummary>) -> Self {
        let all_ids = all.iter().map(|s| s.id.clone()).collect();
        Self { songs: all.into_iter().take(ROW_SIZE).collect(), all_ids }
    }
}

/// Rows read from the local cache.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeLibrary {
    pub starred_songs: SongRow,
    pub starred_albums: Vec<AlbumCard>,
    pub forgotten_starred: SongRow,
    pub starred_artists: Vec<ArtistCard>,
}

/// Rows that come from the server when possible.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeActivity {
    pub listen_again: Vec<ListenAgainItem>,
    pub most_played_albums: Vec<AlbumCard>,
    /// `false` when the local cache stood in for the server.
    pub from_server: bool,
}

pub fn library(conn: &Connection) -> DbResult<HomeLibrary> {
    Ok(HomeLibrary {
        starred_songs: SongRow::new(home::starred_songs(conn)?),
        starred_albums: home::starred_albums(conn, ROW_SIZE as u32)?,
        forgotten_starred: SongRow::new(home::forgotten_starred_songs(conn)?),
        starred_artists: home::starred_artists(conn, ROW_SIZE as u32)?,
    })
}

/// Builds the server rows. `recent` and `frequent` are the server's album
/// lists, `None` if it could not be reached.
pub fn activity(conn: &Connection, recent: Option<Vec<Album>>, frequent: Option<Vec<Album>>) -> DbResult<HomeActivity> {
    let from_server = recent.is_some() && frequent.is_some();

    let albums: Vec<(AlbumCard, Option<i64>)> = match recent {
        Some(albums) => albums.iter().map(|a| (card_of(a), a.played.as_deref().and_then(parse_timestamp_ms))).collect(),
        None => home::recently_played_albums(conn, ROW_SIZE as u32)?
            .into_iter()
            .map(|(card, played)| (card, parse_timestamp_ms(&played)))
            .collect(),
    };
    let songs = home::recently_played_songs(conn, ROW_SIZE as u32)?
        .into_iter()
        .map(|(song, played_secs)| (song, played_secs * 1000))
        .collect();

    let most_played_albums = match frequent {
        Some(albums) => albums.iter().take(ROW_SIZE).map(card_of).collect(),
        None => home::most_played_albums(conn, ROW_SIZE as u32)?,
    };
    Ok(HomeActivity { listen_again: listen_again(albums, songs, ROW_SIZE), most_played_albums, from_server })
}

/// "Listen again": albums and songs mixed, newest play first. A song is left
/// out when its album is in the row. Times are Unix milliseconds; an album
/// without a date goes after the dated items.
fn listen_again(
    albums: Vec<(AlbumCard, Option<i64>)>,
    songs: Vec<(SongSummary, i64)>,
    limit: usize,
) -> Vec<ListenAgainItem> {
    let album_ids: Vec<String> = albums.iter().map(|(album, _)| album.id.clone()).collect();
    let mut items: Vec<(i64, ListenAgainItem)> = albums
        .into_iter()
        .map(|(album, played)| (played.unwrap_or(i64::MIN), ListenAgainItem::Album(album)))
        .chain(
            songs
                .into_iter()
                .filter(|(song, _)| song.album_id.as_ref().is_none_or(|id| !album_ids.contains(id)))
                .map(|(song, played)| (played, ListenAgainItem::Song(song))),
        )
        .collect();
    // Stable: on the same time, albums (from the server) stay first.
    items.sort_by_key(|(played, _)| std::cmp::Reverse(*played));
    items.into_iter().take(limit).map(|(_, item)| item).collect()
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

    fn album(id: &str) -> AlbumCard {
        AlbumCard { id: id.into(), name: id.into(), artist: None, artist_id: None, cover_art: None }
    }

    fn song(id: &str, album_id: &str) -> SongSummary {
        SongSummary {
            id: id.into(),
            title: id.into(),
            artist: None,
            album: None,
            artist_id: None,
            album_id: Some(album_id.into()),
            cover_art: None,
            duration: None,
            starred: false,
        }
    }

    fn names(items: &[ListenAgainItem]) -> Vec<String> {
        items
            .iter()
            .map(|item| match item {
                ListenAgainItem::Album(a) => format!("album:{}", a.id),
                ListenAgainItem::Song(s) => format!("song:{}", s.id),
            })
            .collect()
    }

    #[test]
    fn listen_again_mixes_albums_and_songs_newest_first() {
        let albums = vec![(album("al1"), Some(5_000)), (album("al2"), Some(1_000))];
        let songs = vec![(song("s1", "al9"), 3_000), (song("s2", "al8"), 6_000)];
        assert_eq!(
            names(&listen_again(albums, songs, 20)),
            ["song:s2", "album:al1", "song:s1", "album:al2"]
        );
    }

    #[test]
    fn a_song_whose_album_is_in_the_row_is_left_out() {
        let albums = vec![(album("al1"), Some(5_000))];
        let songs = vec![(song("s1", "al1"), 9_000), (song("s2", "al2"), 4_000)];
        assert_eq!(names(&listen_again(albums, songs, 20)), ["album:al1", "song:s2"]);
    }

    #[test]
    fn listen_again_keeps_at_most_the_limit_and_undated_albums_go_last() {
        let albums = vec![(album("undated"), None), (album("al1"), Some(2_000))];
        let songs = (0..30).map(|i| (song(&format!("s{i}"), "x"), 10_000 + i)).collect();
        let items = listen_again(albums, songs, 20);
        assert_eq!(items.len(), 20);
        assert_eq!(names(&items)[0], "song:s29");

        let few = listen_again(vec![(album("undated"), None), (album("al1"), Some(2_000))], vec![], 20);
        assert_eq!(names(&few), ["album:al1", "album:undated"]);
    }

    #[test]
    fn song_rows_show_twenty_cards_but_keep_every_id() {
        let all: Vec<SongSummary> = (0..25).map(|i| song(&format!("s{i}"), "al")).collect();
        let row = SongRow::new(all);
        assert_eq!(row.songs.len(), 20);
        assert_eq!(row.all_ids.len(), 25);
        assert_eq!(row.all_ids[24], "s24");
    }

    #[test]
    fn listen_again_items_say_their_kind() {
        let item = ListenAgainItem::Album(album("al1"));
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["kind"], "album");
        assert_eq!(json["id"], "al1");
    }
}
