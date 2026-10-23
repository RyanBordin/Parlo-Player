use thiserror::Error;

/// Everything that can go wrong when talking to a Subsonic server.
#[derive(Debug, Error)]
pub enum SubsonicError {
    /// Network failure, timeout, invalid URL or non-2xx HTTP status.
    #[error("connection error: {0}")]
    Http(reqwest::Error),

    /// The server URL cannot be turned into a request URL.
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    /// The server answered, but not with a valid `subsonic-response`
    /// (e.g. the URL points to some other website).
    #[error("invalid server response: {0}")]
    InvalidResponse(String),

    /// The server returned `status: "failed"` with an error code.
    /// Common codes: 10 missing parameter, 40 wrong username or password,
    /// 50 not authorized, 70 not found.
    #[error("error {code}: {message}")]
    Api { code: i32, message: String },
}

// reqwest puts the request URL in its error text, and GET URLs carry the
// token and salt: drop it before the error can reach the UI or a log.
impl From<reqwest::Error> for SubsonicError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error.without_url())
    }
}
