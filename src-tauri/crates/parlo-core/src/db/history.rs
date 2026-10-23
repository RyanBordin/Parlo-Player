//! Play history (spec 11.4) and scrobbles waiting to be sent (spec 5.7).
//!
//! Every counted play is a row in `play_history`. `scrobbled` says what
//! happened to it on the server: 0 pending (not confirmed yet), 1 sent, 2 not
//! to be sent (scrobble was off for this server, spec 6.10: the local history
//! is kept anyway).

use rusqlite::{Connection, params};

use super::DbResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingScrobble {
    pub history_id: i64,
    pub song_id: String,
    /// Unix seconds, when the play started.
    pub played_at: i64,
}

const PENDING: i64 = 0;
const NOT_SENT: i64 = 2;

/// Records a counted play: adds it to the history (pending scrobble) and bumps
/// the local play counts of the song and its album, like the server does.
/// Returns the history row ID.
pub fn record_play(conn: &mut Connection, song_id: &str, played_at: i64) -> DbResult<i64> {
    record(conn, song_id, played_at, PENDING)
}

/// Same as `record_play`, for a play that is never sent (scrobble off).
pub fn record_local_play(conn: &mut Connection, song_id: &str, played_at: i64) -> DbResult<i64> {
    record(conn, song_id, played_at, NOT_SENT)
}

fn record(conn: &mut Connection, song_id: &str, played_at: i64, scrobbled: i64) -> DbResult<i64> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO play_history (song_id, played_at, scrobbled) VALUES (?1, ?2, ?3)",
        params![song_id, played_at, scrobbled],
    )?;
    let history_id = tx.last_insert_rowid();

    let played = "strftime('%Y-%m-%dT%H:%M:%SZ', ?2, 'unixepoch')";
    tx.execute(
        &format!(
            "UPDATE songs SET play_count = COALESCE(play_count, 0) + 1, played = {played}
             WHERE id = ?1"
        ),
        params![song_id, played_at],
    )?;
    tx.execute(
        &format!(
            "UPDATE albums SET play_count = COALESCE(play_count, 0) + 1, played = {played}
             WHERE id = (SELECT album_id FROM songs WHERE id = ?1)"
        ),
        params![song_id, played_at],
    )?;
    tx.commit()?;
    Ok(history_id)
}

/// Oldest pending scrobbles first.
pub fn pending_scrobbles(conn: &Connection, limit: u32) -> DbResult<Vec<PendingScrobble>> {
    let mut stmt = conn.prepare(
        "SELECT id, song_id, played_at FROM play_history
         WHERE scrobbled = 0 ORDER BY played_at, id LIMIT ?1",
    )?;
    let rows = stmt
        .query_map([limit], |row| {
            Ok(PendingScrobble { history_id: row.get(0)?, song_id: row.get(1)?, played_at: row.get(2)? })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// How many plays wait to be sent (spec 6.10, Sync).
pub fn pending_count(conn: &Connection) -> DbResult<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM play_history WHERE scrobbled = 0", [], |row| row.get(0))?)
}

/// Marks history rows as confirmed by the server.
pub fn mark_scrobbled(conn: &Connection, history_ids: &[i64]) -> DbResult<()> {
    let ids = serde_json::to_string(history_ids).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "UPDATE play_history SET scrobbled = 1 WHERE id IN (SELECT value FROM json_each(?1))",
        [ids],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn insert_song(conn: &Connection, id: &str, album_id: &str, play_count: Option<i64>) {
        conn.execute(
            "INSERT INTO songs (id, title, album_id, play_count, raw_json) VALUES (?1, 'T', ?2, ?3, '{}')",
            params![id, album_id, play_count],
        )
        .unwrap();
    }

    #[test]
    fn plays_kept_local_are_never_pending() {
        let mut conn = open_in_memory().unwrap();
        insert_song(&conn, "s1", "al1", Some(2));
        record_local_play(&mut conn, "s1", 1000).unwrap();
        assert!(pending_scrobbles(&conn, 10).unwrap().is_empty());
        assert_eq!(pending_count(&conn).unwrap(), 0);
        let plays: i64 = conn.query_row("SELECT play_count FROM songs", [], |r| r.get(0)).unwrap();
        assert_eq!(plays, 3, "the local count still goes up");

        record_play(&mut conn, "s1", 2000).unwrap();
        assert_eq!(pending_count(&conn).unwrap(), 1, "only the play to send");
    }

    #[test]
    fn record_play_adds_history_and_bumps_play_counts() {
        let mut conn = open_in_memory().unwrap();
        conn.execute("INSERT INTO albums (id, name, play_count, raw_json) VALUES ('al1', 'A', 4, '{}')", [])
            .unwrap();
        insert_song(&conn, "s1", "al1", Some(2));

        // 2026-10-18 12:00:00 UTC
        let id = record_play(&mut conn, "s1", 1_792_324_800).unwrap();
        assert!(id > 0);

        let (count, played): (i64, String) = conn
            .query_row("SELECT play_count, played FROM songs WHERE id = 's1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(count, 3);
        assert_eq!(played, "2026-10-18T12:00:00Z");
        let album_count: i64 =
            conn.query_row("SELECT play_count FROM albums WHERE id = 'al1'", [], |r| r.get(0)).unwrap();
        assert_eq!(album_count, 5);
    }

    #[test]
    fn first_play_of_a_song_without_count_starts_at_one() {
        let mut conn = open_in_memory().unwrap();
        insert_song(&conn, "s1", "al1", None);
        record_play(&mut conn, "s1", 1000).unwrap();
        let count: i64 =
            conn.query_row("SELECT play_count FROM songs WHERE id = 's1'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn plays_stay_pending_until_marked() {
        let mut conn = open_in_memory().unwrap();
        insert_song(&conn, "s1", "al1", None);
        let first = record_play(&mut conn, "s1", 2000).unwrap();
        let second = record_play(&mut conn, "s1", 1000).unwrap();

        let pending = pending_scrobbles(&conn, 10).unwrap();
        assert_eq!(
            pending.iter().map(|p| p.history_id).collect::<Vec<_>>(),
            [second, first],
            "oldest first"
        );

        mark_scrobbled(&conn, &[second]).unwrap();
        let pending = pending_scrobbles(&conn, 10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].history_id, first);
    }

    #[test]
    fn a_song_missing_from_the_cache_is_still_recorded() {
        let mut conn = open_in_memory().unwrap();
        record_play(&mut conn, "unknown", 1000).unwrap();
        assert_eq!(pending_scrobbles(&conn, 10).unwrap().len(), 1);
    }
}
