// Global search (spec 6.9). The results page lives at /search?q=…&tab=…, so
// Back and Forward go through the searches and the tabs. The search runs in
// SQLite (db::search); the last results stay in memory, so going back draws
// them at once at the same scroll position.
import {
  addRecentSearch,
  getRecentSearches,
  removeRecentSearch,
  search,
  type SearchResults,
  type TrackListSong,
} from "#lib/api.ts";
import { describeError, type ErrorMessage } from "#lib/errors.ts";
import { SongRowLoader } from "#lib/song-rows.svelte.ts";

export const SEARCH_TABS = ["top", "songs", "albums", "artists"] as const;
export type SearchTab = (typeof SEARCH_TABS)[number];

/** The search runs once typing pauses this long. */
export const SEARCH_DELAY_MS = 150;
/** Searches whose results stay in memory, for Back. */
const KEPT = 10;

/** Address of a search and tab ("Top" is the default and is left out). */
export function searchHref(query: string, tab: SearchTab = "top"): string {
  const params = new URLSearchParams();
  if (query) params.set("q", query);
  if (tab !== "top") params.set("tab", tab);
  const text = params.toString();
  return text ? `/search?${text}` : "/search";
}

export function parseTab(value: string | null): SearchTab {
  return SEARCH_TABS.find((tab) => tab === value) ?? "top";
}

// `$state.raw`: the results are only ever replaced, and 50 000 song IDs do
// not need to be reactive one by one.
class SearchState {
  /** What `results` were found for; `null` before the first search. */
  query = $state<string | null>(null);
  results = $state.raw<SearchResults | null>(null);
  error = $state<ErrorMessage | null>(null);
  /** The last searches, newest first; `null` until read. */
  recent = $state.raw<string[] | null>(null);
}
export const searchState = new SearchState();

/** Results kept, by search; the oldest are dropped first. */
const kept = new Map<string, SearchResults>();
/** Grows with every search: answers for an older one are dropped. */
let generation = 0;

/** Lines of the "Songs" tab, loaded as they come into view. */
const songRows = new SongRowLoader(() => searchState.results?.songIds ?? null, {
  error: (e) => (searchState.error = describeError(e)),
});

/** The line at `index` of the songs found, or `undefined` while it loads. */
export function foundSongAt(index: number): TrackListSong | undefined {
  return songRows.at(index);
}

/**
 * Shows the results of `query`: at once if kept, otherwise once found (the
 * last results stay meanwhile). `fresh` drops the kept ones (after a sync).
 */
export async function runSearch(query: string, fresh = false): Promise<void> {
  const mine = ++generation;
  if (fresh) {
    kept.clear();
    songRows.clear();
  }
  const known = kept.get(query);
  if (known) {
    show(query, known);
    return;
  }
  try {
    const results = await search(query);
    kept.set(query, results);
    for (const old of kept.keys()) {
      if (kept.size <= KEPT) break;
      kept.delete(old);
    }
    if (mine === generation) show(query, results);
  } catch (e) {
    if (mine === generation) searchState.error = describeError(e);
  }
}

function show(query: string, results: SearchResults): void {
  songRows.newList();
  searchState.query = query;
  searchState.results = results;
  searchState.error = null;
}

export async function loadRecentSearches(): Promise<void> {
  try {
    searchState.recent = await getRecentSearches();
  } catch {
    // Unavailable: the list stays empty.
    searchState.recent ??= [];
  }
}

/** A search was used (Enter, or a result opened or played): first in the recent list. */
export function rememberSearch(query: string): void {
  if (!query.trim()) return;
  addRecentSearch(query)
    .then((list) => (searchState.recent = list))
    .catch(() => {});
}

export function forgetSearch(query: string): void {
  removeRecentSearch(query)
    .then((list) => (searchState.recent = list))
    .catch(() => {});
}

/** Sign out: the next account must not see this one's searches. */
export function clearSearch(): void {
  generation++;
  kept.clear();
  songRows.clear();
  searchState.query = null;
  searchState.results = null;
  searchState.error = null;
  searchState.recent = null;
}
