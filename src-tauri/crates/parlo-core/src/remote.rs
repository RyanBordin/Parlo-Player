//! Remote control on the network (spec 12.8): the phone shows and controls
//! what the computer is playing.
//!
//! The computer announces itself with mDNS and accepts WebSocket connections;
//! the phone finds it and connects. This module holds the parts both sides
//! share and that need no network: the messages, the proof that the phone
//! signs in to the same account, and the window of the queue that is sent.
//!
//! Sign in: the computer sends a random `nonce`, the phone answers with its
//! own and `proof = md5("parlo-remote:" + nonce + ":" + phone nonce + ":" +
//! password)`. The password never travels, and the fixed prefix keeps the
//! proof from ever being a valid Subsonic token (`md5(password + salt)`), so a
//! fake computer cannot turn it into access to the music server.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

use crate::player::events::{PlayerState, Position, Track};
use crate::player::queue::RepeatMode;

/// The mDNS service the computer announces.
pub const SERVICE_TYPE: &str = "_parlo-remote._tcp.local.";
/// Bumped when the messages change in a way the other side cannot read.
pub const PROTOCOL_VERSION: u32 = 1;
/// Fixed, so a firewall rule keeps working (and an address can be typed one
/// day); another one is used if it is taken.
pub const DEFAULT_PORT: u16 = 47_410;
/// Songs of the queue sent to the phone: the current one and those after it.
pub const QUEUE_WINDOW: usize = 100;

/// TXT keys of the announcement.
pub const TXT_VERSION: &str = "v";
pub const TXT_USER: &str = "user";
pub const TXT_NAME: &str = "name";

/// Identifies the account in the announcement without showing its name:
/// the phone only lists computers signed in with the same user.
pub fn user_tag(username: &str) -> String {
    let hex = md5_hex(&format!("parlo-remote-user:{}", username.trim().to_lowercase()));
    hex[..16].to_string()
}

/// What the phone answers to the computer's nonce (see the module docs).
pub fn proof(password: &str, server_nonce: &str, client_nonce: &str) -> String {
    md5_hex(&format!("parlo-remote:{server_nonce}:{client_nonce}:{password}"))
}

/// A fresh random nonce for the sign in.
pub fn new_nonce() -> String {
    crate::subsonic::auth::generate_salt()
}

/// Compares two proofs in a time that does not depend on where they differ.
pub fn same_proof(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn md5_hex(text: &str) -> String {
    Md5::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// From the computer to the phone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ServerMessage {
    /// First message: who the computer is, and the nonce to sign.
    Challenge { version: u32, name: String, nonce: String },
    /// The proof was right: the state follows.
    Welcome,
    /// The proof was wrong, or nobody is signed in on the computer.
    Denied { reason: DeniedReason },
    State { state: PlayerState },
    Position { position: Position },
    Queue { queue: RemoteQueue },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeniedReason {
    /// Another account, or another password.
    WrongAccount,
    /// Nobody is signed in on the computer.
    SignedOut,
    /// The other side speaks another version of the messages.
    Version,
}

/// From the phone to the computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ClientMessage {
    /// Answer to `Challenge`.
    Hello { version: u32, device: String, nonce: String, proof: String },
    Command { command: RemoteCommand },
}

/// What the phone can do on the computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RemoteCommand {
    TogglePause,
    Next,
    Previous,
    /// 0 to 100.
    SetVolume { volume: f64 },
    /// A song of Up next. The ID guards against a queue that changed meanwhile.
    JumpTo { index: usize, song_id: String },
}

/// The part of the computer's queue the phone shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteQueue {
    /// "Playing from".
    pub label: String,
    pub continuing_from: Option<String>,
    /// Position in the whole queue of the first song of `tracks`.
    pub offset: usize,
    pub tracks: Vec<Track>,
    /// Position in the whole queue.
    pub current_index: Option<usize>,
    /// Songs in the whole queue.
    pub total: usize,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

/// Where the window of `QUEUE_WINDOW` songs starts and ends: from the current
/// song (or the first) on.
pub fn queue_window(total: usize, current: Option<usize>) -> std::ops::Range<usize> {
    let start = current.unwrap_or(0).min(total);
    start..(start + QUEUE_WINDOW).min(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_tag_ignores_case_and_spaces_and_hides_the_name() {
        assert_eq!(user_tag("Ryan"), user_tag(" ryan "));
        assert_ne!(user_tag("ryan"), user_tag("ana"));
        assert_eq!(user_tag("ryan").len(), 16);
        assert!(!user_tag("ryan").contains("ryan"));
    }

    #[test]
    fn proof_depends_on_every_part_and_is_not_a_subsonic_token() {
        let base = proof("sesame", "abc", "xyz");
        assert_ne!(base, proof("other", "abc", "xyz"));
        assert_ne!(base, proof("sesame", "abd", "xyz"));
        assert_ne!(base, proof("sesame", "abc", "xyw"));
        // md5(password + salt) for any salt the computer could pick.
        assert_ne!(base, crate::subsonic::auth::make_token("sesame", "abc"));
        assert_eq!(base.len(), 32);
    }

    #[test]
    fn same_proof_compares_whole_strings() {
        assert!(same_proof("abc", "abc"));
        assert!(!same_proof("abc", "abd"));
        assert!(!same_proof("abc", "ab"));
    }

    #[test]
    fn window_starts_at_the_current_song() {
        assert_eq!(queue_window(0, None), 0..0);
        assert_eq!(queue_window(10, None), 0..10);
        assert_eq!(queue_window(10, Some(4)), 4..10);
        assert_eq!(queue_window(500, Some(20)), 20..120);
        assert_eq!(queue_window(5, Some(9)), 5..5);
    }

    #[test]
    fn messages_have_the_documented_shape() {
        let json = serde_json::to_value(ClientMessage::Command {
            command: RemoteCommand::JumpTo { index: 3, song_id: "a1".into() },
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "type": "command", "command": { "action": "jumpTo", "index": 3, "songId": "a1" } })
        );
        let json = serde_json::to_value(ServerMessage::Denied { reason: DeniedReason::WrongAccount }).unwrap();
        assert_eq!(json, serde_json::json!({ "type": "denied", "reason": "wrongAccount" }));
    }

    #[test]
    fn messages_survive_a_round_trip() {
        let messages = [
            ServerMessage::Challenge { version: 1, name: "Desk".into(), nonce: "n".into() },
            ServerMessage::Welcome,
            ServerMessage::Position { position: Position { position: 12.5, duration: Some(200.0) } },
            ServerMessage::Queue {
                queue: RemoteQueue {
                    label: "Album".into(),
                    tracks: vec![Track { id: "s1".into(), title: "Song".into(), ..Track::default() }],
                    current_index: Some(0),
                    total: 1,
                    ..RemoteQueue::default()
                },
            },
        ];
        for message in messages {
            let text = serde_json::to_string(&message).unwrap();
            assert_eq!(serde_json::from_str::<ServerMessage>(&text).unwrap(), message);
        }
    }
}
