// Artist page (spec 6.7) and its "See all": both read the local part and the
// server part into the page cache, and show the songs in the same order.
import {
  getArtistAbout,
  getArtistPage,
  setArtistStarred,
  type ArtistAbout,
  type ArtistPage,
  type TrackListSong,
} from "#lib/api.ts";
import { artistsView } from "#lib/artists.svelte.ts";
import { keepArtist, keepArtistAbout } from "#lib/page-cache.svelte.ts";

/** Reads the page from the local cache; the last copy stays on screen meanwhile. */
export async function loadArtistPage(id: string): Promise<void> {
  keepArtist(id, await getArtistPage(id));
}

/** Biography, similar artists and photo: from the server, kept 7 days by the backend. */
export async function loadArtistAbout(id: string): Promise<void> {
  keepArtistAbout(id, await getArtistAbout(id));
}

/**
 * The artist's songs in the order shown: most played first, or the
 * `getTopSongs` order while the artist has no plays (spec 6.7).
 */
export function artistSongs(page: ArtistPage, about: ArtistAbout | null | undefined): TrackListSong[] {
  const order = about?.songOrder;
  if (!order) return page.songs;
  const byId = new Map(page.songs.map((song) => [song.id, song]));
  const listed = new Set(order);
  // A song synced after the order was read goes at the end.
  return [...order.flatMap((id) => byId.get(id) ?? []), ...page.songs.filter((song) => !listed.has(song.id))];
}

/** The heart changes at once (also in the Artists grid) and goes back if the server refuses. */
export async function toggleArtistStar(page: ArtistPage): Promise<void> {
  const artist = page.artist;
  const starred = !artist.starred;
  setStarred(artist.id, starred, page);
  try {
    await setArtistStarred(artist.id, starred);
  } catch (e) {
    setStarred(artist.id, !starred, page);
    throw e;
  }
}

function setStarred(id: string, starred: boolean, page: ArtistPage): void {
  page.artist.starred = starred;
  const item = artistsView.items?.find((artist) => artist.id === id);
  if (item) item.starred = starred;
}
