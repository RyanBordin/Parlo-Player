//! Integration tests against a real Navidrome server.
//!
//! Ignored by default. To run them, copy `.env.example` to `.env` at the
//! repository root, fill in your server details and run:
//!
//! ```sh
//! cargo test -- --ignored
//! ```

use parlo_player_lib::subsonic::{SubsonicClient, SubsonicError};

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
