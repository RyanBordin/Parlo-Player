// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import { loadAppSettings } from "#lib/settings/app-settings.svelte.ts";

export const ssr = false;

// The preferences (language, density...) before anything is drawn.
let loaded: Promise<void> | null = null;

export function load(): Promise<void> {
  loaded ??= loadAppSettings();
  return loaded;
}
