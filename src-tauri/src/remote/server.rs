//! The computer's side of remote control (spec 12.8).
//!
//! A WebSocket server on the local network, announced with mDNS while
//! someone is signed in. A phone signs in with the proof of the same account
//! (see `parlo_core::remote`), then gets the player state, the position and
//! the queue as they change, and sends commands.

use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tauri::{AppHandle, Listener, Manager};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, Semaphore, broadcast};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use super::{
    ClientMessage, DEFAULT_PORT, DeniedReason, PROTOCOL_VERSION, RemoteCommand, SERVICE_TYPE, ServerMessage,
    TXT_NAME, TXT_USER, TXT_VERSION, new_nonce, same_proof, user_tag,
};
use crate::playback::{
    PLAYER_POSITION_EVENT, PLAYER_STATE_EVENT, Playback, PlaybackError, QUEUE_CHANGED_EVENT, QUEUE_POSITION_EVENT,
};
use crate::player::PlayerState;
use crate::player::events::Position;
use crate::state::{AppState, PlaybackSlot};

/// Phones connected at the same time.
const MAX_CLIENTS: usize = 8;
/// Time a phone has to sign in after connecting.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// A wrong proof waits this long before the answer, so guessing is slow.
const DENY_DELAY: Duration = Duration::from_secs(1);
/// How often the announcement checks who is signed in.
const ANNOUNCE_CHECK: Duration = Duration::from_secs(5);
/// The position goes to the phones at most this often.
const POSITION_INTERVAL: Duration = Duration::from_millis(500);
/// Messages kept for a slow phone; past that it gets the whole state again.
const BACKLOG: usize = 64;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Sink = SplitSink<WebSocketStream<TcpStream>, Message>;
type Stream = SplitStream<WebSocketStream<TcpStream>>;

/// The mDNS announcement, kept to take it back when the app closes.
struct Announcement {
    daemon: ServiceDaemon,
    fullname: Mutex<Option<String>>,
}

static ANNOUNCEMENT: OnceLock<Announcement> = OnceLock::new();

/// Starts the server and the announcement in the background. Failing to
/// start only disables remote control (logged).
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run(app).await {
            log::warn!("remote: the server stopped: {error}");
        }
    });
}

/// Takes the announcement back (the app is closing), so phones stop listing
/// this computer at once.
pub fn stop() {
    if let Some(announcement) = ANNOUNCEMENT.get() {
        if let Some(fullname) = lock(&announcement.fullname).take()
            && let Ok(done) = announcement.daemon.unregister(&fullname)
        {
            let _ = done.recv_timeout(Duration::from_millis(300));
        }
        let _ = announcement.daemon.shutdown();
    }
}

async fn run(app: AppHandle) -> Result<(), Error> {
    let listener = match TcpListener::bind(("0.0.0.0", DEFAULT_PORT)).await {
        Ok(listener) => listener,
        Err(error) => {
            log::info!("remote: port {DEFAULT_PORT} is taken ({error}), using another one");
            TcpListener::bind(("0.0.0.0", 0)).await?
        }
    };
    let port = listener.local_addr()?.port();
    log::info!("remote: listening on port {port}");

    let (events, _) = broadcast::channel::<ServerMessage>(BACKLOG);
    forward_player_events(&app, events.clone());
    tauri::async_runtime::spawn(announce(app.clone(), port));

    let slots = Arc::new(Semaphore::new(MAX_CLIENTS));
    loop {
        let (stream, address) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                log::warn!("remote: could not accept a connection: {error}");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
        let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
            log::info!("remote: too many phones connected, refusing {address}");
            continue;
        };
        let app = app.clone();
        let updates = events.subscribe();
        tauri::async_runtime::spawn(async move {
            match serve(&app, stream, updates).await {
                Ok(()) => log::info!("remote: {address} disconnected"),
                Err(error) => log::info!("remote: {address} disconnected: {error}"),
            }
            drop(slot);
        });
    }
}

