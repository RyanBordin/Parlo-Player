// Reactive sync state shared by every screen, fed by the backend events.
import { listen } from "@tauri-apps/api/event";
import {
  SYNC_FINISHED_EVENT,
  SYNC_PROGRESS_EVENT,
  syncLibrary,
  type CommandError,
  type SyncFinished,
  type SyncProgress,
} from "#lib/api.ts";
import { formatNumber } from "#lib/format.ts";
import { m } from "#lib/paraglide/messages.js";

export const syncState = $state<{
  running: boolean;
  progress: SyncProgress | null;
  error: CommandError | null;
  /** Increments after each sync, so screens can reload their data. */
  finishedCount: number;
}>({ running: false, progress: null, error: null, finishedCount: 0 });

let listening: Promise<void> | null = null;

/** Subscribes to the sync events once for the whole app. */
function listenToSyncEvents(): Promise<void> {
  listening ??= (async () => {
    await listen<SyncProgress>(SYNC_PROGRESS_EVENT, (event) => {
      syncState.running = true;
      syncState.progress = event.payload;
    });
    await listen<SyncFinished>(SYNC_FINISHED_EVENT, (event) => {
      syncState.running = false;
      syncState.progress = null;
      syncState.error = event.payload.error;
      syncState.finishedCount += 1;
    });
  })();
  return listening;
}

/**
 * Starts a sync (the backend applies the rules of spec 12.4 unless
 * `forceFull`). Listeners are ready before it starts, so no event is missed.
 */
export async function startSync(forceFull: boolean): Promise<void> {
  await listenToSyncEvents();
  syncState.error = null;
  try {
    if (await syncLibrary(forceFull)) {
      syncState.running = true;
    }
  } catch (error) {
    syncState.error = error as CommandError;
  }
}

/** Translated name of the current phase, e.g. "Downloading songs". */
export function phaseLabel(progress: SyncProgress | null): string {
  switch (progress?.phase) {
    case "artists":
      return m.sync_phase_artists();
    case "albums":
      return m.sync_phase_albums();
    case "songs":
      return m.sync_phase_songs();
    case "saving":
      return m.sync_phase_saving();
    case "starred":
      return m.sync_phase_starred();
    case "playlists":
      return m.sync_phase_playlists();
    default:
      return m.sync_starting();
  }
}

/** "1,500 of 5,120" when the total is known, "350 so far" otherwise. */
export function progressLabel(progress: SyncProgress | null): string | null {
  if (!progress || progress.done === 0) return null;
  const done = formatNumber(progress.done);
  return progress.total
    ? m.sync_progress_of({ done, total: formatNumber(progress.total) })
    : m.sync_progress_count({ done });
}
