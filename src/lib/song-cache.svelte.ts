// Lyrics and Related of the last songs played: switching tabs, or going back
// to a song, does not ask the server again.
import type { Lyrics, Related } from "#lib/api.ts";

/** Songs kept per kind. */
const LIMIT = 10;

export const lyricsBySong = $state<Record<string, Lyrics>>({});
/** `null`: the song is not in the library. */
export const relatedBySong = $state<Record<string, Related | null>>({});

const lyricsOrder: string[] = [];
const relatedOrder: string[] = [];

function keep<T>(record: Record<string, T>, order: string[], songId: string, value: T): void {
  record[songId] = value;
  const at = order.indexOf(songId);
  if (at !== -1) order.splice(at, 1);
  order.push(songId);
  while (order.length > LIMIT) delete record[order.shift()!];
}

export function keepLyrics(songId: string, lyrics: Lyrics): void {
  keep(lyricsBySong, lyricsOrder, songId, lyrics);
}

export function keepRelated(songId: string, related: Related | null): void {
  keep(relatedBySong, relatedOrder, songId, related);
}

/** Sign out: the next account must not see this one's data. */
export function clearSongCache(): void {
  for (const id of lyricsOrder.splice(0)) delete lyricsBySong[id];
  for (const id of relatedOrder.splice(0)) delete relatedBySong[id];
}
