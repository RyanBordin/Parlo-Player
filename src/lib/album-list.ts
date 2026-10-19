// Quick search and "Sort by" of the Albums page (spec 6.4), on the albums
// the page already has from the local cache.
import type { AlbumListItem } from "#lib/api.ts";
import { getLocale } from "#lib/paraglide/runtime.js";
import { normalize } from "#lib/tracklist.ts";

/** "Sort by" options, in the order of the menu (spec 6.4). */
export const ALBUM_SORTS = ["name", "artist", "year", "added", "mostPlayed", "recentlyPlayed", "random"] as const;
export type AlbumSort = (typeof ALBUM_SORTS)[number];
export const DEFAULT_ALBUM_SORT: AlbumSort = "added";

// Built once per album: every field the search looks at.
const haystacks = new WeakMap<AlbumListItem, string>();

function haystack(album: AlbumListItem): string {
  let text = haystacks.get(album);
  if (text === undefined) {
    const fields = [album.name, album.sortName, album.artist, album.year, album.genre, ...album.genres];
    text = normalize(fields.filter((f) => f != null && f !== "").join("\n"));
    haystacks.set(album, text);
  }
  return text;
}

/** Every word typed must be in the album's name, artist, year or genres. */
export function filterAlbums(albums: AlbumListItem[], query: string): AlbumListItem[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return albums;
  return albums.filter((album) => {
    const text = haystack(album);
    return words.every((word) => text.includes(word));
  });
}

/** Milliseconds, or -Infinity without a readable date (goes last when newest first). */
function time(iso: string | null): number {
  const value = iso ? Date.parse(iso) : NaN;
  return Number.isNaN(value) ? -Infinity : value;
}

/**
 * A new sorted copy. `randomRank` is the order drawn for "Random" (lower
 * first); albums it does not know go last.
 */
export function sortAlbums(albums: AlbumListItem[], sort: AlbumSort, randomRank: Map<string, number>): AlbumListItem[] {
  const collator = new Intl.Collator(getLocale(), { sensitivity: "base", numeric: true });
  const byName = (a: AlbumListItem, b: AlbumListItem) => collator.compare(a.sortName ?? a.name, b.sortName ?? b.name);
  const newest = (x: number, y: number) => (y === x ? 0 : y > x ? 1 : -1);

  const compare: (a: AlbumListItem, b: AlbumListItem) => number = {
    name: byName,
    artist: (a: AlbumListItem, b: AlbumListItem) =>
      collator.compare(a.artist ?? "", b.artist ?? "") || (a.year ?? 0) - (b.year ?? 0) || byName(a, b),
    year: (a: AlbumListItem, b: AlbumListItem) => (b.year ?? -Infinity) - (a.year ?? -Infinity) || byName(a, b),
    added: (a: AlbumListItem, b: AlbumListItem) => newest(time(a.created), time(b.created)) || byName(a, b),
    mostPlayed: (a: AlbumListItem, b: AlbumListItem) =>
      (b.playCount ?? 0) - (a.playCount ?? 0) || newest(time(a.played), time(b.played)) || byName(a, b),
    recentlyPlayed: (a: AlbumListItem, b: AlbumListItem) => newest(time(a.played), time(b.played)) || byName(a, b),
    random: (a: AlbumListItem, b: AlbumListItem) =>
      (randomRank.get(a.id) ?? Infinity) - (randomRank.get(b.id) ?? Infinity) || byName(a, b),
  }[sort];
  // `Infinity - Infinity` is NaN: treat as equal.
  return [...albums].sort((a, b) => compare(a, b) || 0);
}
