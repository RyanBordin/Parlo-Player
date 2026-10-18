//! The operating system's display language, handed to the interface before
//! it loads, so it can open in that language (pt-BR or en; spec 12.7).
//!
//! The webview's `navigator.languages` is not used: on Windows it follows
//! the "preferred languages" list, which can start with a language that is
//! not the one Windows is displayed in.

use tauri::Runtime;
use tauri::plugin::{Builder, TauriPlugin};

/// Plugin that sets `window.__PARLO_SYSTEM_LANGUAGE__` (a BCP 47 tag like
/// "pt-BR", or "" if unknown) before any page script runs.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    let language = system_language().unwrap_or_default();
    // JSON encoding gives a safe JavaScript string literal.
    let literal = serde_json::to_string(&language).unwrap_or_else(|_| "\"\"".into());
    Builder::new("system-language")
        .js_init_script(format!("window.__PARLO_SYSTEM_LANGUAGE__ = {literal};"))
        .build()
}

/// Windows display language (the same one `Get-UICulture` shows).
#[cfg(windows)]
fn system_language() -> Option<String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
        fn LCIDToLocaleName(locale: u32, name: *mut u16, size: i32, flags: u32) -> i32;
    }
    // LOCALE_NAME_MAX_LENGTH, including the final NUL.
    let mut name = [0u16; 85];
    // SAFETY: both functions only read their arguments; `name` is a valid
    // buffer of the size passed, and the returned length counts the NUL.
    let length = unsafe {
        LCIDToLocaleName(u32::from(GetUserDefaultUILanguage()), name.as_mut_ptr(), name.len() as i32, 0)
    };
    let length = usize::try_from(length).ok().filter(|&n| n > 1)?;
    Some(String::from_utf16_lossy(&name[..length - 1]))
}

/// Linux: the message language from the environment, e.g. `pt_BR.UTF-8`.
#[cfg(not(windows))]
fn system_language() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|key| std::env::var(key).ok())
        .find(|value| !value.is_empty())
        .and_then(|value| posix_to_tag(&value))
}

/// `pt_BR.UTF-8` → `pt-BR`; `C` and `POSIX` mean no language.
#[cfg_attr(windows, allow(dead_code))]
fn posix_to_tag(value: &str) -> Option<String> {
    let base = value.split(['.', '@']).next()?.trim();
    if base.is_empty() || base == "C" || base == "POSIX" {
        return None;
    }
    Some(base.replace('_', "-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_locales_become_language_tags() {
        assert_eq!(posix_to_tag("pt_BR.UTF-8").as_deref(), Some("pt-BR"));
        assert_eq!(posix_to_tag("en_US").as_deref(), Some("en-US"));
        assert_eq!(posix_to_tag("de_DE@euro").as_deref(), Some("de-DE"));
        assert_eq!(posix_to_tag("C.UTF-8"), None);
        assert_eq!(posix_to_tag("POSIX"), None);
    }

    #[test]
    fn system_language_is_a_tag_when_known() {
        if let Some(tag) = system_language() {
            assert!(!tag.is_empty() && !tag.contains(['_', '.']), "{tag}");
        }
    }
}
