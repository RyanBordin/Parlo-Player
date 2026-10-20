// Runs before the app starts.
import { defineCustomClientStrategy, type Locale } from "#lib/paraglide/runtime.js";
import { appSettings } from "#lib/settings/app-settings.svelte.ts";

declare global {
  interface Window {
    /** OS display language, set by the backend (src-tauri/src/locale.rs). */
    __PARLO_SYSTEM_LANGUAGE__?: string;
  }
}

/**
 * Any Portuguese variant opens in pt-BR and any English one in en. Other
 * languages give `undefined`, so Paraglide falls back to baseLocale (pt-BR).
 */
function systemLocale(): Locale | undefined {
  const language = (window.__PARLO_SYSTEM_LANGUAGE__ ?? "").toLowerCase().split("-")[0];
  if (language === "pt") return "pt-BR";
  if (language === "en") return "en";
  return undefined;
}

// The language of the preferences (spec 6.10), read before the first screen
// (src/routes/+layout.ts); "system" follows the OS.
defineCustomClientStrategy("custom-settings", {
  getLocale: () => (appSettings.language === "system" ? systemLocale() : appSettings.language),
  // Changed through the preferences (setLanguage), never by Paraglide.
  setLocale: () => {},
});
