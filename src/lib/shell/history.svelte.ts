// Position in the navigation history, so the Topbar knows whether Back and
// Forward have somewhere to go (the browser does not tell).
import { goto, type AfterNavigate } from "$app/navigation";

export const navHistory = $state({ index: 0, length: 1 });

/** A `replaceUrl` is under way: the entry changes, the position does not. */
let replacing = false;

/**
 * Changes the address of the current entry instead of adding one (the
 * search as it is typed). Focus and scrolling stay as they are.
 */
export async function replaceUrl(url: string): Promise<void> {
  replacing = true;
  try {
    await goto(url, { replace: true, reset: false });
  } finally {
    replacing = false;
  }
}

/** Call from `afterNavigate`. */
export function trackNavigation(navigation: AfterNavigate): void {
  if (replacing && navigation.type === "goto") return;
  switch (navigation.type) {
    case "enter":
      navHistory.index = 0;
      navHistory.length = 1;
      break;
    case "popstate":
      navHistory.index = Math.min(Math.max(navHistory.index + navigation.delta, 0), navHistory.length - 1);
      break;
    default:
      // A link to the page already open replaces the entry instead of adding one.
      if (navigation.from?.url.href === navigation.to?.url.href) break;
      navHistory.index += 1;
      // A new page after going back drops the pages that were ahead.
      navHistory.length = navHistory.index + 1;
  }
}

export function canGoBack(): boolean {
  return navHistory.index > 0;
}

export function canGoForward(): boolean {
  return navHistory.index < navHistory.length - 1;
}
