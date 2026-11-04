// Multiple selection on the phone (spec 4.5): a long press on a song starts
// it, and the bar at the top shows how many are selected with their actions.
// The list being selected hands the bar what it needs.

export type TouchSelection = {
  count: number;
  /** IDs of the selected songs, in the list's order. */
  ids: () => string[];
  /** The selection's menu, built by the page (spec 8.1). */
  openMenu: (event: MouseEvent) => void;
  /** Ends the selection (X, Back). */
  close: () => void;
};

export const touchSelection = $state<{ current: TouchSelection | null }>({ current: null });
