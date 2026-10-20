// Last album, playlist and artist pages opened. Going back draws them at
// once, so the scroll position can be restored; then they are read again.
import type { AlbumPage, ArtistAbout, ArtistPage, PlaylistPage } from "#lib/api.ts";

/** Pages kept; older ones are dropped. */
const LIMIT = 20;

/** By ID. `null`: known not to exist (deleted / not in the library). */
export const albumPages = $state<Record<string, AlbumPage | null>>({});
export const playlistPages = $state<Record<string, PlaylistPage | null>>({});
export const artistPages = $state<Record<string, ArtistPage | null>>({});
/** The server parts of the artist pages kept (biography, similar artists). */
export const artistAbouts = $state<Record<string, ArtistAbout | null>>({});

/** Opening order, oldest first, e.g. "album:al1". */
const order: string[] = [];

function touch(key: string): void {
  const at = order.indexOf(key);
  if (at !== -1) order.splice(at, 1);
  order.push(key);
  while (order.length > LIMIT) {
    const [kind, id] = splitKey(order.shift()!);
    if (kind === "album") delete albumPages[id];
    else if (kind === "artist") {
      delete artistPages[id];
      delete artistAbouts[id];
    } else delete playlistPages[id];
  }
}

function splitKey(key: string): [string, string] {
  const at = key.indexOf(":");
  return [key.slice(0, at), key.slice(at + 1)];
}

export function keepAlbum(id: string, page: AlbumPage | null): void {
  albumPages[id] = page;
  touch(`album:${id}`);
}

export function keepPlaylist(id: string, page: PlaylistPage | null): void {
  playlistPages[id] = page;
  touch(`playlist:${id}`);
}

export function keepArtist(id: string, page: ArtistPage | null): void {
  artistPages[id] = page;
  touch(`artist:${id}`);
}

export function keepArtistAbout(id: string, about: ArtistAbout | null): void {
  artistAbouts[id] = about;
  touch(`artist:${id}`);
}

/** Sign out: another account must not see these pages. */
export function clearPages(): void {
  order.length = 0;
  for (const pages of [albumPages, playlistPages, artistPages, artistAbouts]) {
    for (const id of Object.keys(pages)) delete pages[id];
  }
}
