// Columns of a tracklist table (spec 6.3, 6.5): what each one shows, its
// width and the key it sorts by.
import { m } from "#lib/paraglide/messages.js";

export type ColumnId =
  | "number"
  | "title"
  | "artist"
  | "album"
  | "year"
  | "genre"
  | "bitRate"
  | "playCount"
  | "duration"
  | "favorite";

/** Optional columns of the Songs page, in the order they appear (spec 6.3). */
export const OPTIONAL_COLUMNS = ["year", "genre", "bitRate", "playCount"] as const satisfies readonly ColumnId[];
export type OptionalColumn = (typeof OPTIONAL_COLUMNS)[number];

/** A line: a disc separator, or the song at `index` of the list shown. */
export type TableRow = { kind: "disc"; disc: number } | { kind: "song"; index: number };

/** A sorted column of the table; `null` is the page's own order. */
export type TableSort = { key: string; direction: "asc" | "desc" } | null;

type Column = {
  /** CSS grid track. */
  width: string;
  /** Header text; `null` for an icon or nothing. */
  label: (() => string) | null;
  /** Numbers line up on the right. */
  numeric?: boolean;
};

export const COLUMNS: Record<ColumnId, Column> = {
  number: { width: "48px", label: null },
  title: { width: "minmax(0, 3fr)", label: m.tracklist_title },
  artist: { width: "minmax(0, 2fr)", label: m.tracklist_artist },
  album: { width: "minmax(0, 2fr)", label: m.tracklist_album },
  year: { width: "56px", label: m.tracklist_year, numeric: true },
  genre: { width: "minmax(0, 1fr)", label: m.tracklist_genre },
  bitRate: { width: "88px", label: m.tracklist_bit_rate, numeric: true },
  playCount: { width: "104px", label: m.tracklist_play_count, numeric: true },
  duration: { width: "72px", label: null, numeric: true },
  favorite: { width: "48px", label: null },
};

export function gridColumns(columns: ColumnId[]): string {
  return columns.map((id) => COLUMNS[id].width).join(" ");
}

/** Labels of the optional columns, for the column picker. */
export const OPTIONAL_COLUMN_LABELS: Record<OptionalColumn, () => string> = {
  year: m.tracklist_year,
  genre: m.tracklist_genre,
  bitRate: m.tracklist_bit_rate,
  playCount: m.tracklist_play_count,
};
