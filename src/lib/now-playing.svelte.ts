// The Now Playing screen (spec 4.4): open or closed, and the tab chosen,
// which is kept between sessions (this computer only).
const TAB_KEY = "parlo.nowPlayingTab";

export const NOW_PLAYING_TABS = ["upNext", "lyrics", "related"] as const;
export type NowPlayingTab = (typeof NOW_PLAYING_TABS)[number];

function readTab(): NowPlayingTab {
  try {
    const saved = localStorage.getItem(TAB_KEY);
    return NOW_PLAYING_TABS.find((tab) => tab === saved) ?? "upNext";
  } catch {
    return "upNext";
  }
}

export const nowPlaying = $state<{ open: boolean; tab: NowPlayingTab }>({ open: false, tab: readTab() });

export function toggleNowPlaying(): void {
  nowPlaying.open = !nowPlaying.open;
}

export function closeNowPlaying(): void {
  nowPlaying.open = false;
}

export function setNowPlayingTab(tab: NowPlayingTab): void {
  nowPlaying.tab = tab;
  try {
    localStorage.setItem(TAB_KEY, tab);
  } catch {
    // Storage unavailable: the choice lasts until the app closes.
  }
}
