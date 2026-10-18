//! Saved login.
//!
//! - Server URL and username: `config.json` in the app config directory.
//! - Password: the OS credential store (Windows Credential Manager, Secret
//!   Service on Linux). It is never written to disk by the app.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const CONFIG_FILE: &str = "config.json";
const KEYRING_SERVICE: &str = "com.ryanbordin.parloplayer";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSession {
    pub server_url: String,
    pub username: String,
}

/// A saved login with its password. Stays in the backend, never sent to the UI.
#[derive(Clone, PartialEq, Eq)]
pub struct StoredLogin {
    pub session: SavedSession,
    pub password: String,
}

// Hand-written so the password never ends up in logs.
impl fmt::Debug for StoredLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoredLogin")
            .field("session", &self.session)
            .field("password", &"<hidden>")
            .finish()
    }
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("config file error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid config file: {0}")]
    InvalidConfig(#[from] serde_json::Error),

    #[error("credential store error: {0}")]
    CredentialStore(#[from] keyring::Error),
}

/// Returns the saved session, or `None` if there is no config file or its
/// password is missing from the credential store.
pub fn load(config_dir: &Path) -> Result<Option<StoredLogin>, SessionError> {
    let Some(session) = read_config(config_dir)? else {
        return Ok(None);
    };

    match keyring_entry(&session)?.get_password() {
        Ok(password) => Ok(Some(StoredLogin { session, password })),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Stores the password in the credential store, then writes the config file.
/// If the config file cannot be written, the password is removed again.
pub fn save(config_dir: &Path, session: &SavedSession, password: &str) -> Result<(), SessionError> {
    let entry = keyring_entry(session)?;
    entry.set_password(password)?;

    if let Err(e) = write_config(config_dir, session) {
        let _ = entry.delete_credential();
        return Err(e);
    }
    Ok(())
}

/// Removes the password from the credential store and deletes the config file.
/// Succeeds if there was nothing to remove.
pub fn clear(config_dir: &Path) -> Result<(), SessionError> {
    // A corrupted config file is still deleted, even if we can't find its password.
    if let Ok(Some(session)) = read_config(config_dir) {
        match keyring_entry(&session)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.into()),
        }
    }

    match fs::remove_file(config_dir.join(CONFIG_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// One entry per user and server. On Windows it appears in Credential Manager
// as "{username}@{server_url}.com.ryanbordin.parloplayer".
fn keyring_entry(session: &SavedSession) -> Result<keyring::Entry, SessionError> {
    let account = format!("{}@{}", session.username, session.server_url);
    Ok(keyring::Entry::new(KEYRING_SERVICE, &account)?)
}

fn read_config(config_dir: &Path) -> Result<Option<SavedSession>, SessionError> {
    match fs::read_to_string(config_dir.join(CONFIG_FILE)) {
        Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn write_config(config_dir: &Path, session: &SavedSession) -> Result<(), SessionError> {
    fs::create_dir_all(config_dir)?;
    let json = serde_json::to_string_pretty(session)?;
    fs::write(config_dir.join(CONFIG_FILE), json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Creates an empty temporary directory, unique per test.
    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("parlo-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn sample_session() -> SavedSession {
        SavedSession {
            server_url: "https://music.example.com".into(),
            username: "alice".into(),
        }
    }

    #[test]
    fn config_round_trip() {
        let dir = temp_dir("round-trip");
        write_config(&dir, &sample_session()).unwrap();

        assert_eq!(read_config(&dir).unwrap(), Some(sample_session()));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn config_file_never_contains_password_field() {
        let dir = temp_dir("no-password");
        write_config(&dir, &sample_session()).unwrap();

        let json = fs::read_to_string(dir.join(CONFIG_FILE)).unwrap();
        assert!(json.contains("\"serverUrl\""));
        assert!(!json.to_lowercase().contains("password"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_config_means_no_session() {
        let dir = temp_dir("missing");
        assert_eq!(read_config(&dir).unwrap(), None);
        assert_eq!(load(&dir).unwrap(), None);
    }

    #[test]
    fn corrupted_config_is_an_error() {
        let dir = temp_dir("corrupted");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(CONFIG_FILE), "not json").unwrap();

        assert!(matches!(read_config(&dir), Err(SessionError::InvalidConfig(_))));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_without_saved_session_succeeds() {
        let dir = temp_dir("clear-empty");
        clear(&dir).unwrap();
    }

    #[test]
    #[ignore = "writes to the real OS credential store"]
    fn save_load_clear_with_real_credential_store() {
        let dir = temp_dir("keyring");
        let session = SavedSession {
            server_url: "https://keyring-test.invalid".into(),
            username: "parlo-test-user".into(),
        };

        save(&dir, &session, "not-a-real-password").unwrap();
        let stored = load(&dir).unwrap().expect("saved login");
        assert_eq!(stored.session, session);
        assert_eq!(stored.password, "not-a-real-password");
        assert!(!format!("{stored:?}").contains("not-a-real-password"));

        clear(&dir).unwrap();
        assert_eq!(load(&dir).unwrap(), None);
        assert!(matches!(
            keyring_entry(&session).unwrap().get_password(),
            Err(keyring::Error::NoEntry)
        ));
        let _ = fs::remove_dir_all(&dir);
    }
}
