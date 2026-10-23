//! Preferences tied to a server + user (spec 6.10), in its library database:
//! each one a row of `settings` with a JSON value. A missing row, or a value
//! this version does not understand, gives the preference's default.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::DbResult;
use crate::settings::lenient;

/// Automatic sync intervals offered, in minutes; 0 is "manual only".
pub const SYNC_INTERVALS: [u32; 4] = [15, 30, 60, 0];
const DEFAULT_SYNC_INTERVAL: u32 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibrarySettings {
    /// Send plays to the server (spec 5.7). Off, the local history is still kept.
    #[serde(deserialize_with = "lenient")]
    pub scrobble: bool,
    /// Minutes between automatic syncs, one of `SYNC_INTERVALS` (0: manual).
    #[serde(deserialize_with = "lenient")]
    pub sync_interval_minutes: u32,
    /// Resume a queue another app saved on the server later (spec 5.6).
    #[serde(deserialize_with = "lenient")]
    pub continue_other_device: bool,
    /// Music folders (`getMusicFolders` IDs) left out of the library.
    #[serde(deserialize_with = "lenient")]
    pub hidden_folders: Vec<String>,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        Self {
            scrobble: true,
            sync_interval_minutes: DEFAULT_SYNC_INTERVAL,
            continue_other_device: true,
            hidden_folders: Vec::new(),
        }
    }
}

impl LibrarySettings {
    /// The same settings with values out of range set to their default.
    pub fn normalized(mut self) -> Self {
        if !SYNC_INTERVALS.contains(&self.sync_interval_minutes) {
            self.sync_interval_minutes = DEFAULT_SYNC_INTERVAL;
        }
        self.hidden_folders.sort();
        self.hidden_folders.dedup();
        self
    }
}

pub fn load(conn: &Connection) -> DbResult<LibrarySettings> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let mut values = Map::new();
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (key, text) = row?;
        if let Ok(value) = serde_json::from_str(&text) {
            values.insert(key, value);
        }
    }
    let settings: LibrarySettings = serde_json::from_value(Value::Object(values)).unwrap_or_default();
    Ok(settings.normalized())
}

/// Writes every preference (one row each).
pub fn save(conn: &mut Connection, settings: &LibrarySettings) -> DbResult<()> {
    let Ok(Value::Object(values)) = serde_json::to_value(settings.clone().normalized()) else {
        return Ok(());
    };
    let tx = conn.transaction()?;
    for (key, value) in values {
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value.to_string()],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    #[test]
    fn no_rows_give_the_defaults() {
        let conn = open_in_memory().unwrap();
        let settings = load(&conn).unwrap();
        assert_eq!(settings, LibrarySettings::default());
        assert!(settings.scrobble && settings.continue_other_device);
        assert_eq!(settings.sync_interval_minutes, 30);
        assert!(settings.hidden_folders.is_empty());
    }

    #[test]
    fn saved_settings_come_back() {
        let mut conn = open_in_memory().unwrap();
        let settings = LibrarySettings {
            scrobble: false,
            sync_interval_minutes: 0,
            continue_other_device: false,
            hidden_folders: vec!["2".into()],
        };
        save(&mut conn, &settings).unwrap();
        assert_eq!(load(&conn).unwrap(), settings);
    }

    #[test]
    fn unknown_or_broken_rows_are_ignored() {
        let conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('future', '42'), ('scrobble', 'not json'), ('syncIntervalMinutes', '7')",
            [],
        )
        .unwrap();
        assert_eq!(load(&conn).unwrap(), LibrarySettings::default());
    }
}
