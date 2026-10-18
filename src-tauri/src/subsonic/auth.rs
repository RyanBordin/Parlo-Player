//! Subsonic token authentication (API 1.13.0+).
//!
//! Each request sends `t = md5(password + salt)` and `s = salt`, so the
//! password itself never travels over the network.

use md5::{Digest, Md5};
use rand::{RngExt, distr::Alphanumeric};

const SALT_LENGTH: usize = 16;

/// Generates a new random alphanumeric salt. Call once per request.
pub fn generate_salt() -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(SALT_LENGTH)
        .map(char::from)
        .collect()
}

/// Computes the Subsonic token: lowercase hex of `md5(password + salt)`.
pub fn make_token(password: &str, salt: &str) -> String {
    let digest = Md5::digest(format!("{password}{salt}").as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_matches_subsonic_docs_example() {
        // Example from the official Subsonic API documentation.
        assert_eq!(
            make_token("sesame", "c19b2d"),
            "26719a1196d2a940705a59634eb18eab"
        );
    }

    #[test]
    fn salt_has_expected_length_and_charset() {
        let salt = generate_salt();
        assert_eq!(salt.len(), SALT_LENGTH);
        assert!(salt.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn salts_are_different_each_time() {
        assert_ne!(generate_salt(), generate_salt());
    }
}
