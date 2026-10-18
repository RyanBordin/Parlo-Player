//! URLs that carry credentials (token + salt), such as `stream` URLs.

use std::fmt;

/// A URL with credentials in its query string. `Debug` and `Display` hide the
/// query, so it never shows up in logs or error messages. Only `expose()` gives
/// the full text, for handing it to the player.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretUrl(String);

impl SecretUrl {
    pub(super) fn new(url: reqwest::Url) -> Self {
        Self(url.into())
    }

    /// The full URL, credentials included. Never log the result.
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Wraps any media address, for player tests that play synthetic media
    /// (e.g. `av://lavfi:...`, which is not a standard URL).
    #[cfg(test)]
    pub fn for_test(url: &str) -> Self {
        Self(url.to_string())
    }
}

impl fmt::Display for SecretUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Anything that cannot be parsed is hidden entirely.
        let Ok(url) = reqwest::Url::parse(&self.0) else {
            return write!(f, "<redacted>");
        };
        write!(f, "{}://{}", url.scheme(), url.host_str().unwrap_or(""))?;
        if let Some(port) = url.port() {
            write!(f, ":{port}")?;
        }
        write!(f, "{}?<redacted>", url.path())
    }
}

impl fmt::Debug for SecretUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretUrl({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_debug_hide_the_query() {
        let url = SecretUrl::new(
            reqwest::Url::parse("http://192.168.0.10:4533/rest/stream?id=s1&u=alice&t=abc123&s=salt")
                .unwrap(),
        );
        for text in [url.to_string(), format!("{url:?}")] {
            assert!(text.contains("192.168.0.10:4533/rest/stream"), "{text}");
            assert!(!text.contains("abc123") && !text.contains("salt") && !text.contains("alice"));
        }
        assert!(url.expose().contains("t=abc123"));
    }

    #[test]
    fn unparsable_address_is_fully_hidden() {
        let url = SecretUrl::for_test("av://lavfi:sine=frequency=440");
        assert_eq!(url.to_string(), "<redacted>");
    }
}
