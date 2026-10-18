//! Integration tests against a real Navidrome server.
//!
//! Ignored by default. To run them, copy `.env.example` to `.env` at the
//! repository root, fill in your server details and run:
//!
//! ```sh
//! cargo test -- --ignored
//! ```

use parlo_player_lib::subsonic::{SearchPage, SubsonicClient, SubsonicError};

struct TestServer {
    url: String,
    user: String,
    password: String,
}

fn test_server() -> TestServer {
    // Searches for `.env` in the current directory and its parents.
    let _ = dotenvy::dotenv();
    let var = |name: &str| {
        std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set (see .env.example)"))
    };

    TestServer {
        url: var("PARLO_TEST_URL"),
        user: var("PARLO_TEST_USER"),
        password: var("PARLO_TEST_PASSWORD"),
    }
}

#[tokio::test]
#[ignore = "requires a Navidrome server configured in .env"]
async fn ping_succeeds_with_valid_credentials() {
    let server = test_server();
    let client = SubsonicClient::new(server.url, server.user, server.password).unwrap();

    let info = client.ping().await.expect("ping should succeed");
    assert!(!info.api_version.is_empty());
}

#[tokio::test]
#[ignore = "requires a Navidrome server configured in .env"]
async fn ping_fails_with_wrong_password() {
    let server = test_server();
    let wrong_password = format!("{}-wrong", server.password);
    let client = SubsonicClient::new(server.url, server.user, wrong_password).unwrap();

    match client.ping().await {
        Err(SubsonicError::Api { code, .. }) => assert_eq!(code, 40),
        other => panic!("expected error 40, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires a Navidrome server configured in .env"]
async fn empty_search3_returns_library_pages() {
    let server = test_server();
    let client = SubsonicClient::new(server.url, server.user, server.password).unwrap();

    let first = client.search3(SearchPage::songs(10, 0)).await.unwrap();
    assert!(!first.songs.is_empty(), "empty query should return songs");
    assert!(first.artists.is_empty() && first.albums.is_empty(), "count 0 skips other types");

    // Pages must not overlap: natural order is stable.
    let second = client.search3(SearchPage::songs(10, 10)).await.unwrap();
    if let (Some(a), Some(b)) = (first.songs.last(), second.songs.first()) {
        assert_ne!(a.id, b.id);
    }
}

#[tokio::test]
#[ignore = "requires a Navidrome server configured in .env"]
async fn scan_status_reports_song_count() {
    let server = test_server();
    let client = SubsonicClient::new(server.url, server.user, server.password).unwrap();

    let status = client.get_scan_status().await.unwrap();
    assert!(status.count.unwrap_or(0) > 0);
}

#[tokio::test]
#[ignore = "requires a Navidrome server configured in .env"]
async fn server_takes_post_forms_and_reads_the_play_queue() {
    let server = test_server();
    let client = SubsonicClient::new(server.url, server.user, server.password).unwrap();

    // Large queues are saved by POST (spec 5.6). Read-only: the saved queue
    // is not changed.
    assert!(client.supports_form_post().await, "server should announce formPost");
    if let Some(queue) = client.get_play_queue().await.unwrap() {
        assert!(!queue.entry.is_empty());
        assert!(queue.changed.is_some());
    }
}
