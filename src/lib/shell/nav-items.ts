// Sidebar menu: exactly these 7 items, in this order (spec 4.2).
import Disc3 from "@lucide/svelte/icons/disc-3";
import Download from "@lucide/svelte/icons/download";
import Heart from "@lucide/svelte/icons/heart";
import House from "@lucide/svelte/icons/house";
import MicVocal from "@lucide/svelte/icons/mic-vocal";
import Music from "@lucide/svelte/icons/music";
import Settings from "@lucide/svelte/icons/settings";
import { m } from "#lib/paraglide/messages.js";

export type NavItem = {
  href: string;
  label: () => string;
  icon: typeof House;
  /** Not shown in this version. */
  hidden?: boolean;
};

export const NAV_ITEMS: NavItem[] = [
  { href: "/", label: m.nav_home, icon: House },
  { href: "/favorites", label: m.nav_favorites, icon: Heart },
  { href: "/songs", label: m.nav_songs, icon: Music },
  { href: "/albums", label: m.nav_albums, icon: Disc3 },
  { href: "/artists", label: m.nav_artists, icon: MicVocal },
  // Downloads come in v0.2 (spec 6.8, 10).
  { href: "/downloads", label: m.nav_downloads, icon: Download, hidden: true },
  { href: "/settings", label: m.nav_settings, icon: Settings },
];

/** Home only matches itself; the others also match their sub-pages. */
export function isActive(item: NavItem, pathname: string): boolean {
  return item.href === "/" ? pathname === "/" : pathname === item.href || pathname.startsWith(`${item.href}/`);
}
