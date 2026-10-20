// Sidebar menu: these 7 items, in this default order (spec 4.2). The user
// can reorder and hide them, except Settings: always last and shown, so the
// way back to the settings is never lost (spec 6.10).
import Disc3 from "@lucide/svelte/icons/disc-3";
import Download from "@lucide/svelte/icons/download";
import Heart from "@lucide/svelte/icons/heart";
import House from "@lucide/svelte/icons/house";
import MicVocal from "@lucide/svelte/icons/mic-vocal";
import Music from "@lucide/svelte/icons/music";
import Settings from "@lucide/svelte/icons/settings";
import { m } from "#lib/paraglide/messages.js";
import { appSettings } from "#lib/settings/app-settings.svelte.ts";
import { arrange } from "#lib/settings/layout.ts";

export type NavItem = {
  /** Kept in the preferences. */
  id: string;
  href: string;
  label: () => string;
  icon: typeof House;
  /** Not shown in this version. */
  hidden?: boolean;
};

export const NAV_ITEMS: NavItem[] = [
  { id: "home", href: "/", label: m.nav_home, icon: House },
  { id: "favorites", href: "/favorites", label: m.nav_favorites, icon: Heart },
  { id: "songs", href: "/songs", label: m.nav_songs, icon: Music },
  { id: "albums", href: "/albums", label: m.nav_albums, icon: Disc3 },
  { id: "artists", href: "/artists", label: m.nav_artists, icon: MicVocal },
  // Downloads come in v0.2 (spec 6.8, 10).
  { id: "downloads", href: "/downloads", label: m.nav_downloads, icon: Download, hidden: true },
];

/** Always last and shown (spec 6.10). */
export const SETTINGS_ITEM: NavItem = { id: "settings", href: "/settings", label: m.nav_settings, icon: Settings };

/** The items the user can arrange, in their order, each shown or not. */
export function sidebarLayout() {
  return arrange(
    NAV_ITEMS.filter((item) => !item.hidden),
    appSettings.sidebarItems,
  );
}

/** What the sidebar shows, in order. */
export function sidebarItems(): NavItem[] {
  return [...sidebarLayout().filter((item) => item.visible), SETTINGS_ITEM];
}

/** Home matches itself and its "See more" pages; the others, their sub-pages. */
export function isActive(item: NavItem, pathname: string): boolean {
  if (item.href === "/") return pathname === "/" || pathname.startsWith("/home/");
  return pathname === item.href || pathname.startsWith(`${item.href}/`);
}
