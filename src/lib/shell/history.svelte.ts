// Position in the navigation history, so the Topbar knows whether Back and
// Forward have somewhere to go (the browser does not tell).
import type { AfterNavigate } from "$app/navigation";

export const navHistory = $state({ index: 0, length: 1 });

/** Call from `afterNavigate`. */
export function trackNavigation(navigation: AfterNavigate): void {
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
