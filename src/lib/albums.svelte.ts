// State of the Albums page (spec 6.4). The list stays in memory, so going
// back from an album draws the grid at once at the same scroll position.
// The search and "Sort by" are kept between sessions (this computer only);
// the "Random" order lasts until it is chosen again or the app closes.
import { listAlbums, type AlbumListItem } from "#lib/api.ts";
import { ALBUM_SORTS, DEFAULT_ALBUM_SORT, type AlbumSort } from "#lib/album-list.ts";
import { describeError, type ErrorMessage } from "#lib/errors.ts";

const STORAGE_KEY = "parlo.albums";

type Saved = { query: string; sort: AlbumSort };

function readSaved(): Saved {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    return {
      query: typeof saved.query === "string" ? saved.query : "",
      sort: ALBUM_SORTS.includes(saved.sort) ? saved.sort : DEFAULT_ALBUM_SORT,
    };
  } catch {
    return { query: "", sort: DEFAULT_ALBUM_SORT };
  }
}

export const albumsView = $state<Saved & { items: AlbumListItem[] | null; error: ErrorMessage | null; draw: number }>({
  ...readSaved(),
  items: null,
  error: null,
  /** Changes on every new "Random" draw, so the grid sorts again. */
  draw: 0,
});

/** Rank of each album in the current "Random" order. */
export const randomRank = new Map<string, number>();

function save(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ query: albumsView.query, sort: albumsView.sort }));
  } catch {
    // Storage unavailable: the choice lasts until the app closes.
  }
}

export function setAlbumQuery(query: string): void {
  albumsView.query = query;
  save();
}

export function setAlbumSort(sort: AlbumSort): void {
  albumsView.sort = sort;
  if (sort === "random") drawRandomOrder();
  save();
}

/** A new random order of every album known. */
export function drawRandomOrder(): void {
  randomRank.clear();
  for (const album of albumsView.items ?? []) randomRank.set(album.id, Math.random());
  albumsView.draw += 1;
}

/**
 * Reads the list again. With "Random" (also when the app opens with it
 * saved), albums the draw does not know get random places.
 */
export async function refreshAlbums(): Promise<void> {
  try {
    albumsView.error = null;
    const items = await listAlbums();
    for (const album of items) if (!randomRank.has(album.id)) randomRank.set(album.id, Math.random());
    albumsView.items = items;
  } catch (e) {
    albumsView.error = describeError(e);
  }
}

/** Sign out: the next account must not see this one's albums. */
export function clearAlbums(): void {
  albumsView.items = null;
  randomRank.clear();
}
