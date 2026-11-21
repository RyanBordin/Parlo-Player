// Remote control on the network (spec 12.8): the computers found while Now
// Playing is open, and what the connected one is playing, fed by the
// backend's `remote-*` events.
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  REMOTE_DEVICES_EVENT,
  REMOTE_POSITION_EVENT,
  REMOTE_QUEUE_EVENT,
  REMOTE_STATE_EVENT,
  REMOTE_STATUS_EVENT,
  remoteCommand,
  remoteConnect,
  remoteDisconnect,
  startRemoteDiscovery,
  stopRemoteDiscovery,
  type PlayerPosition,
  type PlayerState,
  type RemoteCommand,
  type RemoteDevice,
  type RemoteQueue,
  type RemoteStatus,
} from "#lib/api.ts";

export const remote = $state<{
  /** Computers signed in with the same account, while looking. */
  devices: RemoteDevice[];
  /** The connection; `null` before the first one. */
  status: RemoteStatus | null;
  player: PlayerState | null;
  position: PlayerPosition;
  queue: RemoteQueue | null;
}>({ devices: [], status: null, player: null, position: { position: 0, duration: null }, queue: null });

let unlisten: UnlistenFn[] = [];
let looking = false;

/**
 * Back on screen after the phone slept: Android drops the connection with
 * the screen off, so it comes back by itself (to the same computer).
 */
function onVisibilityChange() {
  const status = remote.status;
  if (document.visibilityState !== "visible" || status?.status !== "closed" || status.problem !== "lost") return;
  const device = remote.devices.find((d) => d.id === status.deviceId);
  if (device) connectRemote(device).catch(() => {});
}

/** Now Playing opened: listen to the events and look for computers. */
export async function startRemote(): Promise<void> {
  if (looking) return;
  looking = true;
  document.addEventListener("visibilitychange", onVisibilityChange);
  unlisten = await Promise.all([
    listen<RemoteDevice[]>(REMOTE_DEVICES_EVENT, (event) => (remote.devices = event.payload)),
    listen<RemoteStatus>(REMOTE_STATUS_EVENT, (event) => {
      remote.status = event.payload;
      if (event.payload.status !== "connected") {
        remote.player = null;
        remote.queue = null;
        remote.position = { position: 0, duration: null };
      }
    }),
    listen<PlayerState>(REMOTE_STATE_EVENT, (event) => (remote.player = event.payload)),
    listen<PlayerPosition>(REMOTE_POSITION_EVENT, (event) => (remote.position = event.payload)),
    listen<RemoteQueue>(REMOTE_QUEUE_EVENT, (event) => (remote.queue = event.payload)),
  ]);
  if (!looking) {
    // Closed while the listeners were starting.
    stopListening();
    return;
  }
  try {
    remote.devices = await startRemoteDiscovery();
  } catch {
    // No network or not signed in: the button just does not show.
    remote.devices = [];
  }
}

/** Now Playing closed: stop looking and let the computer go. */
export function stopRemote(): void {
  if (!looking) return;
  looking = false;
  document.removeEventListener("visibilitychange", onVisibilityChange);
  stopListening();
  remote.devices = [];
  remote.status = null;
  remote.player = null;
  remote.queue = null;
  stopRemoteDiscovery().catch(() => {});
  remoteDisconnect().catch(() => {});
}

function stopListening() {
  for (const stop of unlisten) stop();
  unlisten = [];
}

export function connectRemote(device: RemoteDevice): Promise<void> {
  remote.status = { deviceId: device.id, name: device.name, status: "connecting", problem: null };
  return remoteConnect(device.id);
}

export function disconnectRemote(): Promise<void> {
  remote.status = null;
  remote.player = null;
  remote.queue = null;
  return remoteDisconnect();
}

export function sendRemote(command: RemoteCommand): Promise<void> {
  return remoteCommand(command);
}
