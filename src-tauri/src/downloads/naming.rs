//! Names of "Save to computer" (spec 8.2): files and folders valid on
//! Windows and Linux, the layout of each kind of item, the rule for files
//! already there, and the playlist's `.m3u8`. Pure logic: the file system is
//! only asked through `exists`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::db::items::SaveSong;

/// Longest name part, in bytes: Linux allows 255 per name, and the suffix
/// " (n)" and the extension still have to fit.
const MAX_NAME_BYTES: usize = 180;
/// Names Windows keeps for devices, with or without an extension.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
    "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// A name valid on both systems: characters Windows refuses become "_",
/// no trailing dot or space, no leading dot (hidden on Linux), no device
/// name, not too long, never empty.
pub fn clean(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| match c {
            // Tabs and line breaks are spaces; other control characters are not allowed.
            c if c.is_whitespace() => ' ',
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let mut text = replaced.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.len() > MAX_NAME_BYTES {
        let mut end = MAX_NAME_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    let text = text.trim_end_matches(['.', ' ']);
    let mut text = match text.strip_prefix('.') {
        Some(rest) => format!("_{rest}"),
        None => text.to_string(),
    };
    let stem = text.split('.').next().unwrap_or("").to_ascii_uppercase();
    if RESERVED.contains(&stem.as_str()) {
        text.insert(0, '_');
    }
    if text.is_empty() { "_".to_string() } else { text }
}

/// The file's extension: the server's format, else the one of its path.
pub fn extension(song: &SaveSong) -> String {
    let from_path = || {
        song.path
            .as_deref()
            .and_then(|path| Path::new(path).extension())
            .map(|ext| ext.to_string_lossy().into_owned())
    };
    let ext = song.suffix.clone().filter(|s| !s.is_empty()).or_else(from_path).unwrap_or_else(|| "bin".into());
    clean(&ext.to_lowercase())
}

fn file(stem: &str, song: &SaveSong) -> String {
    format!("{}.{}", clean(stem), extension(song))
}

/// A song, a selection: "Artist - Title.ext".
pub fn song_file(song: &SaveSong) -> String {
    match song.artist.as_deref().filter(|a| !a.is_empty()) {
        Some(artist) => file(&format!("{artist} - {}", song.title), song),
        None => file(&song.title, song),
    }
}

/// An album's folder: "Album artist - Album (Year)".
pub fn album_folder(artist: Option<&str>, album: &str, year: Option<i64>) -> String {
    let mut name = match artist.filter(|a| !a.is_empty()) {
        Some(artist) => format!("{artist} - {album}"),
        None => album.to_string(),
    };
    if let Some(year) = year.filter(|y| *y > 0) {
        name.push_str(&format!(" ({year})"));
    }
    clean(&name)
}

/// A song of an album: "NN - Title.ext", or "D-NN - Title.ext" with more
/// than one disc.
pub fn album_file(song: &SaveSong, many_discs: bool) -> String {
    let track = song.track.unwrap_or(0).max(0);
    if many_discs {
        file(&format!("{}-{track:02} - {}", song.disc_number.unwrap_or(1).max(0), song.title), song)
    } else {
        file(&format!("{track:02} - {}", song.title), song)
    }
}

/// A song of a playlist, at `position` (from 1): "NNN - Artist - Title.ext".
pub fn playlist_file(position: usize, song: &SaveSong) -> String {
    match song.artist.as_deref().filter(|a| !a.is_empty()) {
        Some(artist) => file(&format!("{position:03} - {artist} - {}", song.title), song),
        None => file(&format!("{position:03} - {}", song.title), song),
    }
}

/// The playlist file other players open: the songs in order, by their name
/// next to it (UTF-8, so any title fits).
pub fn m3u8(entries: &[(&SaveSong, String)]) -> String {
    let mut text = String::from("#EXTM3U\n");
    for (song, name) in entries {
        let title = match song.artist.as_deref().filter(|a| !a.is_empty()) {
            Some(artist) => format!("{artist} - {}", song.title),
            None => song.title.clone(),
        };
        let title = title.replace(['\r', '\n'], " ");
        text.push_str(&format!("#EXTINF:{},{title}\n{name}\n", song.duration.unwrap_or(-1)));
    }
    text
}

/// `path` with " (n)" before the extension.
fn numbered(path: &Path, n: usize) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let name = match path.extension() {
        Some(ext) => format!("{stem} ({n}).{}", ext.to_string_lossy()),
        None => format!("{stem} ({n})"),
    };
    path.with_file_name(name)
}

/// Where a file goes (spec 8.2): a file already there with the same size is
/// the same file, so it is skipped (`None`); with another size the new one
/// gets " (1)", " (2)"... `taken` holds the paths already given in this
/// save (compared without case: Windows ignores it). `exists` gives the size
/// of a file already on disk.
pub fn target(
    path: &Path,
    size: Option<u64>,
    taken: &mut HashSet<String>,
    exists: impl Fn(&Path) -> Option<u64>,
) -> Option<PathBuf> {
    let mut candidate = path.to_path_buf();
    for n in 1.. {
        let key = candidate.to_string_lossy().to_lowercase();
        if !taken.contains(&key) {
            match exists(&candidate) {
                None => {
                    taken.insert(key);
                    return Some(candidate);
                }
                Some(on_disk) if Some(on_disk) == size => return None,
                Some(_) => {}
            }
        }
        candidate = numbered(path, n);
    }
    unreachable!("an unused name is always found")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(title: &str, artist: Option<&str>) -> SaveSong {
        SaveSong {
            id: "s1".into(),
            title: title.into(),
            artist: artist.map(Into::into),
            suffix: Some("FLAC".into()),
            track: Some(3),
            disc_number: Some(2),
            duration: Some(185),
            ..SaveSong::default()
        }
    }

    #[test]
    fn names_lose_what_windows_refuses() {
        assert_eq!(clean("AC/DC: Live? <2>*"), "AC_DC_ Live_ _2__");
        assert_eq!(clean("  Tabs\tand\nlines  "), "Tabs and lines");
        assert_eq!(clean("Ends with dots..."), "Ends with dots");
        assert_eq!(clean(".hidden"), "_hidden");
        assert_eq!(clean("con"), "_con");
        assert_eq!(clean("NUL.txt"), "_NUL.txt");
        assert_eq!(clean("Console"), "Console", "only the exact device names");
        assert_eq!(clean("???"), "___");
        assert_eq!(clean(""), "_");
        assert_eq!(clean("..."), "_");
    }

    #[test]
    fn long_names_are_cut_on_a_character() {
        let name = clean(&"é".repeat(200));
        assert!(name.len() <= MAX_NAME_BYTES);
        assert!(name.chars().all(|c| c == 'é'));
    }

    #[test]
    fn layouts_follow_the_spec() {
        let s = song("Song / Part 1", Some("Band"));
        assert_eq!(song_file(&s), "Band - Song _ Part 1.flac");
        assert_eq!(song_file(&song("Alone", None)), "Alone.flac");
        assert_eq!(album_file(&s, false), "03 - Song _ Part 1.flac");
        assert_eq!(album_file(&s, true), "2-03 - Song _ Part 1.flac");
        assert_eq!(playlist_file(7, &s), "007 - Band - Song _ Part 1.flac");
        assert_eq!(album_folder(Some("Band"), "Best Of", Some(1999)), "Band - Best Of (1999)");
        assert_eq!(album_folder(None, "Best Of", None), "Best Of");
    }

    #[test]
    fn extension_falls_back_to_the_path() {
        let mut s = song("t", None);
        s.suffix = None;
        s.path = Some("Music/a/t.Opus".into());
        assert_eq!(extension(&s), "opus");
        s.path = None;
        assert_eq!(extension(&s), "bin");
    }

    #[test]
    fn m3u8_lists_the_songs_in_order() {
        let a = song("One", Some("Band"));
        let b = song("Two\nlines", None);
        let text = m3u8(&[(&a, "001 - Band - One.flac".into()), (&b, "002 - Two lines.flac".into())]);
        assert_eq!(
            text,
            "#EXTM3U\n#EXTINF:185,Band - One\n001 - Band - One.flac\n#EXTINF:185,Two lines\n002 - Two lines.flac\n"
        );
    }

    #[test]
    fn files_already_there_are_skipped_or_numbered() {
        let on_disk = |path: &Path| match path.file_name()?.to_str()? {
            "same.flac" => Some(100),
            "other.flac" | "other (1).flac" => Some(5),
            _ => None,
        };
        let mut taken = HashSet::new();
        let dir = Path::new("dir");
        assert_eq!(target(&dir.join("new.flac"), Some(100), &mut taken, on_disk), Some(dir.join("new.flac")));
        assert_eq!(target(&dir.join("same.flac"), Some(100), &mut taken, on_disk), None, "same size: skipped");
        assert_eq!(
            target(&dir.join("other.flac"), Some(100), &mut taken, on_disk),
            Some(dir.join("other (2).flac")),
            "another size: the next free number"
        );
        assert_eq!(
            target(&dir.join("NEW.flac"), Some(100), &mut taken, on_disk),
            Some(dir.join("NEW (1).flac")),
            "already given in this save, whatever the case"
        );
        assert_eq!(target(&dir.join("same.flac"), None, &mut taken, on_disk), Some(dir.join("same (1).flac")));
    }
}
