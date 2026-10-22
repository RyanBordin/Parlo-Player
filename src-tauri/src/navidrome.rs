//! "Open in Navidrome" (spec 8.2): the item's page in the server's web
//! interface. Navidrome's interface (react-admin) routes after the "#":
//! albums and artists have a page (`show`); songs have none, so a song opens
//! its album. The address never carries credentials.

/// Pages of the web interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Album,
    Artist,
}

/// Whether the `type` of `ping` (OpenSubsonic) is Navidrome.
pub fn is_navidrome(server_type: Option<&str>) -> bool {
    server_type.is_some_and(|kind| kind.eq_ignore_ascii_case("navidrome"))
}

/// e.g. `https://music.example.com/app/#/album/al-1/show`. `base_url` is the
/// server address without a trailing slash (a sub path such as
/// `/navidrome` included).
pub fn page_url(base_url: &str, page: Page, id: &str) -> String {
    let resource = match page {
        Page::Album => "album",
        Page::Artist => "artist",
    };
    format!("{base_url}/app/#/{resource}/{}/show", encode(id))
}

/// IDs are opaque strings: anything but letters, digits and `-_.~` is
/// percent-encoded.
fn encode(id: &str) -> String {
    let mut text = String::with_capacity(id.len());
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            text.push(byte as char);
        } else {
            text.push_str(&format!("%{byte:02X}"));
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_follow_navidromes_routes() {
        assert_eq!(page_url("https://m.example.com", Page::Album, "al-1"), "https://m.example.com/app/#/album/al-1/show");
        assert_eq!(
            page_url("http://10.0.0.2:4533/navidrome", Page::Artist, "a b/c"),
            "http://10.0.0.2:4533/navidrome/app/#/artist/a%20b%2Fc/show"
        );
    }

    #[test]
    fn only_navidrome_counts() {
        assert!(is_navidrome(Some("navidrome")));
        assert!(is_navidrome(Some("Navidrome")));
        assert!(!is_navidrome(Some("gonic")));
        assert!(!is_navidrome(None));
    }
}
