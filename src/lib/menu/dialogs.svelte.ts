// Dialogs opened by the "..." menus (spec 8.1, 8.2), one at a time, drawn
// by `MenuDialogs.svelte`.
import type { Duplicates, ItemTarget, PlaylistSummary } from "#lib/api.ts";

export type MenuDialog =
  /** "+ New playlist": created with the item's songs (none from the sidebar). */
  | { kind: "newPlaylist"; target: ItemTarget | null }
  /** "Rename / edit description". */
  | { kind: "editPlaylist"; playlist: PlaylistSummary }
  | { kind: "deletePlaylist"; playlist: PlaylistSummary }
  /** Some songs are already in the playlist: add anyway or skip. */
  | { kind: "duplicates"; count: number; total: number; playlist: string; resolve: (choice: Duplicates | null) => void }
  | { kind: "info"; target: { kind: "song" | "album" | "artist"; id: string } };

export const menuDialog = $state<{ current: MenuDialog | null }>({ current: null });

export function showDialog(dialog: MenuDialog): void {
  menuDialog.current = dialog;
}

export function closeDialog(): void {
  menuDialog.current = null;
}

/** Asks what to do with the songs already in the playlist; `null` if cancelled. */
export function askDuplicates(count: number, total: number, playlist: string): Promise<Duplicates | null> {
  return new Promise((resolve) => showDialog({ kind: "duplicates", count, total, playlist, resolve }));
}
