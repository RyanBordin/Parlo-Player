//! Tauri commands exposed to the UI.

use crate::subsonic::{ServerInfo, SubsonicClient, SubsonicError};

/// Pings the server with the given credentials. Nothing is stored.
#[tauri::command]
pub async fn test_connection(
    url: String,
    user: String,
    password: String,
) -> Result<ServerInfo, String> {
    let client = SubsonicClient::new(url, user, password).map_err(describe_error)?;
    client.ping().await.map_err(describe_error)
}

// User-facing messages (Portuguese until i18n is in place).
fn describe_error(error: SubsonicError) -> String {
    match error {
        SubsonicError::Http(e) if e.is_builder() => "URL inválida.".to_string(),
        SubsonicError::Http(e) if e.is_timeout() => {
            "O servidor demorou demais para responder.".to_string()
        }
        SubsonicError::Http(e) if e.is_status() => match e.status() {
            Some(status) => format!("O servidor respondeu com erro HTTP {status}."),
            None => format!("Erro de conexão: {e}"),
        },
        SubsonicError::Http(e) => format!("Não foi possível conectar ao servidor: {e}"),
        SubsonicError::InvalidResponse(_) => {
            "A URL respondeu, mas não parece ser um servidor Subsonic.".to_string()
        }
        SubsonicError::Api { code, message } => format!("Erro {code}: {message}"),
    }
}
