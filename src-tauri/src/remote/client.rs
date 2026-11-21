//! The phone's side of remote control (spec 12.8).
//!
//! While Now Playing is open, the phone looks for the computers announced
//! with mDNS and lists those signed in with the same account. Connecting
//! signs in with the proof (see `parlo_core::remote`) and turns what the
//! computer sends into events for the interface:
//!
//! - `remote-devices`: the computers found (`RemoteDevice` list);
//! - `remote-status`: the connection (`RemoteStatus`);
//! - `remote-state`, `remote-position`, `remote-queue`: what the computer plays.

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::{
    ClientMessage, DeniedReason, PROTOCOL_VERSION, RemoteCommand, SERVICE_TYPE, ServerMessage, TXT_NAME, TXT_USER,
    TXT_VERSION, new_nonce, user_tag,
};
use crate::state::AppState;

pub const DEVICES_EVENT: &str = "remote-devices";
pub const STATUS_EVENT: &str = "remote-status";
pub const STATE_EVENT: &str = "remote-state";
pub const POSITION_EVENT: &str = "remote-position";
pub const QUEUE_EVENT: &str = "remote-queue";

/// Time to open the connection to one address of the computer.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
/// Time the computer has to answer the sign in.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(10);
/// The name the computer writes in its log.
const DEVICE_NAME: &str = "Parlo (Android)";

/// A computer found on the network, signed in with the same account.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDevice {
    /// The mDNS name of the announcement; stays the same while it runs.
    pub id: String,
    pub name: String,
    #[serde(skip)]
    pub addresses: Vec<IpAddr>,
    #[serde(skip)]
    pub port: u16,
}

/// Payload of `remote-status`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStatus {
    pub device_id: String,
    pub name: String,
    pub status: ConnectionStatus,
    /// Why it closed, when it was not the phone's choice.
    pub problem: Option<Problem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Problem {
    /// No address of the computer answered (firewall, another network).
    Unreachable,
    /// The computer is signed in with another account or password.
    WrongAccount,
    /// Nobody is signed in on the computer (or on the phone).
    SignedOut,
    /// One of the two apps needs an update.
    Version,
    /// The connection dropped.
    Lost,
}

impl From<DeniedReason> for Problem {
    fn from(reason: DeniedReason) -> Self {
        match reason {
            DeniedReason::WrongAccount => Self::WrongAccount,
            DeniedReason::SignedOut => Self::SignedOut,
            DeniedReason::Version => Self::Version,
        }
    }
}

struct Connection {
    id: u64,
    commands: mpsc::UnboundedSender<RemoteCommand>,
}

/// Discovery and the connection to one computer, managed by the app.
#[derive(Default)]
pub struct RemoteClient {
    devices: Mutex<BTreeMap<String, RemoteDevice>>,
    discovery: Mutex<Option<ServiceDaemon>>,
    connection: Mutex<Option<Connection>>,
    next_id: AtomicU64,
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    #[error("not signed in")]
    NotLoggedIn,
    #[error("mDNS is not available: {0}")]
    Discovery(#[from] mdns_sd::Error),
    #[error("this computer is no longer on the network")]
    UnknownDevice,
    #[error("not connected to a computer")]
    NotConnected,
}

impl RemoteClient {
    /// Starts looking for computers (again) and returns those already found.
    pub fn start_discovery(self: &Arc<Self>, app: &AppHandle) -> Result<Vec<RemoteDevice>, RemoteError> {
        let library = app.state::<Arc<AppState>>().active().ok_or(RemoteError::NotLoggedIn)?;
        let tag = user_tag(library.client.username());
        let mut discovery = lock(&self.discovery);
        if discovery.is_none() {
            set_multicast(app, true);
            let daemon = ServiceDaemon::new()?;
            let events = daemon.browse(SERVICE_TYPE)?;
            *discovery = Some(daemon);
            let client = Arc::clone(self);
            let app = app.clone();
            std::thread::Builder::new()
                .name("remote-discovery".into())
                .spawn(move || {
                    while let Ok(event) = events.recv() {
                        if client.on_discovery_event(event, &tag) {
                            let _ = app.emit(DEVICES_EVENT, client.devices());
                        }
                    }
                })
                .map_err(|error| mdns_sd::Error::Msg(error.to_string()))?;
        }
        Ok(self.devices())
    }

