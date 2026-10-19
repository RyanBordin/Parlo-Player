// Favorites data kept between visits (spec 6.2): going back draws the last
// data at once, then it is read again. A song unstarred on the page stays in
// the lists, with an empty heart, until the page is left: nothing jumps and
// one click undoes it.
import { getFavorites, type Favorites, type TrackListSong } from "#lib/api.ts";
import { describeError, type ErrorMessage } from "#lib/errors.ts";
import { formatNumber } from "#lib/format.ts";
import { m } from "#lib/paraglide/messages.js";
import { player } from "#lib/player.svelte.ts";

export const favorites = $state<{
  data: Favorites | null;
  error: ErrorMessage | null;
}>({ data: null, error: null });

/** Only the last read counts, if two overlap (opening and a sync ending). */
let latestRead = 0;

/** Whether the song is still a favorite: the Player Bar may have changed the current one. */
export function isStarredNow(song: TrackListSong): boolean {
  return player.track?.id === song.id ? player.track.starred : song.starred;
}

/**
 * Reads the favorites again. `keepUnstarred` (same visit): songs unstarred
 * on the page stay where they were.
 */
export async function refreshFavorites(keepUnstarred: boolean): Promise<void> {
  const read = ++latestRead;
  try {
    const fresh = await getFavorites();
    if (read !== latestRead) return;
    const previous = favorites.data;
    if (keepUnstarred && previous) {
      const held = new Set(previous.songs.filter((s) => !isStarredNow(s)).map((s) => s.id));
      fresh.songs = keepInPlace(previous.songs, fresh.songs, (s) => s.id, held);
      fresh.recent = keepInPlace(previous.recent, fresh.recent, (i) => (i.kind === "song" ? i.id : ""), held);
    }
    favorites.data = fresh;
    favorites.error = null;
  } catch (e) {
    if (read === latestRead) favorites.error = describeError(e);
  }
}

/** `fresh`, plus the `held` items of `previous` it lacks, back at their place. */
function keepInPlace<T>(previous: T[], fresh: T[], key: (item: T) => string, held: Set<string>): T[] {
  if (held.size === 0) return fresh;
  const present = new Set(fresh.map(key));
  const result = [...fresh];
  previous.forEach((item, index) => {
    const id = key(item);
    if (held.has(id) && !present.has(id)) result.splice(Math.min(index, result.length), 0, item);
  });
  return result;
}

/** Leaving the page: the songs unstarred there go away. */
export function dropUnstarred(): void {
  const data = favorites.data;
  if (!data) return;
  const gone = new Set(data.songs.filter((s) => !isStarredNow(s)).map((s) => s.id));
  if (gone.size === 0) return;
  data.songs = data.songs.filter((s) => !gone.has(s.id));
  data.recent = data.recent.filter((item) => item.kind !== "song" || !gone.has(item.id));
}

/**
 * "3 albums • 30 tracks • 1 artist". Kinds without favorites are left out;
 * empty when there is none at all.
 */
export function favoritesCount(albums: number, songs: number, artists: number): string {
  const part = (count: number, message: typeof m.favorites_album_count) =>
    count > 0 ? message({ count, countText: formatNumber(count) }) : null;
  return [
    part(albums, m.favorites_album_count),
    part(songs, m.favorites_song_count),
    part(artists, m.favorites_artist_count),
  ]
    .filter(Boolean)
    .join(" • ");
}

/** Sign out: the next account must not see this one's data. */
export function clearFavorites(): void {
  latestRead++;
  favorites.data = null;
  favorites.error = null;
}
