// URLs of the app's `cover` protocol (served by src-tauri/src/covers). They
// only carry the cover ID and a size: the credentials stay in the backend.
import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * Image URL for a cover shown `size` CSS pixels wide. The size is doubled on
 * high-density screens; the backend rounds it to the sizes it stores.
 */
export function coverUrl(coverId: string, size: number): string {
  const pixels = Math.ceil(size * (window.devicePixelRatio || 1));
  // "http://cover.localhost/" on Windows, "cover://localhost/" on Linux.
  const base = convertFileSrc("", "cover");
  return `${base}?id=${encodeURIComponent(coverId)}&size=${pixels}`;
}
