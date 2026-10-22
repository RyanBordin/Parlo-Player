// The "..." menus (spec 8): one menu open at a time, opened by the "..."
// button or a right click with the same content. `Menu.svelte` draws it.
import type { Component } from "svelte";
import { describeError } from "#lib/errors.ts";
import { showToast } from "#lib/toasts.svelte.ts";

/** A Lucide icon. */
export type MenuIcon = Component<{ size?: number }>;

export type MenuAction = {
  kind: "action";
  label: string;
  icon?: MenuIcon;
  /** Shown in a secondary tone (spec 8). */
  shortcut?: string;
  /** Kept when the submenu's search filters (e.g. "+ New playlist"). */
  pinned?: boolean;
  run: () => unknown;
};

export type MenuSubmenu = {
  kind: "submenu";
  label: string;
  icon?: MenuIcon;
  groups: MenuGroup[];
  /** A search field on top, e.g. more than 10 playlists (spec 8.2). */
  search?: { placeholder: string; empty: string };
};

export type MenuEntry = MenuAction | MenuSubmenu;
/** Items between two separator lines. Empty groups are not drawn. */
export type MenuGroup = MenuEntry[];

export type MenuSpec = {
  /** e.g. "3 songs selected" (spec 8.1). */
  title?: string;
  groups: MenuGroup[];
  search?: MenuSubmenu["search"];
};

/** Where a panel goes, and where it goes instead when it does not fit. */
export type Placement = { left: number; top: number; flipLeft: number; flipTop: number };

export const menuState = $state<{
  spec: MenuSpec | null;
  placement: Placement;
  /** The "..." button that opened it, which gets the focus back. */
  trigger: HTMLElement | null;
  /** Opened from the keyboard: the first item takes the focus. */
  keyboard: boolean;
}>({ spec: null, placement: { left: 0, top: 0, flipLeft: 0, flipTop: 0 }, trigger: null, keyboard: false });

/** Only the last menu asked for opens, if building one takes a moment. */
let latest = 0;

/**
 * Opens a menu at the "..." button (click) or at the mouse (right click).
 * `build` may read the local cache first (e.g. whether the item is a
 * favorite). A second click on the same button closes it.
 */
export function openMenu(event: MouseEvent, build: () => MenuSpec | Promise<MenuSpec>): void {
  event.preventDefault();
  event.stopPropagation();
  const trigger = event.type === "contextmenu" ? null : (event.currentTarget as HTMLElement | null);
  if (trigger && menuState.spec && menuState.trigger === trigger) {
    closeMenu();
    return;
  }

  let placement: Placement;
  if (trigger) {
    const rect = trigger.getBoundingClientRect();
    placement = { left: rect.left, top: rect.bottom + 4, flipLeft: rect.right, flipTop: rect.top - 4 };
  } else {
    placement = { left: event.clientX, top: event.clientY, flipLeft: event.clientX, flipTop: event.clientY };
  }
  // A click from the keyboard (Enter / Space on the button) has no pointer.
  const keyboard = event.type === "click" && event.detail === 0;

  const request = ++latest;
  Promise.resolve()
    .then(build)
    .then((spec) => {
      if (request !== latest) return;
      menuState.spec = spec;
      menuState.placement = placement;
      menuState.trigger = trigger;
      menuState.keyboard = keyboard;
    })
    .catch((e) => showToast(describeError(e).message));
}

export function closeMenu(restoreFocus = false): void {
  latest++;
  const trigger = menuState.trigger;
  menuState.spec = null;
  menuState.trigger = null;
  if (restoreFocus) trigger?.focus();
}

/** Runs an item: the menu closes first; a failure shows as a toast. */
export async function runAction(action: MenuAction): Promise<void> {
  closeMenu();
  try {
    await action.run();
  } catch (e) {
    showToast(describeError(e).message);
  }
}
