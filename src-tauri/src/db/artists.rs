//! The Artists page (spec 6.6): who appears on each song (`song_artists`),
//! every artist with the fields its card, quick search and "Sort by" need,
//! and the songs of several artists at once.
//!
//! - Album artists: the OpenSubsonic `albumartist` role, or the main artist
//!   of an album (servers without OpenSubsonic).
//! - All artists: album artists plus anyone in a song's `artists` list
//!   (featured artists). People only in the credits (producer, engineer...)
//!   are left out.

use rusqlite::Connection;
use serde::Serialize;

use super::DbResult;

/// An artist of the Artists grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistListItem {
    pub id: String,
    pub name: String,
    pub sort_name: Option<String>,
    /// The server's photo.
    pub cover_art: Option<String>,
    /// Cover of the latest album (or, for featured artists, of their latest
    /// song): shown when the server has no photo.
    pub fallback_cover_art: Option<String>,
    pub starred: bool,
    pub album_count: i64,
    /// Plays of the artist's songs (the server has no count per artist).
    pub play_count: i64,
    /// When the artist's latest song was added (ISO 8601).
    pub added: Option<String>,
    /// Shown with the "Album artists" filter (the default).
    pub album_artist: bool,
}

/// Rebuilds `song_artists` from the songs' raw JSON: `artists` and
/// `albumArtists` (OpenSubsonic). Without them, the song's main artist and
/// its album's artist stand in.
pub fn fill_song_artists(conn: &Connection) -> DbResult<()> {
    conn.execute_batch(
        "DELETE FROM song_artists;
         INSERT OR IGNORE INTO song_artists (song_id, artist_id, role)
         SELECT s.id, json_extract(a.value, '$.id'), 'artist'
           FROM songs s, json_each(s.raw_json, '$.artists') a
          WHERE json_extract(a.value, '$.id') IS NOT NULL
         UNION ALL
         SELECT s.id, json_extract(a.value, '$.id'), 'albumartist'
           FROM songs s, json_each(s.raw_json, '$.albumArtists') a
          WHERE json_extract(a.value, '$.id') IS NOT NULL
         UNION ALL
         SELECT id, artist_id, 'artist' FROM songs
          WHERE artist_id IS NOT NULL AND json_type(raw_json, '$.artists') IS NULL
         UNION ALL
         SELECT s.id, al.artist_id, 'albumartist' FROM songs s JOIN albums al ON al.id = s.album_id
          WHERE al.artist_id IS NOT NULL AND json_type(s.raw_json, '$.albumArtists') IS NULL;",
    )?;
    Ok(())
}

/// SQL for the cover shown when the server has no photo of artist `ar`: the
/// latest album's cover or, for featured artists, the latest song's.
pub(super) const FALLBACK_COVER: &str = "COALESCE(
    (SELECT al.cover_art FROM albums al
      WHERE al.artist_id = ar.id AND al.cover_art IS NOT NULL
      ORDER BY al.year DESC NULLS LAST, julianday(al.created) DESC LIMIT 1),
    (SELECT s.cover_art FROM song_artists sa JOIN songs s ON s.id = sa.song_id
      WHERE sa.artist_id = ar.id AND s.cover_art IS NOT NULL
      ORDER BY s.year DESC NULLS LAST, julianday(s.created) DESC LIMIT 1)
)";