/// Sends the player's events to the connected phones. The queue is read
/// again on another task: the events are emitted while the queue is locked.
fn forward_player_events(app: &AppHandle, events: broadcast::Sender<ServerMessage>) {
    app.listen_any(PLAYER_STATE_EVENT, {
        let events = events.clone();
        move |event| {
            if let Ok(state) = serde_json::from_str::<PlayerState>(event.payload()) {
                let _ = events.send(ServerMessage::State { state });
            }
        }
    });

    let last_position = Mutex::new(None::<Instant>);
    app.listen_any(PLAYER_POSITION_EVENT, {
        let events = events.clone();
        move |event| {
            if events.receiver_count() == 0 {
                return;
            }
            let now = Instant::now();
            {
                let mut last = lock(&last_position);
                if last.is_some_and(|at| now.duration_since(at) < POSITION_INTERVAL) {
                    return;
                }
                *last = Some(now);
            }
            if let Ok(position) = serde_json::from_str::<Position>(event.payload()) {
                let _ = events.send(ServerMessage::Position { position });
            }
        }
    });

    let queue_changed = Arc::new(Notify::new());
    for name in [QUEUE_CHANGED_EVENT, QUEUE_POSITION_EVENT] {
        let queue_changed = Arc::clone(&queue_changed);
        app.listen_any(name, move |_| queue_changed.notify_one());
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            queue_changed.notified().await;
            if events.receiver_count() == 0 {
                continue;
            }
            let Some(playback) = playback(&app) else { continue };
            if let Ok(queue) = tauri::async_runtime::spawn_blocking(move || playback.remote_queue()).await {
                let _ = events.send(ServerMessage::Queue { queue });
            }
        }
    });
}

/// Announces this computer while someone is signed in, with the tag of that
/// account; signing in with another account announces again.
async fn announce(app: AppHandle, port: u16) {
    let daemon = match ServiceDaemon::new() {
        Ok(daemon) => daemon,
        Err(error) => {
            log::warn!("remote: mDNS is not available, phones will not find this computer: {error}");
            return;
        }
    };
    let announcement = ANNOUNCEMENT.get_or_init(|| Announcement { daemon, fullname: Mutex::new(None) });
    let name = computer_name();
    let mut announced: Option<String> = None;
    loop {
        let tag = app.state::<Arc<AppState>>().active().map(|library| user_tag(library.client.username()));
        if tag != announced {
            if let Some(fullname) = lock(&announcement.fullname).take() {
                let _ = announcement.daemon.unregister(&fullname);
            }
            if let Some(tag) = &tag {
                match service_info(&name, tag, port) {
                    Ok(info) => {
                        let fullname = info.get_fullname().to_string();
                        match announcement.daemon.register(info) {
                            Ok(()) => *lock(&announcement.fullname) = Some(fullname),
                            Err(error) => log::warn!("remote: could not announce this computer: {error}"),
                        }
                    }
                    Err(error) => log::warn!("remote: could not announce this computer: {error}"),
                }
            }
            announced = tag;
        }
        tokio::time::sleep(ANNOUNCE_CHECK).await;
    }
}

fn service_info(name: &str, tag: &str, port: u16) -> Result<ServiceInfo, mdns_sd::Error> {
    let host = format!("{}.local.", host_label(name));
    let version = PROTOCOL_VERSION.to_string();
    let properties = [(TXT_VERSION, version.as_str()), (TXT_USER, tag), (TXT_NAME, name)];
    Ok(ServiceInfo::new(SERVICE_TYPE, name, &host, "", port, &properties[..])?.enable_addr_auto())
}

/// One phone: sign in, then the state, then updates and commands.
async fn serve(app: &AppHandle, stream: TcpStream, mut updates: broadcast::Receiver<ServerMessage>) -> Result<(), Error> {
    let _ = stream.set_nodelay(true);
    let (mut sink, mut stream) = tokio_tungstenite::accept_async(stream).await?.split();

    let nonce = new_nonce();
    send(&mut sink, &ServerMessage::Challenge { version: PROTOCOL_VERSION, name: computer_name(), nonce: nonce.clone() })
        .await?;
    let hello = tokio::time::timeout(HELLO_TIMEOUT, receive(&mut stream)).await.map_err(|_| "no sign in")??;
    let Some(ClientMessage::Hello { version, device, nonce: phone_nonce, proof }) = hello else {
        return Err("expected a sign in".into());
    };
    if version != PROTOCOL_VERSION {
        return deny(&mut sink, DeniedReason::Version).await;
    }
    let Some(library) = app.state::<Arc<AppState>>().active() else {
        return deny(&mut sink, DeniedReason::SignedOut).await;
    };
    if !same_proof(&library.client.remote_proof(&nonce, &phone_nonce), &proof) {
        tokio::time::sleep(DENY_DELAY).await;
        return deny(&mut sink, DeniedReason::WrongAccount).await;
    }
    log::info!("remote: \"{device}\" connected");
    send(&mut sink, &ServerMessage::Welcome).await?;
    send_snapshot(app, &mut sink).await?;

    loop {
        tokio::select! {
            message = receive(&mut stream) => match message? {
                Some(ClientMessage::Command { command }) => run_command(app, command).await,
                Some(ClientMessage::Hello { .. }) => {}
                None => return Ok(()),
            },
            update = updates.recv() => match update {
                Ok(message) => send(&mut sink, &message).await?,
                // Too slow to keep up: everything again, as it is now.
                Err(broadcast::error::RecvError::Lagged(_)) => send_snapshot(app, &mut sink).await?,
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            },
        }
    }
}

