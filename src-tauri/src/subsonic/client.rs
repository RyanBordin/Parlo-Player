use std::fmt;
use std::time::Duration;

use serde::Serialize;

use super::auth::{generate_salt, make_token};
use super::error::SubsonicError;
use super::response::{ResponseBody, parse_response};

const API_VERSION: &str = "1.16.1";
const CLIENT_NAME: &str = "ParloPlayer";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Server details returned by `ping`, sent to the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub api_version: String,
    pub server_type: Option<String>,
    pub server_version: Option<String>,
    pub open_subsonic: bool,
}

pub struct SubsonicClient {
    base_url: String,
    username: String,
    password: String,
    http: reqwest::Client,
}

impl SubsonicClient {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, SubsonicError> {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self {
            base_url,
            username: username.into(),
            password: password.into(),
            http,
        })
    }

    /// `GET /rest/ping` — checks that the server is reachable and the
    /// credentials are valid.
    pub async fn ping(&self) -> Result<ServerInfo, SubsonicError> {
        let body = self.get("ping").await?;
        Ok(ServerInfo {
            api_version: body.version,
            server_type: body.server_type,
            server_version: body.server_version,
            open_subsonic: body.open_subsonic.unwrap_or(false),
        })
    }

    async fn get(&self, endpoint: &str) -> Result<ResponseBody, SubsonicError> {
        let url = format!("{}/rest/{endpoint}", self.base_url);
        let params = self.auth_params(&generate_salt());

        let text = self
            .http
            .get(url)
            .query(&params)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        parse_response(&text)
    }

    /// Query parameters sent with every request. A fresh salt must be passed
    /// each time.
    fn auth_params(&self, salt: &str) -> [(&'static str, String); 6] {
        [
            ("u", self.username.clone()),
            ("t", make_token(&self.password, salt)),
            ("s", salt.to_string()),
            ("v", API_VERSION.to_string()),
            ("c", CLIENT_NAME.to_string()),
            ("f", "json".to_string()),
        ]
    }
}

// Hand-written so the password never ends up in logs or error messages.
impl fmt::Debug for SubsonicClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SubsonicClient")
            .field("base_url", &self.base_url)
            .field("username", &self.username)
            .field("password", &"<hidden>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client() -> SubsonicClient {
        SubsonicClient::new("https://music.example.com/", "alice", "sesame").unwrap()
    }

    #[test]
    fn trailing_slash_is_removed_from_base_url() {
        assert_eq!(client().base_url, "https://music.example.com");
    }

    #[test]
    fn auth_params_use_given_salt() {
        let params = client().auth_params("c19b2d");
        let get = |key: &str| {
            params
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.as_str())
        };

        assert_eq!(get("u"), Some("alice"));
        assert_eq!(get("t"), Some("26719a1196d2a940705a59634eb18eab"));
        assert_eq!(get("s"), Some("c19b2d"));
        assert_eq!(get("v"), Some("1.16.1"));
        assert_eq!(get("c"), Some("ParloPlayer"));
        assert_eq!(get("f"), Some("json"));
        assert_eq!(get("p"), None, "plain password must never be sent");
    }

    #[test]
    fn debug_output_hides_password() {
        let debug = format!("{:?}", client());
        assert!(!debug.contains("sesame"));
        assert!(debug.contains("<hidden>"));
    }
}
