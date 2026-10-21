// Lines of a long song list (the Songs page, the songs found by the search):
// the page holds the IDs in the order shown, and the lines load in blocks as
// they come into view, so a list of 50 000 songs stays light (spec 12.3).
import { getSongRows, type TrackListSong } from "#lib/api.ts";

/** Lines asked together. */
const BLOCK_SIZE = 100;
/** Lines kept in memory; the oldest are dropped first. */
const ROWS_KEPT = 5000;

export class SongRowLoader {
  /** Lines already loaded, by song ID. */
  rows = $state<Record<string, TrackListSong>>({});
  /** Load order of `rows`, oldest first. */
  #order: string[] = [];
  #pending = new Set<number>();
  #loading = new Set<string>();
  #scheduled = false;
  /** Grows with every new list: blocks of an older one are not "fresh". */
  #list = 0;

  constructor(
    /** The IDs in the order shown; `null` before the first list. */
    private readonly ids: () => string[] | null,
    private readonly events: {
      /** A block arrived; `fresh` if it belongs to the list shown now. */
      loaded?: (fresh: boolean) => void;
      error?: (error: unknown) => void;
    } = {},
  ) {}

  /** A new list is shown (search, sort...): its blocks start over. */
  newList(): void {
    this.#list++;
  }

  /**
   * The line at `index` of the list shown, or `undefined` while it loads (its
   * block is then asked for). Called while drawing: the request goes out right
   * after, together with the other blocks in view.
   */
  at(index: number): TrackListSong | undefined {
    const id = this.ids()?.[index];
    if (id === undefined) return undefined;
    const row = this.rows[id];
    if (row === undefined) {
      this.#pending.add(Math.floor(index / BLOCK_SIZE));
      if (!this.#scheduled) {
        this.#scheduled = true;
        queueMicrotask(() => this.#load());
      }
    }
    return row;
  }

  #load(): void {
    this.#scheduled = false;
    const ids = this.ids();
    if (!ids) return;
    const mine = this.#list;
    for (const block of this.#pending) {
      const wanted = ids.slice(block * BLOCK_SIZE, (block + 1) * BLOCK_SIZE).filter((id) => !(id in this.rows));
      const tag = `${mine}:${block}`;
      if (wanted.length === 0 || this.#loading.has(tag)) continue;
      this.#loading.add(tag);
      getSongRows(wanted)
        .then((rows) => {
          for (const row of rows) this.#keep(row);
          this.events.loaded?.(mine === this.#list);
        })
        .catch((e) => this.events.error?.(e))
        .finally(() => this.#loading.delete(tag));
    }
    this.#pending.clear();
  }

  #keep(row: TrackListSong): void {
    if (!(row.id in this.rows)) this.#order.push(row.id);
    this.rows[row.id] = row;
    while (this.#order.length > ROWS_KEPT) delete this.rows[this.#order.shift()!];
  }

  /** The lines may have changed (a sync), or belong to another account. */
  clear(): void {
    this.#list++;
    for (const id of this.#order.splice(0)) delete this.rows[id];
  }
}
