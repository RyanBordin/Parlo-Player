// Quick search and sorting of a tracklist (spec 7), on the songs the page
// already has from the local cache.
import type { TrackListSong } from "#lib/api.ts";
import { getLocale } from "#lib/paraglide/runtime.js";

export type SortKey = "title" | "album" | "duration";
export type SortDirection = "asc" | "desc";
/** `null`: the page's own order (track number, or the playlist's order). */
export type Sort = { key: SortKey; direction: SortDirection } | null;

/** A song of the list and its place in the page's own order. */
export type Entry = { song: TrackListSong; position: number };

/** Lower case, without accents: "Beyoncé" and "beyonce" match. */
export function normalize(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

// Built once per song: every field the search looks at.
const haystacks = new WeakMap<TrackListSong, string>();

function haystack(song: TrackListSong): string {
  let text = haystacks.get(song);
  if (text === undefined) {
    const fields = [
      song.title,
      song.artist,
      song.album,
      song.albumArtist,
      song.composer,
      song.genre,
      ...song.genres,
      song.year,
      song.suffix,
      song.bitRate ? `${song.bitRate} kbps` : null,
      song.path,
    ];
    text = normalize(fields.filter((f) => f != null && f !== "").join("\n"));
    haystacks.set(song, text);
  }
  return text;
}

/** Every word typed must be somewhere in the song's metadata. */
export function filterEntries(entries: Entry[], query: string): Entry[] {
  const words = normalize(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return entries;
  return entries.filter(({ song }) => {
    const text = haystack(song);
    return words.every((word) => text.includes(word));
  });
}

/** A new sorted copy; equal values keep the page's own order. */
export function sortEntries(entries: Entry[], sort: Sort): Entry[] {
  if (!sort) return entries;
  const collator = new Intl.Collator(getLocale(), { sensitivity: "base", numeric: true });
  const sign = sort.direction === "asc" ? 1 : -1;
  const value = (song: TrackListSong) =>
    sort.key === "duration" ? (song.duration ?? 0) : ((sort.key === "title" ? song.title : song.album) ?? "");
  return [...entries].sort((a, b) => {
    const x = value(a.song);
    const y = value(b.song);
    const order = typeof x === "number" && typeof y === "number" ? x - y : collator.compare(String(x), String(y));
    return sign * order || a.position - b.position;
  });
}

/** First click sorts up, the second down (spec 7). */
export function nextSort(current: Sort, key: SortKey): Sort {
  if (current?.key === key) return { key, direction: current.direction === "asc" ? "desc" : "asc" };
  return { key, direction: "asc" };
}
