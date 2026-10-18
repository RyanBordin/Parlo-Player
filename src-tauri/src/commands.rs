//! Tauri commands exposed to the UI.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::session::{self, SavedSession, SessionError};
use crate::subsonic::{ServerInfo, SubsonicClient, SubsonicError};

/// Error sent to the UI. The UI picks the translated message from `kind`,
/// e.g. `{ "kind": "api", "code": 40, "message": "Wrong username or password" }`.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CommandError {
    InvalidUrl,
    Timeout,
    HttpStatus { status: u16 },
    Network { detail: String },
    InvalidResponse,
    Api { code: i32, message: String },
    Storage { detail: String },
}

impl From<SubsonicError> for CommandError {
    fn from(error: SubsonicError) -> Self {
        match error {
            SubsonicError::Http(e) if e.is_builder() => Self::InvalidUrl,
            SubsonicError::Http(e) if e.is_timeout() => Self::Timeout,
            SubsonicError::Http(e) if e.is_status() => match e.status() {
                Some(status) => Self::HttpStatus { status: status.as_u16() },
                None => Self::Network { detail: e.to_string() },
            },
            SubsonicError::Http(e) => Self::Network { detail: e.to_string() },
            SubsonicError::InvalidResponse(_) => Self::InvalidResponse,
            SubsonicError::Api { code, message } => Self::Api { code, message },
        }
    }
}

impl From<SessionError> for CommandError {
    fn from(error: SessionError) -> Self {
        Self::Storage { detail: error.to_string() }
    }
}

/// Pings the server with the given credentials. Nothing is stored.
#[tauri::command]
pub async fn test_connection(
    url: String,
    user: String,
    password: String,
) -> Result<ServerInfo, CommandError> {
    let client = SubsonicClient::new(url, user, password)?;
    Ok(client.ping().await?)
}

/// Returns the saved login, if any. The password never leaves the backend.
#[tauri::command]
pub async fn get_session(app: AppHandle) -> Result<Option<SavedSession>, CommandError> {
    let dir = config_dir(&app)?;
    run_blocking(move || session::load(&dir)).await
}

/// Checks the credentials again, then saves them.
#[tauri::command]
pub async fn login(
    app: AppHandle,
    url: String,
    user: String,
    password: String,
) -> Result<SavedSession, CommandError> {
    let client = SubsonicClient::new(url, user.clone(), password.clone())?;
    client.ping().await?;

    let saved = SavedSession {
        server_url: client.base_url().to_string(),
        username: user,
    };
    let dir = config_dir(&app)?;
    let to_save = saved.clone();
    run_blocking(move || session::save(&dir, &to_save, &password)).await?;
    Ok(saved)
}

/// Deletes the saved password and config file.
#[tauri::command]
pub async fn logout(app: AppHandle) -> Result<(), CommandError> {
    let dir = config_dir(&app)?;
    run_blocking(move || session::clear(&dir)).await
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path().app_config_dir().map_err(|e| CommandError::Storage {
        detail: e.to_string(),
    })
}

// Credential store calls block the thread, so they run outside the async runtime.
async fn run_blocking<T, F>(task: F) -> Result<T, CommandError>
where
    F: FnOnce() -> Result<T, SessionError> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| CommandError::Storage { detail: e.to_string() })?
        .map_err(CommandError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn api_error_serializes_with_kind_tag() {
        let error = CommandError::from(SubsonicError::Api {
            code: 40,
            message: "Wrong username or password".into(),
        });

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({ "kind": "api", "code": 40, "message": "Wrong username or password" })
        );
    }

    #[test]
    fn unit_variants_serialize_in_camel_case() {
        assert_eq!(
            serde_json::to_value(CommandError::InvalidUrl).unwrap(),
            json!({ "kind": "invalidUrl" })
        );
        assert_eq!(
            serde_json::to_value(CommandError::HttpStatus { status: 404 }).unwrap(),
            json!({ "kind": "httpStatus", "status": 404 })
        );
    }

    #[test]
    fn invalid_response_maps_to_its_own_kind() {
        let error = CommandError::from(SubsonicError::InvalidResponse("html".into()));
        assert_eq!(error, CommandError::InvalidResponse);
    }
}
