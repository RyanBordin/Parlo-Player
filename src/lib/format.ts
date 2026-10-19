// Numbers and dates in the user's chosen language.
import { m } from "#lib/paraglide/messages.js";
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

/** A date from the API (ISO 8601), e.g. "Jul 21, 2026". Empty if unreadable. */
export function formatDate(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "" : new Intl.DateTimeFormat(getLocale(), { dateStyle: "medium" }).format(date);
}

/** "12 songs", with the right plural. */
export function formatSongCount(count: number): string {
  return count === 0 ? m.collection_song_count_zero() : m.collection_song_count({ count, countText: formatNumber(count) });
}

/** Length of an album or playlist, e.g. "48 min" or "2 h 10 min". */
export function formatTotalDuration(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return m.duration_minutes({ minutes });
  return m.duration_hours_minutes({ hours: Math.floor(minutes / 60), minutes: minutes % 60 });
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