/// Album artists and featured artists, by name.
pub fn artist_list(conn: &Connection) -> DbResult<Vec<ArtistListItem>> {
    let mut stmt = conn.prepare(&format!(
        "WITH stats AS (
             SELECT l.artist_id, SUM(COALESCE(s.play_count, 0)) AS plays, MAX(julianday(s.created)) AS added
               FROM (SELECT DISTINCT artist_id, song_id FROM song_artists) l
               JOIN songs s ON s.id = l.song_id
              GROUP BY l.artist_id
         )
         SELECT ar.id, ar.name, ar.sort_name, ar.cover_art, {FALLBACK_COVER},
                ar.starred IS NOT NULL,
                COALESCE(ar.album_count, (SELECT COUNT(*) FROM albums al WHERE al.artist_id = ar.id)),
                COALESCE(st.plays, 0),
                strftime('%Y-%m-%dT%H:%M:%SZ', st.added),
                EXISTS (SELECT 1 FROM json_each(ar.roles) WHERE value = 'albumartist')
                    OR ar.id IN (SELECT artist_id FROM song_artists WHERE role = 'albumartist')
                    OR ar.id IN (SELECT artist_id FROM albums WHERE artist_id IS NOT NULL) AS album_artist
           FROM artists ar LEFT JOIN stats st ON st.artist_id = ar.id
          WHERE album_artist OR ar.id IN (SELECT artist_id FROM song_artists WHERE role = 'artist')
          ORDER BY COALESCE(ar.sort_name, ar.name) COLLATE NOCASE"
    ))?;
    let artists = stmt
        .query_map([], |row| {
            Ok(ArtistListItem {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_name: row.get(2)?,
                cover_art: row.get(3)?,
                fallback_cover_art: row.get(4)?,
                starred: row.get(5)?,
                album_count: row.get(6)?,
                play_count: row.get(7)?,
                added: row.get(8)?,
                album_artist: row.get(9)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(artists)
}

/// Every song where one of these artists appears, each once.
pub fn songs_of_artists(conn: &Connection, artist_ids: &[String]) -> DbResult<Vec<String>> {
    let ids = serde_json::to_string(artist_ids).unwrap_or_else(|_| "[]".into());
    let mut stmt = conn.prepare(
        "SELECT DISTINCT sa.song_id FROM song_artists sa
          WHERE sa.artist_id IN (SELECT value FROM json_each(?1))
            AND sa.song_id IN (SELECT id FROM songs)",
    )?;
    let songs = stmt.query_map([ids], |row| row.get(0))?.collect::<Result<_, _>>()?;
    Ok(songs)
}

/// Records a star / unstar of an artist done in the app (see
/// `library::set_song_starred`).
pub fn set_artist_starred(conn: &Connection, artist_id: &str, starred: bool) -> DbResult<()> {
    if starred {
        conn.execute(
            "UPDATE artists SET starred = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                raw_json = json_set(raw_json, '$.starred', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
             WHERE id = ?1",
            [artist_id],
        )?;
    } else {
        conn.execute(
            "UPDATE artists SET starred = NULL, raw_json = json_remove(raw_json, '$.starred') WHERE id = ?1",
            [artist_id],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use rusqlite::params;

    #[test]
    fn starring_an_artist_updates_the_column_and_the_json() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "a", "[]");
        let starred = |conn: &Connection| -> (bool, String) {
            conn.query_row("SELECT starred IS NOT NULL, raw_json FROM artists", [], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
        };
        set_artist_starred(&conn, "a", true).unwrap();
        let (is_starred, raw) = starred(&conn);
        assert!(is_starred && raw.contains("starred"));
        set_artist_starred(&conn, "a", false).unwrap();
        let (is_starred, raw) = starred(&conn);
        assert!(!is_starred && !raw.contains("starred"));
    }

    fn artist(conn: &Connection, id: &str, roles: &str) {
        conn.execute(
            "INSERT INTO artists (id, name, cover_art, roles, raw_json) VALUES (?1, ?1, 'ar-' || ?1, ?2, '{}')",
            params![id, roles],
        )
        .unwrap();
    }

    fn song(conn: &Connection, id: &str, raw: &str, plays: i64, created: &str) {
        conn.execute(
            "INSERT INTO songs (id, title, cover_art, play_count, created, raw_json) VALUES (?1, ?1, 'c-' || ?1, ?2, ?3, ?4)",
            params![id, plays, created, raw],
        )
        .unwrap();
    }

    fn links(conn: &Connection) -> Vec<(String, String, String)> {
        conn.prepare("SELECT song_id, artist_id, role FROM song_artists ORDER BY 1, 2, 3")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn link(song: &str, artist: &str, role: &str) -> (String, String, String) {
        (song.into(), artist.into(), role.into())
    }

    #[test]
    fn song_artists_come_from_the_raw_json() {
        let conn = open_in_memory().unwrap();
        song(
            &conn,
            "feat",
            r#"{"artists":[{"id":"main","name":"Main"},{"id":"guest","name":"Guest"}],"albumArtists":[{"id":"main"}]}"#,
            0,
            "2026-01-01T00:00:00Z",
        );
        fill_song_artists(&conn).unwrap();
        assert_eq!(
            links(&conn),
            [link("feat", "guest", "artist"), link("feat", "main", "albumartist"), link("feat", "main", "artist")]
        );
    }

    #[test]
    fn without_opensubsonic_the_main_artists_stand_in() {
        let conn = open_in_memory().unwrap();
        conn.execute("INSERT INTO albums (id, name, artist_id, raw_json) VALUES ('al1', 'A', 'band', '{}')", [])
            .unwrap();
        conn.execute(
            "INSERT INTO songs (id, title, artist_id, album_id, raw_json) VALUES ('s1', 'S', 'singer', 'al1', '{}')",
            [],
        )
        .unwrap();
        fill_song_artists(&conn).unwrap();
        assert_eq!(links(&conn), [link("s1", "band", "albumartist"), link("s1", "singer", "artist")]);
    }

    #[test]
    fn filling_again_replaces_the_old_links() {
        let conn = open_in_memory().unwrap();
        song(&conn, "s1", r#"{"artists":[{"id":"a"}]}"#, 0, "2026-01-01T00:00:00Z");
        fill_song_artists(&conn).unwrap();
        conn.execute("UPDATE songs SET raw_json = '{\"artists\":[{\"id\":\"b\"}]}'", []).unwrap();
        fill_song_artists(&conn).unwrap();
        assert_eq!(links(&conn), [link("s1", "b", "artist")]);
    }

    #[test]
    fn the_list_has_album_and_featured_artists_but_not_credits() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "main", r#"["albumartist","artist"]"#);
        artist(&conn, "guest", r#"["artist"]"#);
        artist(&conn, "producer", r#"["producer"]"#);
        song(
            &conn,
            "s1",
            r#"{"artists":[{"id":"main"},{"id":"guest"}],"albumArtists":[{"id":"main"}]}"#,
            3,
            "2026-01-01T00:00:00Z",
        );
        song(&conn, "s2", r#"{"artists":[{"id":"main"}],"albumArtists":[{"id":"main"}]}"#, 4, "2026-05-01T10:00:00.5Z");
        fill_song_artists(&conn).unwrap();

        let artists = artist_list(&conn).unwrap();
        let ids: Vec<&str> = artists.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["guest", "main"], "the producer only has credits");

        let main = &artists[1];
        assert!(main.album_artist);
        // Artist and album artist of s1 and s2: each song counted once.
        assert_eq!(main.play_count, 7);
        assert_eq!(main.added.as_deref(), Some("2026-05-01T10:00:00Z"));

        let guest = &artists[0];
        assert!(!guest.album_artist);
        assert_eq!(guest.play_count, 3);
        assert_eq!(guest.fallback_cover_art.as_deref(), Some("c-s1"), "no album: the cover of their song");
    }

    #[test]
    fn the_fallback_is_the_latest_album_cover() {
        let conn = open_in_memory().unwrap();
        artist(&conn, "band", "[]");
        for (id, year) in [("old", 2001), ("new", 2019)] {
            conn.execute(
                "INSERT INTO albums (id, name, artist_id, cover_art, year, raw_json) VALUES (?1, ?1, 'band', 'al-' || ?1, ?2, '{}')",
                params![id, year],
            )
            .unwrap();
        }
        let artists = artist_list(&conn).unwrap();
        assert_eq!(artists.len(), 1, "main artist of an album: album artist");
        assert!(artists[0].album_artist);
        assert_eq!(artists[0].fallback_cover_art.as_deref(), Some("al-new"));
        assert_eq!(artists[0].album_count, 2, "no count from the server: counted here");
    }

    #[test]
    fn songs_of_artists_lists_each_song_once() {
        let conn = open_in_memory().unwrap();
        song(&conn, "s1", r#"{"artists":[{"id":"a"},{"id":"b"}]}"#, 0, "2026-01-01T00:00:00Z");
        song(&conn, "s2", r#"{"artists":[{"id":"b"}]}"#, 0, "2026-01-01T00:00:00Z");
        song(&conn, "s3", r#"{"artists":[{"id":"c"}]}"#, 0, "2026-01-01T00:00:00Z");
        fill_song_artists(&conn).unwrap();

        let mut songs = songs_of_artists(&conn, &["a".into(), "b".into()]).unwrap();
        songs.sort();
        assert_eq!(songs, ["s1", "s2"]);
    }
}
