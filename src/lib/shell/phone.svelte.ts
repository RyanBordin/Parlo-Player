// Phone layout (spec 4.5): below ~700px wide the app uses bottom tabs and a
// phone Topbar instead of the sidebar. From 700px up, the desktop layout
// (spec 4.1 to 4.4) stays, which also serves tablets in landscape.
import Disc3 from "@lucide/svelte/icons/disc-3";
import Heart from "@lucide/svelte/icons/heart";
import House from "@lucide/svelte/icons/house";
import LibraryBig from "@lucide/svelte/icons/library-big";
import ListMusic from "@lucide/svelte/icons/list-music";
import MicVocal from "@lucide/svelte/icons/mic-vocal";
import Music from "@lucide/svelte/icons/music";
import Search from "@lucide/svelte/icons/search";
import { m } from "#lib/paraglide/messages.js";
import type { NavItem } from "#lib/shell/nav-items.ts";

const PHONE_QUERY = "(max-width: 699px)";

const query = window.matchMedia(PHONE_QUERY);

export const layout = $state({ phone: query.matches });

query.addEventListener("change", (event) => {
  layout.phone = event.matches;
});

/** Library sections, as tabs at the top of their pages (spec 4.5). */
export const LIBRARY_TABS: NavItem[] = [
  { id: "songs", href: "/songs", label: m.nav_songs, icon: Music },
  { id: "albums", href: "/albums", label: m.nav_albums, icon: Disc3 },
  { id: "artists", href: "/artists", label: m.nav_artists, icon: MicVocal },
  { id: "playlists", href: "/playlists", label: m.playlists_label, icon: ListMusic },
];

const LIBRARY_KEY = "parlo.library-tab";

/** The Library tab opens the section chosen last time (spec 4.5). */
export function libraryHref(): string {
  try {
    const saved = localStorage.getItem(LIBRARY_KEY);
    if (LIBRARY_TABS.some((tab) => tab.href === saved)) return saved as string;
  } catch {
    // Storage unavailable: the first section.
  }
  return LIBRARY_TABS[0].href;
}

export function rememberLibraryTab(href: string) {
  try {
    localStorage.setItem(LIBRARY_KEY, href);
  } catch {
    // Not remembered this time.
  }
}

/** The library section a page belongs to, if any (its sub-pages included). */
export function libraryTabOf(pathname: string): NavItem | undefined {
  return LIBRARY_TABS.find((tab) => pathname === tab.href || pathname.startsWith(`${tab.href}/`));
}

/** Exactly these 4 tabs, in this order (spec 4.5). */
export const PHONE_TABS: NavItem[] = [
  { id: "home", href: "/", label: m.nav_home, icon: House },
  { id: "search", href: "/search", label: m.nav_search, icon: Search },
  { id: "library", href: "/songs", label: m.nav_library, icon: LibraryBig },
  { id: "favorites", href: "/favorites", label: m.nav_favorites, icon: Heart },
];

/** The pages the tabs open: everything else is an inner page, with a back arrow. */
const TAB_PAGES = new Set(["/", "/search", "/favorites", "/songs", "/albums", "/artists", "/playlists"]);

export function isTabPage(pathname: string): boolean {
  return TAB_PAGES.has(pathname);
}

/** The tab a page belongs to: the library's sections and their sub-pages go to Library. */
export function activeTab(pathname: string): string {
  if (libraryTabOf(pathname)) return "library";
  if (pathname === "/search") return "search";
  if (pathname === "/favorites" || pathname.startsWith("/favorites/")) return "favorites";
  return "home";
}
