// Runs before the app starts.
import { defineCustomClientStrategy, type Locale } from "#lib/paraglide/runtime.js";

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

defineCustomClientStrategy("custom-system", {
  getLocale: systemLocale,
  // The system language is read-only: a manual choice goes to localStorage.
  setLocale: () => {},
});
