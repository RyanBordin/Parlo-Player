// Preferences of the signed-in server (spec 6.10), kept in its library
// database: read when the library opens, forgotten on sign out. Like the app
// preferences, a change applies at once and is saved in the background.
import { getLibrarySettings, saveLibrarySettings, type LibrarySettings } from "#lib/api.ts";
import { describeError } from "#lib/errors.ts";
import { showToast } from "#lib/toasts.svelte.ts";

export const DEFAULT_LIBRARY_SETTINGS: LibrarySettings = {
  scrobble: true,
  syncIntervalMinutes: 30,
  continueOtherDevice: true,
  hiddenFolders: [],
};

export const librarySettings = $state<LibrarySettings>(structuredClone(DEFAULT_LIBRARY_SETTINGS));

let saving: Promise<void> = Promise.resolve();

/** When a library opens. */
export async function loadLibrarySettings(): Promise<void> {
  try {
    Object.assign(librarySettings, await getLibrarySettings());
  } catch (e) {
    console.error("settings: could not read the server preferences", e);
  }
}

export function updateLibrarySettings(change: Partial<LibrarySettings>): Promise<void> {
  Object.assign(librarySettings, change);
  const snapshot = $state.snapshot(librarySettings);
  saving = saving
    .then(() => saveLibrarySettings(snapshot))
    .catch((e) => showToast(describeError(e).message));
  return saving;
}

/** "Restore defaults" of a sector: only these preferences go back. */
export function restoreLibraryDefaults(keys: (keyof LibrarySettings)[]): Promise<void> {
  const change: Partial<LibrarySettings> = {};
  for (const key of keys) Object.assign(change, { [key]: structuredClone(DEFAULT_LIBRARY_SETTINGS[key]) });
  return updateLibrarySettings(change);
}

/** Sign out: the next account starts from its own. */
export function clearLibrarySettings(): void {
  Object.assign(librarySettings, structuredClone(DEFAULT_LIBRARY_SETTINGS));
}