    /// Stops looking (Now Playing closed). The list empties: what is on the
    /// network may change until the next look.
    pub fn stop_discovery(&self, app: &AppHandle) {
        if let Some(daemon) = lock(&self.discovery).take() {
            let _ = daemon.shutdown();
            set_multicast(app, false);
        }
        lock(&self.devices).clear();
    }

    pub fn devices(&self) -> Vec<RemoteDevice> {
        lock(&self.devices).values().cloned().collect()
    }

    /// Returns whether the list changed.
    fn on_discovery_event(&self, event: ServiceEvent, tag: &str) -> bool {
        let mut devices = lock(&self.devices);
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let version = info.get_property_val_str(TXT_VERSION).and_then(|v| v.parse::<u32>().ok());
                if info.get_property_val_str(TXT_USER) != Some(tag) || version != Some(PROTOCOL_VERSION) {
                    return false;
                }
                let name = info
                    .get_property_val_str(TXT_NAME)
                    .map(str::to_string)
                    .unwrap_or_else(|| mdns_sd::instance_name_unescaped(info.get_fullname()));
                let mut addresses: Vec<IpAddr> = info
                    .get_addresses()
                    .iter()
                    .map(|ip| ip.to_ip_addr())
                    .filter(|ip| !ip.is_loopback() && !is_link_local_v6(ip))
                    .collect();
                addresses.sort();
                let device = RemoteDevice { id: info.get_fullname().to_string(), name, addresses, port: info.get_port() };
                devices.insert(device.id.clone(), device.clone()) != Some(device)
            }
            ServiceEvent::ServiceRemoved(_, fullname) => devices.remove(&fullname).is_some(),
            _ => false,
        }
    }

    /// Connects to a computer found, closing any other connection.
    pub fn connect(self: &Arc<Self>, app: &AppHandle, device_id: &str) -> Result<(), RemoteError> {
        let device = lock(&self.devices).get(device_id).cloned().ok_or(RemoteError::UnknownDevice)?;
        let (commands, receiver) = mpsc::unbounded_channel();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // Dropping the old sender ends the old connection.
        *lock(&self.connection) = Some(Connection { id, commands });

        let client = Arc::clone(self);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let status = |status, problem| RemoteStatus {
                device_id: device.id.clone(),
                name: device.name.clone(),
                status,
                problem,
            };
            let _ = app.emit(STATUS_EVENT, status(ConnectionStatus::Connecting, None));
            let problem = session(&app, &device, receiver, || {
                let _ = app.emit(STATUS_EVENT, status(ConnectionStatus::Connected, None));
            })
            .await
            .err();
            if let Some(problem) = problem {
                log::info!("remote: connection to \"{}\" closed: {problem:?}", device.name);
            }
            let mut connection = lock(&client.connection);
            // A newer connection took its place: it reports for itself.
            if connection.as_ref().is_some_and(|c| c.id == id) {
                *connection = None;
                drop(connection);
                let _ = app.emit(STATUS_EVENT, status(ConnectionStatus::Closed, problem));
            }
        });
        Ok(())
    }

    pub fn disconnect(&self) {
        lock(&self.connection).take();
    }

    pub fn send(&self, command: RemoteCommand) -> Result<(), RemoteError> {
        let connection = lock(&self.connection);
        let connection = connection.as_ref().ok_or(RemoteError::NotConnected)?;
        connection.commands.send(command).map_err(|_| RemoteError::NotConnected)
    }
}

