// State of the Songs page (spec 6.3). Search and sorting run in SQLite: the
// page holds the IDs in the order shown and loads the lines in blocks as they
// come into view (SongRowLoader), so a library of 50 000 songs stays light
// (spec 12.3). The search, the sort and the optional columns are kept between
// sessions (this computer only); a random order lasts until it is drawn again
// or the app closes, as on the Albums page. The list stays in memory, so going
// back to the page draws it at once at the same scroll position.
import { listSongIds, type SongSort, type SongSortKey, type TrackListSong } from "#lib/api.ts";
import { describeError, type ErrorMessage } from "#lib/errors.ts";
import { SongRowLoader } from "#lib/song-rows.svelte.ts";
import { OPTIONAL_COLUMNS, type OptionalColumn } from "#lib/track-columns.ts";

const STORAGE_KEY = "parlo.songs";

const SORT_KEYS: SongSortKey[] = [
  "added",
  "title",
  "artist",
  "album",
  "duration",
  "year",
  "genre",
  "bitRate",
  "playCount",
  "played",
  "random",
];
export const DEFAULT_SONG_SORT: SongSort = { key: "added", direction: "desc" };

/** "Sort by" options, the same as the Albums page (spec 6.4), in its order. */
export const SONG_PRESETS = ["title", "artist", "year", "added", "mostPlayed", "recentlyPlayed", "random"] as const;
export type SongPreset = (typeof SONG_PRESETS)[number];

const PRESET_SORTS: Record<SongPreset, SongSort> = {
  title: { key: "title", direction: "asc" },
  artist: { key: "artist", direction: "asc" },
  year: { key: "year", direction: "desc" },
  added: DEFAULT_SONG_SORT,
  mostPlayed: { key: "playCount", direction: "desc" },
  recentlyPlayed: { key: "played", direction: "desc" },
  random: { key: "random", direction: "asc" },
};

/** The "Sort by" option of a sort; `null` if it came from a column header. */
export function presetOf(sort: SongSort): SongPreset | null {
  return SONG_PRESETS.find((p) => PRESET_SORTS[p].key === sort.key && PRESET_SORTS[p].direction === sort.direction) ?? null;
}

/** The draw of the random order: new each time the app opens. */
let seed = drawSeed();

function drawSeed(): number {
  return Math.floor(Math.random() * 2 ** 32);
}

type Saved = { query: string; sort: SongSort; columns: OptionalColumn[] };

function readSaved(): Saved {
  const fallback: Saved = { query: "", sort: DEFAULT_SONG_SORT, columns: [] };
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
    const sort = saved.sort;
    return {
      query: typeof saved.query === "string" ? saved.query : "",
      sort:
        sort && SORT_KEYS.includes(sort.key) && (sort.direction === "asc" || sort.direction === "desc")
          ? { key: sort.key, direction: sort.direction }
          : DEFAULT_SONG_SORT,
      columns: Array.isArray(saved.columns) ? OPTIONAL_COLUMNS.filter((c) => saved.columns.includes(c)) : [],
    };
  } catch {
    return fallback;
  }
}

export const songsView = $state<Saved & { total: number; error: ErrorMessage | null }>({
  ...readSaved(),
  total: 0,
  error: null,
});

// `$state.raw`: the list is only ever replaced, and 50 000 IDs do not need
// to be reactive one by one.
class SongList {
  /** IDs in the order shown; `null` before the first answer. */
  ids = $state.raw<string[] | null>(null);
}
export const songList = new SongList();

/** Grows with every new list: answers for an older one are dropped. */
let generation = 0;

export type ListChange = "open" | "search" | "sort";
type Timing = { queryMs: number; shownMs: number | null };

/**
 * The last opening, search and sort (shown by the meter of the 50 000 songs
 * test): milliseconds until the IDs arrived, and until the first lines were
 * drawn.
 */
export const songsTiming = $state<Record<ListChange, Timing | null>>({ open: null, search: null, sort: null });
/** The change being timed, until its first lines are drawn. */
let timing: { kind: ListChange; start: number } | null = null;

const rows = new SongRowLoader(() => songList.ids, {
  loaded(fresh) {
    if (!fresh || !timing) return;
    // The lines are drawn by the next frame.
    const { kind, start } = timing;
    timing = null;
    requestAnimationFrame(() => {
      const last = songsTiming[kind];
      if (last) last.shownMs = Math.round(performance.now() - start);
    });
  },
  error: (e) => (songsView.error = describeError(e)),
});

function save(): void {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ query: songsView.query, sort: songsView.sort, columns: songsView.columns }),
    );
  } catch {
    // Storage unavailable: the choice lasts until the app closes.
  }
}

/**
 * Reads the list again (search, sort, a sync, or opening the page). `start`
 * is when the change began, for the timings.
 */
export async function refreshSongs(kind: ListChange = "open", start = performance.now()): Promise<void> {
  const mine = ++generation;
  try {
    const list = await listSongIds(songsView.query, { ...songsView.sort, seed });
    if (mine !== generation) return;
    songsView.error = null;
    rows.newList();
    songList.ids = list.ids;
    songsView.total = list.total;
    songsTiming[kind] = { queryMs: Math.round(performance.now() - start), shownMs: null };
    timing = { kind, start };
  } catch (e) {
    if (mine === generation) songsView.error = describeError(e);
  }
}

export function setSongQuery(query: string): void {
  songsView.query = query;
  save();
}

function setSort(sort: SongSort, start: number): void {
  songsView.sort = sort;
  save();
  refreshSongs("sort", start);
}

/** "Sort by": choosing "Random" (again) draws a new order. */
export function setSongPreset(preset: SongPreset): void {
  const start = performance.now();
  if (preset === "random") seed = drawSeed();
  setSort(PRESET_SORTS[preset], start);
}

/** "Draw again" of the random order. */
export function drawSongOrder(): void {
  setSongPreset("random");
}

/** First click sorts up (newest first for "#"), the second down (spec 7). */
export function sortSongsBy(key: SongSortKey): void {
  const start = performance.now();
  const current = songsView.sort;
  if (current.key === key) {
    setSort({ key, direction: current.direction === "asc" ? "desc" : "asc" }, start);
  } else {
    setSort({ key, direction: key === "added" ? "desc" : "asc" }, start);
  }
}

export function setSongColumns(columns: OptionalColumn[]): void {
  songsView.columns = OPTIONAL_COLUMNS.filter((c) => columns.includes(c));
  save();
}

/** The line at `index` of the list shown, or `undefined` while it loads. */
export function songAt(index: number): TrackListSong | undefined {
  return rows.at(index);
}

/** After a sync: the lines may have changed. */
export function reloadSongs(): Promise<void> {
  rows.clear();
  return refreshSongs("open");
}

/** Sign out: the next account must not see this one's songs. */
export function clearSongs(): void {
  generation++;
  songList.ids = null;
  songsView.total = 0;
  rows.clear();
}
