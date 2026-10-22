// "Save to computer" (spec 8.2): the backend downloads; here are its
// progress ("Saving 3 of 12") and its end, for `SaveProgress.svelte`.
import { listen } from "@tauri-apps/api/event";
import {
  SAVE_FINISHED_EVENT,
  SAVE_PROGRESS_EVENT,
  saveToComputer,
  type SaveFinished,
  type SaveProgress,
  type SaveTarget,
} from "#lib/api.ts";

/** An end without failures goes away by itself after this long. */
const RESULT_MS = 8000;

export const saves = $state<{ progress: SaveProgress | null; finished: SaveFinished | null }>({
  progress: null,
  finished: null,
});

let listening: Promise<void> | null = null;
let hideTimer: ReturnType<typeof setTimeout> | undefined;

/** Subscribes to the save events once for the whole app. */
export function listenToSaves(): Promise<void> {
  listening ??= (async () => {
    await listen<SaveProgress>(SAVE_PROGRESS_EVENT, (event) => {
      clearTimeout(hideTimer);
      saves.finished = null;
      saves.progress = event.payload;
    });
    await listen<SaveFinished>(SAVE_FINISHED_EVENT, (event) => {
      const finished = event.payload;
      saves.progress = null;
      saves.finished = finished;
      clearTimeout(hideTimer);
      if (finished.failed.length === 0 && !finished.forbidden) hideTimer = setTimeout(dismissSaveResult, RESULT_MS);
    });
  })();
  return listening;
}

export function dismissSaveResult(): void {
  clearTimeout(hideTimer);
  saves.finished = null;
}

/** Asks where and starts; the listeners are ready first, so no event is missed. */
export async function saveItem(target: SaveTarget): Promise<void> {
  await listenToSaves();
  await saveToComputer(target);
}