/// One connection, until the phone closes it (`Ok`) or it fails.
async fn session(
    app: &AppHandle,
    device: &RemoteDevice,
    mut commands: mpsc::UnboundedReceiver<RemoteCommand>,
    on_connected: impl FnOnce(),
) -> Result<(), Problem> {
    // Every address at once (a computer with a VPN announces addresses the
    // phone cannot reach): the first one that answers wins.
    let attempts = device.addresses.iter().map(|address| {
        let url = format!("ws://{}", SocketAddr::new(*address, device.port));
        Box::pin(async move {
            match tokio::time::timeout(CONNECT_TIMEOUT, tokio_tungstenite::connect_async(url)).await {
                Ok(Ok((stream, _))) => Ok(stream),
                Ok(Err(error)) => {
                    log::info!("remote: {address} did not answer: {error}");
                    Err(())
                }
                Err(_) => {
                    log::info!("remote: {address} did not answer in time");
                    Err(())
                }
            }
        })
    });
    let attempts: Vec<_> = attempts.collect();
    if attempts.is_empty() {
        return Err(Problem::Unreachable);
    }
    let (socket, _) = futures_util::future::select_ok(attempts).await.map_err(|()| Problem::Unreachable)?;
    let (mut sink, mut stream) = socket.split();

    let mut next = async || -> Result<ServerMessage, Problem> {
        loop {
            match tokio::time::timeout(ANSWER_TIMEOUT, stream.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(message) = serde_json::from_str(&text) {
                        return Ok(message);
                    }
                }
                Ok(Some(Ok(Message::Close(_)) | Err(_)) | None) | Err(_) => return Err(Problem::Lost),
                Ok(Some(Ok(_))) => {}
            }
        }
    };
    let ServerMessage::Challenge { version, nonce, .. } = next().await? else {
        return Err(Problem::Lost);
    };
    if version != PROTOCOL_VERSION {
        return Err(Problem::Version);
    }
    let library = app.state::<Arc<AppState>>().active().ok_or(Problem::SignedOut)?;
    let phone_nonce = new_nonce();
    let hello = ClientMessage::Hello {
        version: PROTOCOL_VERSION,
        device: DEVICE_NAME.to_string(),
        proof: library.client.remote_proof(&nonce, &phone_nonce),
        nonce: phone_nonce,
    };
    send(&mut sink, &hello).await?;
    match next().await? {
        ServerMessage::Welcome => on_connected(),
        ServerMessage::Denied { reason } => return Err(reason.into()),
        _ => return Err(Problem::Lost),
    }

    loop {
        tokio::select! {
            message = stream.next() => match message {
                Some(Ok(Message::Text(text))) => match serde_json::from_str::<ServerMessage>(&text) {
                    Ok(ServerMessage::State { state }) => { let _ = app.emit(STATE_EVENT, state); }
                    Ok(ServerMessage::Position { position }) => { let _ = app.emit(POSITION_EVENT, position); }
                    Ok(ServerMessage::Queue { queue }) => { let _ = app.emit(QUEUE_EVENT, queue); }
                    Ok(_) | Err(_) => {}
                },
                Some(Ok(Message::Close(_)) | Err(_)) | None => return Err(Problem::Lost),
                Some(Ok(_)) => {}
            },
            command = commands.recv() => match command {
                Some(command) => send(&mut sink, &ClientMessage::Command { command }).await?,
                // The phone closed it, or connected elsewhere.
                None => {
                    let _ = sink.close().await;
                    return Ok(());
                }
            },
        }
    }
}

async fn send<S>(sink: &mut S, message: &ClientMessage) -> Result<(), Problem>
where
    S: futures_util::Sink<Message> + Unpin,
{
    let text = serde_json::to_string(message).map_err(|_| Problem::Lost)?;
    sink.send(Message::text(text)).await.map_err(|_| Problem::Lost)
}

fn is_link_local_v6(ip: &IpAddr) -> bool {
    matches!(ip, IpAddr::V6(v6) if (v6.segments()[0] & 0xffc0) == 0xfe80)
}

/// Android drops mDNS answers unless the app holds the Wi-Fi multicast lock.
fn set_multicast(app: &AppHandle, enabled: bool) {
    #[cfg(target_os = "android")]
    if let Some(player) = app.try_state::<tauri_plugin_parlo_audio::AudioPlayer<tauri::Wry>>()
        && let Err(error) = player.set_multicast(enabled)
    {
        log::warn!("remote: could not change the multicast lock: {error}");
    }
    #[cfg(not(target_os = "android"))]
    let _ = (app, enabled);
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
