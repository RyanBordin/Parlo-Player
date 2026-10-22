// Reactive player state shared by every screen, fed by the backend events.
import { listen } from "@tauri-apps/api/event";
import {
  getPlayerState,
  getQueue,
  PLAYER_ERROR_EVENT,
  PLAYER_POSITION_EVENT,
  PLAYER_STATE_EVENT,
  QUEUE_CHANGED_EVENT,
  QUEUE_POSITION_EVENT,
  type PlayerError,
  type PlayerPosition,
  type PlayerState,
  type QueuePosition,
  type QueueState,
  type Track,
} from "#lib/api.ts";

export const player = $state<{
  status: PlayerState["status"];
  track: Track | null;
  volume: number;
  muted: boolean;
  /** Seconds. */
  position: number;
  /** Seconds; from mpv once the song loads, from the library before that. */
  duration: number | null;
  error: PlayerError | null;
}>({ status: "idle", track: null, volume: 100, muted: false, position: 0, duration: null, error: null });

/** The queue as the backend has it (source of truth). */
export const queue = $state<QueueState>({
  label: "",
  continuingFrom: null,
  tracks: [],
  currentIndex: null,
  shuffle: false,
  repeat: "off",
});

/**
 * The song the Next button would play (spec 4.3, 5.1.1), in playback order
 * (shuffle included): the following one; past the last, the first with
 * "repeat queue", otherwise none. "Repeat one" does not apply to Next.
 */
export function nextTrack(state: QueueState): Track | null {
  const current = state.currentIndex;
  if (current === null || state.tracks.length === 0) return null;
  if (current + 1 < state.tracks.length) return state.tracks[current + 1];
  return state.repeat === "all" ? state.tracks[0] : null;
}

let listening: Promise<void> | null = null;

/** Subscribes to the player events once, then reads the current state. */
export function listenToPlayer(): Promise<void> {
  listening ??= (async () => {
    await listen<PlayerState>(PLAYER_STATE_EVENT, (event) => applyState(event.payload));
    await listen<PlayerPosition>(PLAYER_POSITION_EVENT, (event) => {
      player.position = event.payload.position;
      player.duration = event.payload.duration ?? player.duration;
    });
    await listen<PlayerError>(PLAYER_ERROR_EVENT, (event) => {
      player.error = event.payload;
    });
    await listen<QueueState>(QUEUE_CHANGED_EVENT, (event) => Object.assign(queue, event.payload));
    // The same songs: only the current one or the modes changed.
    await listen<QueuePosition>(QUEUE_POSITION_EVENT, (event) => Object.assign(queue, event.payload));
    try {
      applyState(await getPlayerState());
      Object.assign(queue, await getQueue());
    } catch {
      // Player unavailable: commands will report the error when used.
    }
  })();
  return listening;
}

function applyState(state: PlayerState) {
  const newTrack = state.track?.id !== player.track?.id;
  player.status = state.status;
  player.volume = state.volume;
  player.muted = state.muted;
  player.track = state.track;
  if (newTrack) {
    player.position = 0;
    player.duration = state.track?.duration ?? null;
  }
  if (state.status === "loading") player.error = null;
}
