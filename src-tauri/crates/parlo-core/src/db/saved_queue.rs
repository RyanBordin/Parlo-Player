//! The last saved play queue (spec 5.6), one row per library database.

use rusqlite::{Connection, OptionalExtension, params};

use super::DbResult;
use crate::player::queue::{QueueSnapshot, RepeatMode};
use crate::player::resume::SavedQueue;

/// Replaces the saved queue.
pub fn save(conn: &Connection, saved: &SavedQueue) -> DbResult<()> {
    let queue = &saved.queue;
    let songs = serde_json::to_string(&queue.songs).unwrap_or_else(|_| "[]".into());
    let order = serde_json::to_string(&queue.order).unwrap_or_else(|_| "[]".into());
    conn.execute(
        "INSERT INTO saved_queue
             (id, songs, play_order, current, position, shuffle, repeat, label,
              continuing_from, volume, saved_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT (id) DO UPDATE SET
             songs = excluded.songs, play_order = excluded.play_order,
             current = excluded.current, position = excluded.position,
             shuffle = excluded.shuffle, repeat = excluded.repeat, label = excluded.label,
             continuing_from = excluded.continuing_from, volume = excluded.volume,
             saved_at = excluded.saved_at",
        params![
            songs,
            order,
            queue.current as i64,
            saved.position,
            queue.shuffle,
            queue.repeat.as_str(),
            queue.label,
            queue.continuing_from,
            saved.volume,
            saved.saved_at_ms,
        ],
    )?;
    Ok(())
}

/// The saved queue, or `None` if there is none. Unreadable song lists (a
/// damaged row) read as no queue.
pub fn load(conn: &Connection) -> DbResult<Option<SavedQueue>> {
    let row = conn
        .query_row(
            "SELECT songs, play_order, current, position, shuffle, repeat, label,
                    continuing_from, volume, saved_at
             FROM saved_queue WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    SavedQueue {
                        queue: QueueSnapshot {
                            songs: Vec::new(),
                            order: Vec::new(),
                            current: row.get::<_, i64>(2)?.max(0) as usize,
                            shuffle: row.get(4)?,
                            repeat: RepeatMode::from_name(&row.get::<_, String>(5)?),
                            label: row.get(6)?,
                            continuing_from: row.get(7)?,
                        },
                        position: row.get(3)?,
                        volume: row.get(8)?,
                        saved_at_ms: row.get(9)?,
                    },
                ))
            },
        )
        .optional()?;

    let Some((songs, order, mut saved)) = row else {
        return Ok(None);
    };
    let (Ok(songs), Ok(order)) = (serde_json::from_str(&songs), serde_json::from_str(&order)) else {
        return Ok(None);
    };
    saved.queue.songs = songs;
    saved.queue.order = order;
    Ok((!saved.queue.songs.is_empty()).then_some(saved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn saved(songs: &[&str], current: usize, saved_at_ms: i64) -> SavedQueue {
        let n = songs.len();
        SavedQueue {
            queue: QueueSnapshot {
                songs: songs.iter().map(|s| s.to_string()).collect(),
                order: (0..n).rev().collect(),
                current,
                shuffle: true,
                repeat: RepeatMode::One,
                label: "Recently added".into(),
                continuing_from: Some("Symfonium".into()),
            },
            position: 61.25,
            volume: 35.0,
            saved_at_ms,
        }
    }

    #[test]
    fn nothing_saved_yet() {
        let conn = open_in_memory().unwrap();
        assert_eq!(load(&conn).unwrap(), None);
    }

    #[test]
    fn saves_and_loads_every_field() {
        let conn = open_in_memory().unwrap();
        let queue = saved(&["s1", "s2", "s3"], 1, 1_000);
        save(&conn, &queue).unwrap();
        assert_eq!(load(&conn).unwrap(), Some(queue));
    }

    #[test]
    fn a_new_save_replaces_the_old_one() {
        let conn = open_in_memory().unwrap();
        save(&conn, &saved(&["s1", "s2", "s3"], 1, 1_000)).unwrap();
        let newer = saved(&["a", "b"], 0, 2_000);
        save(&conn, &newer).unwrap();

        assert_eq!(load(&conn).unwrap(), Some(newer));
        let rows: i64 = conn.query_row("SELECT COUNT(*) FROM saved_queue", [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_damaged_row_reads_as_no_queue() {
        let conn = open_in_memory().unwrap();
        save(&conn, &saved(&["s1"], 0, 1_000)).unwrap();
        conn.execute("UPDATE saved_queue SET songs = 'not json'", []).unwrap();
        assert_eq!(load(&conn).unwrap(), None);
    }
}
