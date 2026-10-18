// Current song's cover: compact in the Player Bar or expanded at the bottom
// of the sidebar (spec 4.3). A visual preference of this computer, kept in
// the webview's storage between sessions.
const STORAGE_KEY = "parlo.coverExpanded";

function readSaved(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "true";
  } catch {
    return false;
  }
}

export const coverMode = $state({ expanded: readSaved() });

export function setCoverExpanded(expanded: boolean): void {
  coverMode.expanded = expanded;
  try {
    localStorage.setItem(STORAGE_KEY, String(expanded));
  } catch {
    // Storage unavailable: the choice lasts until the app closes.
  }
}
