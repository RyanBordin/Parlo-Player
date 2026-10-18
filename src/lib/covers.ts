// URLs of the app's `cover` protocol (served by src-tauri/src/covers). They
// only carry the cover ID and a size: the credentials stay in the backend.
import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * Image URL for a cover shown `size` CSS pixels wide. The size is doubled on
 * high-density screens; the backend rounds it to the sizes it stores.
 * `fallback` (artists) is shown when the server has no real image for `coverId`.
 */
export function coverUrl(coverId: string, size: number, fallback?: string | null): string {
  const pixels = Math.ceil(size * (window.devicePixelRatio || 1));
  // "http://cover.localhost/" on Windows, "cover://localhost/" on Linux.
  const base = convertFileSrc("", "cover");
  const extra = fallback ? `&fallback=${encodeURIComponent(fallback)}` : "";
  return `${base}?id=${encodeURIComponent(coverId)}&size=${pixels}${extra}`;
}
