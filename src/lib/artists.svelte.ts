// State of the Artists page (spec 6.6). The list stays in memory, so going
// back from an artist draws the grid at once at the same scroll position.
// The search, "Sort by" and the type filter are kept between sessions (this
// computer only).
import { listArtists, type ArtistListItem } from "#lib/api.ts";
import {
  ARTIST_SORTS,
  ARTIST_TYPES,
  DEFAULT_ARTIST_SORT,
  DEFAULT_ARTIST_TYPE,
  type ArtistSort,
  type ArtistType,
} from "#lib/artist-list.ts";
import { describeError, type ErrorMessage } from "#lib/errors.ts";

const STORAGE_KEY = "parlo.artists";

type Saved = { query: string; sort: ArtistSort; type: ArtistType };

function readSaved(): Saved {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    return {
      query: typeof saved.query === "string" ? saved.query : "",
      sort: ARTIST_SORTS.includes(saved.sort) ? saved.sort : DEFAULT_ARTIST_SORT,
      type: ARTIST_TYPES.includes(saved.type) ? saved.type : DEFAULT_ARTIST_TYPE,
    };
  } catch {
    return { query: "", sort: DEFAULT_ARTIST_SORT, type: DEFAULT_ARTIST_TYPE };
  }
}

export const artistsView = $state<Saved & { items: ArtistListItem[] | null; error: ErrorMessage | null }>({
  ...readSaved(),
  items: null,
  error: null,
});

function save(): void {
  try {
    const { query, sort, type } = artistsView;
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ query, sort, type }));
  } catch {
    // Storage unavailable: the choice lasts until the app closes.
  }
}

export function setArtistQuery(query: string): void {
  artistsView.query = query;
  save();
}

export function setArtistSort(sort: ArtistSort): void {
  artistsView.sort = sort;
  save();
}

export function setArtistType(type: ArtistType): void {
  artistsView.type = type;
  save();
}

/** Reads the list again; the last one stays on screen meanwhile. */
export async function refreshArtists(): Promise<void> {
  try {
    artistsView.error = null;
    artistsView.items = await listArtists();
  } catch (e) {
    artistsView.error = describeError(e);
  }
}

/** Sign out: the next account must not see this one's artists. */
export function clearArtists(): void {
  artistsView.items = null;
}
