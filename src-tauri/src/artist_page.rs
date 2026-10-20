//! Artist page (spec 6.7): what the server says about the artist
//! (`getArtistInfo2`, kept 7 days) and the `getTopSongs` order used while
//! the artist has no plays on this server.

use std::collections::HashSet;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::DbResult;
use crate::db::artist_page::{self, CachedInfo};
use crate::db::home::ArtistCard;
use crate::subsonic::ArtistInfo;

/// `getArtistInfo2` is asked again after this long.
pub const INFO_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;
/// Songs asked from `getTopSongs`.
pub const TOP_SONGS_COUNT: u32 = 50;

/// The parts of the page that come from the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistAbout {
    /// Plain text; `None` when there is none.
    pub biography: Option<String>,
    /// Similar artists that are in the library.
    pub similar: Vec<ArtistCard>,
    /// Whether the artist's photo is real, not the server's generic image;
    /// `None` when it could not be told.
    pub has_photo: Option<bool>,
    /// Every song of the artist in the `getTopSongs` order (the others after,
    /// in the page's order). `None`: the page's order stands.
    pub song_order: Option<Vec<String>>,
}

/// Whether an answer kept at `fetched_at` is still used at `now`.
pub fn is_fresh(fetched_at: i64, now: i64) -> bool {
    (0..INFO_MAX_AGE_SECS).contains(&(now - fetched_at))
}

/// What is kept of a `getArtistInfo2` answer.
pub fn to_cache(info: &ArtistInfo, has_photo: Option<bool>, now: i64) -> CachedInfo {
    CachedInfo {
        biography: info.biography.as_deref().and_then(plain_text),
        similar_ids: info.similar_artist.iter().filter(|a| !a.id.is_empty()).map(|a| a.id.clone()).collect(),
        has_photo,
        fetched_at: now,
    }
}

/// The page's server parts from what was kept, and the IDs `getTopSongs`
/// gave (only asked while the artist has no plays).
pub fn about(conn: &Connection, artist_id: &str, info: Option<&CachedInfo>, top_songs: &[String]) -> DbResult<ArtistAbout> {
    let similar = match info {
        Some(info) => artist_page::artist_cards(conn, &info.similar_ids, artist_id)?,
        None => Vec::new(),
    };
    let song_order = if top_songs.is_empty() {
        None
    } else {
        order_by_top_songs(artist_page::song_ids(conn, artist_id)?, top_songs)
    };
    Ok(ArtistAbout {
        biography: info.and_then(|i| i.biography.clone()),
        similar,
        has_photo: info.and_then(|i| i.has_photo),
        song_order,
    })
}

/// The songs of `top` that are in `songs`, in `top`'s order, then the rest
/// of `songs`. `None` when none of them is.
fn order_by_top_songs(songs: Vec<String>, top: &[String]) -> Option<Vec<String>> {
    let known: HashSet<&str> = songs.iter().map(String::as_str).collect();
    let mut first: HashSet<&str> = HashSet::new();
    let mut order: Vec<String> = top
        .iter()
        .filter(|id| known.contains(id.as_str()) && first.insert(id.as_str()))
        .cloned()
        .collect();
    if order.is_empty() {
        return None;
    }
    let rest: Vec<String> = songs.iter().filter(|id| !first.contains(id.as_str())).cloned().collect();
    order.extend(rest);
    Some(order)
}

/// The biography as plain text (spec 6.7): tags removed, scripts and styles
/// dropped with their content, the "Read more on Last.fm" link dropped
/// (other links keep their text), entities decoded and spaces tidied, one
/// line per paragraph. `None` if no text is left.
pub fn plain_text(html: &str) -> Option<String> {
    let mut text = Stripper::default();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        text.push(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('>') else {
            // An unclosed tag: what follows is not text.
            rest = "";
            break;
        };
        text.tag(&after[..end]);
        rest = &after[end + 1..];
    }
    text.push(rest);
    text.close_link();

    let decoded = decode_entities(&text.out);
    let lines: Vec<String> = decoded
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect();
    let joined = lines.join("\n");
    (!joined.is_empty()).then_some(joined)
}

#[derive(Default)]
struct Stripper {
    out: String,
    /// Name of the tag whose content is being dropped (script, style).
    dropping: Option<String>,
    /// Text of the link being read: kept or dropped when it closes.
    link: Option<String>,
}

impl Stripper {
    fn push(&mut self, text: &str) {
        if self.dropping.is_some() {
            return;
        }
        match &mut self.link {
            Some(link) => link.push_str(text),
            None => self.out.push_str(text),
        }
    }

