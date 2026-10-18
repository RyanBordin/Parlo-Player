use thiserror::Error;

/// Everything that can go wrong when talking to a Subsonic server.
#[derive(Debug, Error)]
pub enum SubsonicError {
    /// Network failure, timeout, invalid URL or non-2xx HTTP status.
    #[error("connection error: {0}")]
    Http(#[from] reqwest::Error),

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