async fn deny(sink: &mut Sink, reason: DeniedReason) -> Result<(), Error> {
    send(sink, &ServerMessage::Denied { reason }).await?;
    let _ = sink.close().await;
    Err(format!("sign in refused: {reason:?}").into())
}

/// The player, the position and the queue as they are now.
async fn send_snapshot(app: &AppHandle, sink: &mut Sink) -> Result<(), Error> {
    let Some(playback) = playback(app) else { return Ok(()) };
    let messages = tauri::async_runtime::spawn_blocking(move || {
        let player = playback.player();
        let state = player.state();
        let position = Position { position: player.position(), duration: state.track.as_ref().and_then(|t| t.duration) };
        [
            ServerMessage::State { state },
            ServerMessage::Position { position },
            ServerMessage::Queue { queue: playback.remote_queue() },
        ]
    })
    .await?;
    for message in &messages {
        send(sink, message).await?;
    }
    Ok(())
}

async fn run_command(app: &AppHandle, command: RemoteCommand) {
    let Some(playback) = playback(app) else { return };
    let result = tauri::async_runtime::spawn_blocking(move || apply(&playback, command)).await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => log::warn!("remote: a command failed: {error}"),
        Err(error) => log::warn!("remote: a command failed: {error}"),
    }
}

fn apply(playback: &Playback, command: RemoteCommand) -> Result<(), PlaybackError> {
    match command {
        RemoteCommand::TogglePause => playback.player().toggle_pause()?,
        RemoteCommand::Next => playback.next()?,
        RemoteCommand::Previous => playback.previous()?,
        RemoteCommand::SetVolume { volume } => {
            playback.player().set_volume(volume)?;
        }
        RemoteCommand::JumpTo { index, song_id } => playback.jump_to_song(index, &song_id)?,
    }
    Ok(())
}

async fn send(sink: &mut Sink, message: &ServerMessage) -> Result<(), Error> {
    sink.send(Message::text(serde_json::to_string(message)?)).await?;
    Ok(())
}

/// The next message from the phone; `None` when it closed the connection.
async fn receive(stream: &mut Stream) -> Result<Option<ClientMessage>, Error> {
    while let Some(message) = stream.next().await {
        match message? {
            Message::Text(text) => match serde_json::from_str(&text) {
                Ok(message) => return Ok(Some(message)),
                Err(error) => log::info!("remote: unreadable message: {error}"),
            },
            Message::Close(_) => return Ok(None),
            _ => {}
        }
    }
    Ok(None)
}

fn playback(app: &AppHandle) -> Option<Arc<Playback>> {
    app.state::<PlaybackSlot>().0.as_ref().ok().cloned()
}

/// The name phones show: the computer's name.
fn computer_name() -> String {
    let name = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    name.unwrap_or_else(|| "Parlo".to_string())
}

/// `name` as an mDNS host label: letters, digits and hyphens.
fn host_label(name: &str) -> String {
    let label: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(63)
        .collect();
    if label.is_empty() { "parlo".to_string() } else { format!("{label}-parlo") }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_label_keeps_letters_digits_and_hyphens() {
        assert_eq!(host_label("DESKTOP-AB12"), "desktop-ab12-parlo");
        assert_eq!(host_label("Ryan's PC"), "ryan-s-pc-parlo");
        assert_eq!(host_label("???"), "parlo");
    }
}
