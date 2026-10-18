// Playback keyboard shortcuts (spec 12.2).
import { next, previous, seekBy, togglePause } from "#lib/api.ts";
import { player, queue } from "#lib/player.svelte.ts";

/** Seconds moved by ← and →. */
const SEEK_STEP = 5;

type Action = "togglePause" | "next" | "previous" | "back" | "forward";

/**
 * Typing never triggers a shortcut: text fields, selects and editable areas
 * keep their keys, and so do sliders (their arrows move the slider).
 */
function ownsKeys(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || target.closest("input, textarea, select") !== null;
}

/** Space on a focused button or link presses it instead. */
function pressesSpace(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && target.closest("button, a[href], [role='button'], summary") !== null;
}

function actionFor(event: KeyboardEvent): Action | null {
  if (event.altKey || event.metaKey || event.shiftKey || ownsKeys(event.target)) return null;
  switch (event.key) {
    case " ":
      return event.ctrlKey || pressesSpace(event.target) ? null : "togglePause";
    case "ArrowRight":
      return event.ctrlKey ? "next" : "forward";
    case "ArrowLeft":
      return event.ctrlKey ? "previous" : "back";
    default:
      return null;
  }
}

/** Window `keydown` handler of the app shell. */
export function handleShortcut(event: KeyboardEvent): void {
  if (event.defaultPrevented) return;
  const action = actionFor(event);
  if (!action) return;
  event.preventDefault();
  // Holding Space must not toggle over and over.
  if (event.repeat && action === "togglePause") return;

  const hasTrack = player.track !== null && player.status !== "idle";
  const run = (command: () => Promise<unknown>) => command().catch(() => {});
  switch (action) {
    case "togglePause":
      if (hasTrack) run(togglePause);
      break;
    case "next":
      if (queue.tracks.length) run(next);
      break;
    case "previous":
      if (queue.tracks.length) run(previous);
      break;
    case "forward":
      if (hasTrack) run(() => seekBy(SEEK_STEP));
      break;
    case "back":
      if (hasTrack) run(() => seekBy(-SEEK_STEP));
      break;
  }
}
