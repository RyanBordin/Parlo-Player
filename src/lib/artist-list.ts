// Quick search, type filter and "Sort by" of the Artists page (spec 6.6),
// on the artists the page already has from the local cache.
import type { ArtistListItem } from "#lib/api.ts";
import { getLocale } from "#lib/paraglide/runtime.js";
import { normalize } from "#lib/tracklist.ts";

/** "Sort by" options, in the order of the menu (spec 6.6). */
export const ARTIST_SORTS = ["name", "mostPlayed", "added", "albumCount"] as const;
export type ArtistSort = (typeof ARTIST_SORTS)[number];
export const DEFAULT_ARTIST_SORT: ArtistSort = "name";

/** Album artists only (default), or also those featured on songs. */
export const ARTIST_TYPES = ["albumArtists", "all"] as const;
export type ArtistType = (typeof ARTIST_TYPES)[number];
export const DEFAULT_ARTIST_TYPE: ArtistType = "albumArtists";

// Built once per artist: every field the search looks at.
const haystacks = new WeakMap<ArtistListItem, string>();

function haystack(artist: ArtistListItem): string {
  let text = haystacks.get(artist);
  if (text === undefined) {
    text = normalize([artist.name, artist.sortName].filter(Boolean).join("\n"));
    haystacks.set(artist, text);
  }
  return text;
}

/** The artists of the type chosen whose name has every word typed. */
export function filterArtists(artists: ArtistListItem[], query: string, type: ArtistType): ArtistListItem[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  return artists.filter((artist) => {
    if (type === "albumArtists" && !artist.albumArtist) return false;
    if (words.length === 0) return true;
    const text = haystack(artist);
    return words.every((word) => text.includes(word));
  });
}

/** Milliseconds, or -Infinity without a readable date (goes last). */
function time(iso: string | null): number {
  const value = iso ? Date.parse(iso) : NaN;
  return Number.isNaN(value) ? -Infinity : value;
}

/** A new sorted copy; ties go by name. */
export function sortArtists(artists: ArtistListItem[], sort: ArtistSort): ArtistListItem[] {
  const collator = new Intl.Collator(getLocale(), { sensitivity: "base", numeric: true });
  const byName = (a: ArtistListItem, b: ArtistListItem) => collator.compare(a.sortName ?? a.name, b.sortName ?? b.name);
  const newest = (x: number, y: number) => (y === x ? 0 : y > x ? 1 : -1);

  const compare: (a: ArtistListItem, b: ArtistListItem) => number = {
    name: byName,
    mostPlayed: (a: ArtistListItem, b: ArtistListItem) => b.playCount - a.playCount || byName(a, b),
    added: (a: ArtistListItem, b: ArtistListItem) => newest(time(a.added), time(b.added)) || byName(a, b),
    albumCount: (a: ArtistListItem, b: ArtistListItem) => b.albumCount - a.albumCount || byName(a, b),
  }[sort];
  return [...artists].sort(compare);
}
