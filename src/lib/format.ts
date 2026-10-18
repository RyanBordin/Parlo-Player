// Numbers and dates in the user's chosen language.
import { getLocale } from "#lib/paraglide/runtime.js";

export function formatNumber(value: number): string {
  return new Intl.NumberFormat(getLocale()).format(value);
}

/** Formats Unix seconds, e.g. "Oct 18, 2026, 9:40 AM". */
export function formatDateTime(unixSeconds: number): string {
  return new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(unixSeconds * 1000),
  );
}

/** Song time, e.g. "3:07" or "1:02:45". */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return "--:--";
  const total = Math.floor(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
}