    /// `raw` is what is between `<` and `>`.
    fn tag(&mut self, raw: &str) {
        let raw = raw.trim();
        let closing = raw.starts_with('/');
        let name = raw
            .trim_start_matches('/')
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase();

        if let Some(dropping) = &self.dropping {
            if closing && name == *dropping {
                self.dropping = None;
            }
            return;
        }
        match (name.as_str(), closing) {
            ("script" | "style", false) => self.dropping = Some(name),
            ("a", false) => {
                self.link.get_or_insert_with(String::new);
            }
            ("a", true) => self.close_link(),
            ("br" | "p" | "div" | "li", _) => self.push("\n"),
            _ => {}
        }
    }

    fn close_link(&mut self) {
        if let Some(link) = self.link.take()
            && !link.to_lowercase().contains("last.fm")
        {
            self.out.push_str(&link);
        }
    }
}

/// `&amp;`, `&#39;`, `&#x2014;`... as characters. Unknown ones stay as written.
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let decoded = after
            .find(';')
            .filter(|&end| end <= 10)
            .and_then(|end| entity(&after[..end]).map(|c| (c, end)));
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            char::from_u32(code).map(|c| if c.is_control() { ' ' } else { c })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subsonic::SimilarArtist;

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_last_fm_link_is_dropped_and_tags_removed() {
        let html = "Radiohead are an <b>English</b> rock band &amp; more.\n\n \
                    <a href=\"https://www.last.fm/music/Radiohead\">Read more on Last.fm</a>";
        assert_eq!(plain_text(html).as_deref(), Some("Radiohead are an English rock band & more."));
    }

    #[test]
    fn other_links_keep_their_text_but_not_the_address() {
        let html = r#"Formed with <a href="https://example.com/x" onclick="steal()">Thom Yorke</a>."#;
        assert_eq!(plain_text(html).as_deref(), Some("Formed with Thom Yorke."));
    }

    #[test]
    fn scripts_and_styles_are_dropped_with_their_content() {
        let html = "Safe<script>alert('x')</script> text<style>p { color: red }</style>.";
        assert_eq!(plain_text(html).as_deref(), Some("Safe text."));
    }

    #[test]
    fn escaped_html_stays_as_text() {
        // Decoded after the tags are removed: it is shown, never run.
        assert_eq!(plain_text("1 &lt;script&gt; 2").as_deref(), Some("1 <script> 2"));
    }

    #[test]
    fn paragraphs_are_lines_and_spaces_are_tidied() {
        let html = "<p>First   para.</p><p>Second&nbsp;one&#39;s<br/>end&#x21;</p>";
        assert_eq!(plain_text(html).as_deref(), Some("First para.\nSecond one's\nend!"));
    }

    #[test]
    fn broken_html_and_unknown_entities_do_not_fail() {
        assert_eq!(plain_text("Text &unknown; & co <a href=").as_deref(), Some("Text &unknown; & co"));
        assert_eq!(plain_text("<a href='x'>Read more on Last.fm"), None, "an unclosed link");
        assert_eq!(plain_text("   "), None);
        assert_eq!(plain_text("&#0;"), None, "control characters are dropped");
    }

    #[test]
    fn info_is_fresh_for_seven_days() {
        let day = 24 * 60 * 60;
        assert!(is_fresh(1_000, 1_000));
        assert!(is_fresh(1_000, 1_000 + 7 * day - 1));
        assert!(!is_fresh(1_000, 1_000 + 7 * day));
        assert!(!is_fresh(1_000 + day, 1_000), "kept in the future: the clock moved back");
    }

    #[test]
    fn the_cache_keeps_plain_text_and_known_similar_artists() {
        let info = ArtistInfo {
            biography: Some("<b>Bio</b> <a href='https://last.fm'>Read more on Last.fm</a>".into()),
            similar_artist: vec![
                SimilarArtist { id: "ar2".into(), name: "B".into() },
                SimilarArtist { id: String::new(), name: "Not here".into() },
            ],
            raw_json: String::new(),
        };
        let cached = to_cache(&info, Some(true), 42);
        assert_eq!(cached.biography.as_deref(), Some("Bio"));
        assert_eq!(cached.similar_ids, ["ar2"]);
        assert_eq!((cached.has_photo, cached.fetched_at), (Some(true), 42));
    }

    #[test]
    fn top_songs_go_first_then_the_rest_in_order() {
        let songs = strings(&["a", "b", "c", "d"]);
        assert_eq!(
            order_by_top_songs(songs.clone(), &strings(&["c", "x", "a", "c"])),
            Some(strings(&["c", "a", "b", "d"]))
        );
        assert_eq!(order_by_top_songs(songs, &strings(&["x"])), None, "none in the library");
    }
}
