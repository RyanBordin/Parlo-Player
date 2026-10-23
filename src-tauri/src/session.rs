//! Saved login.
//!
//! - Server URL and username: `config.json` in the app config directory.
//! - Password: the OS credential store (Windows Credential Manager, Secret
//!   Service on Linux). It is never written to disk by the app. Android has
//!   no store yet, so there the password is not kept and each launch asks again.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

const CONFIG_FILE: &str = "config.json";

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

    #[cfg(desktop)]
    #[error("credential store error: {0}")]
    CredentialStore(String),
}

/// Returns the saved session, or `None` if there is no config file or its
/// password is missing from the credential store.
pub fn load(config_dir: &Path) -> Result<Option<StoredLogin>, SessionError> {
    let Some(session) = read_config(config_dir)? else {
        return Ok(None);
    };

    Ok(credentials::get(&session)?.map(|password| StoredLogin { session, password }))
}

/// Stores the password in the credential store, then writes the config file.
/// If the config file cannot be written, the password is removed again.
pub fn save(config_dir: &Path, session: &SavedSession, password: &str) -> Result<(), SessionError> {
    credentials::set(session, password)?;

    if let Err(e) = write_config(config_dir, session) {
        let _ = credentials::delete(session);
        return Err(e);
    }
    Ok(())
}

/// Removes the password from the credential store and deletes the config file.
/// Succeeds if there was nothing to remove.
pub fn clear(config_dir: &Path) -> Result<(), SessionError> {
    // A corrupted config file is still deleted, even if we can't find its password.
    if let Ok(Some(session)) = read_config(config_dir) {
        credentials::delete(&session)?;
    }

    match fs::remove_file(config_dir.join(CONFIG_FILE)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// The OS credential store, through the `keyring` crate.
#[cfg(desktop)]
mod credentials {
    use super::{SavedSession, SessionError};

    const KEYRING_SERVICE: &str = "com.ryanbordin.parloplayer";

    impl From<keyring::Error> for SessionError {
        fn from(error: keyring::Error) -> Self {
            SessionError::CredentialStore(error.to_string())
        }
    }

    // One entry per user and server. On Windows it appears in Credential Manager
    // as "{username}@{server_url}.com.ryanbordin.parloplayer".
    fn entry(session: &SavedSession) -> Result<keyring::Entry, SessionError> {
        let account = format!("{}@{}", session.username, session.server_url);
        Ok(keyring::Entry::new(KEYRING_SERVICE, &account)?)
    }

    pub fn get(session: &SavedSession) -> Result<Option<String>, SessionError> {
        match entry(session)?.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(session: &SavedSession, password: &str) -> Result<(), SessionError> {
        Ok(entry(session)?.set_password(password)?)
    }

    /// Succeeds if there was nothing to remove.
    pub fn delete(session: &SavedSession) -> Result<(), SessionError> {
        match entry(session)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// No credential store on mobile yet (Android Keystore comes later): the
/// password is never kept, so `load` finds no login and the app asks again.
#[cfg(mobile)]
mod credentials {
    use super::{SavedSession, SessionError};

    pub fn get(_: &SavedSession) -> Result<Option<String>, SessionError> {
        Ok(None)
    }

    pub fn set(_: &SavedSession, _: &str) -> Result<(), SessionError> {
        Ok(())
    }

    pub fn delete(_: &SavedSession) -> Result<(), SessionError> {
        Ok(())
    }
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
    #[cfg(desktop)]
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
        assert_eq!(credentials::get(&session).unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
