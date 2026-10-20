// App preferences (spec 6.10), kept by the backend in settings.json. Read
// once before the first screen. A change applies at once (no Save button)
// and is saved in the background, one save after the other, so quick
// changes reach the file in order.
import { getAppSettings, saveAppSettings, type AppSettings } from "#lib/api.ts";
import { describeError } from "#lib/errors.ts";
import { showToast } from "#lib/toasts.svelte.ts";

export const DEFAULT_SETTINGS: AppSettings = {
  language: "system",
  homeSections: [],
  sidebarItems: [],
  density: "comfortable",
  scale: 100,
  reduceMotion: "system",
  replayGain: "auto",
  replayGainPreamp: 0,
  avoidClipping: true,
  audioDevice: "auto",
  exclusiveAudio: false,
  mixSize: 100,
  stream: { mode: "original", format: "opus", maxBitRate: 192 },
  coverCacheMb: 500,
};

export const appSettings = $state<AppSettings>(structuredClone(DEFAULT_SETTINGS));

/** Where the language was kept before settings.json (Paraglide's own key). */
const OLD_LANGUAGE_KEY = "PARAGLIDE_LOCALE";

let saving: Promise<void> = Promise.resolve();

/** Before the first screen: the saved preferences, or the defaults. */
export async function loadAppSettings(): Promise<void> {
  try {
    Object.assign(appSettings, await getAppSettings());
  } catch (e) {
    console.error("settings: could not read the preferences", e);
  }
  await migrateLanguage();
  watchSystemMotion();
  applyToDocument();
}

/** Changes some preferences: they apply now and are saved after the last save. */
export function updateSettings(change: Partial<AppSettings>): Promise<void> {
  Object.assign(appSettings, change);
  applyToDocument();
  const snapshot = $state.snapshot(appSettings);
  saving = saving
    .then(() => saveAppSettings(snapshot))
    .then(() => {})
    .catch((e) => showToast(describeError(e).message));
  return saving;
}

/** "Restore defaults" of a sector: only these preferences go back. */
export async function restoreDefaults(keys: (keyof AppSettings)[]): Promise<void> {
  const change: Partial<AppSettings> = {};
  for (const key of keys) Object.assign(change, { [key]: structuredClone(DEFAULT_SETTINGS[key]) });
  const languageChanged = change.language !== undefined && change.language !== appSettings.language;
  await updateSettings(change);
  if (languageChanged) window.location.reload();
}

/** The interface texts only change on a reload (Paraglide). */
export async function setLanguage(language: AppSettings["language"]): Promise<void> {
  if (language === appSettings.language) return;
  await updateSettings({ language });
  window.location.reload();
}

/** A language chosen before settings.json existed is kept once, then forgotten. */
async function migrateLanguage(): Promise<void> {
  let old: string | null = null;
  try {
    old = localStorage.getItem(OLD_LANGUAGE_KEY);
    localStorage.removeItem(OLD_LANGUAGE_KEY);
  } catch {
    return;
  }
  if ((old === "pt-BR" || old === "en") && appSettings.language === "system") {
    await updateSettings({ language: old });
  }
}

// --- List density (spec 6.10): the song lines of tables and Up Next. ---

/** Height of a song line, in pixels (virtualized lists need the number). */
export function songRowHeight(): number {
  return appSettings.density === "compact" ? 44 : 56;
}

/** Width of the small cover on a song line. */
export function songCoverSize(): number {
  return appSettings.density === "compact" ? 32 : 40;
}

// --- Reduce motion (spec 6.10): "system" follows the OS setting. ---

const systemMotion = $state({ reduce: false });

function watchSystemMotion(): void {
  const query = window.matchMedia("(prefers-reduced-motion: reduce)");
  systemMotion.reduce = query.matches;
  query.addEventListener("change", (event) => {
    systemMotion.reduce = event.matches;
    applyToDocument();
  });
}

/** Whether animations are off now: for scrolls done in code (CSS reads `data-motion`). */
export function reducedMotion(): boolean {
  if (appSettings.reduceMotion === "system") return systemMotion.reduce;
  return appSettings.reduceMotion === "on";
}

/** "smooth" scrolls, unless animations are reduced. */
export function scrollBehavior(): ScrollBehavior {
  return reducedMotion() ? "instant" : "smooth";
}

/** Density and motion are attributes of <html>, read by the styles. */
function applyToDocument(): void {
  const root = document.documentElement;
  root.dataset.density = appSettings.density;
  root.dataset.motion = reducedMotion() ? "reduce" : "full";
}
