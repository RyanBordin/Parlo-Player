// Which part of the app is shown: sign in, first sync or the app shell.
import { goto } from "$app/navigation";
import { getLibraryStatus, getSession, logout, type SavedSession } from "#lib/api.ts";
import { clearAlbums } from "#lib/albums.svelte.ts";
import { clearHome } from "#lib/home.svelte.ts";
import { clearPages } from "#lib/page-cache.svelte.ts";
import { clearSongCache } from "#lib/song-cache.svelte.ts";
import { clearSongs } from "#lib/songs.svelte.ts";
import { startSync } from "#lib/sync.svelte.ts";

export type AppView =
  | { name: "loading" }
  | { name: "login" }
  | { name: "firstSync"; session: SavedSession }
  | { name: "connected"; session: SavedSession };

// "loading" renders nothing, so the login screen doesn't flash on startup.
export const app = $state<{ view: AppView }>({ view: { name: "loading" } });

/** On startup: the saved login goes straight in, otherwise sign in. */
export async function start(): Promise<void> {
  try {
    const session = await getSession();
    if (session) await openLibrary(session);
    else app.view = { name: "login" };
  } catch {
    // Unreadable config or credential store: ask to sign in again.
    app.view = { name: "login" };
  }
}

/** Never synced: show the first sync progress. Then sync by the rules (spec 12.4). */
export async function openLibrary(session: SavedSession): Promise<void> {
  let neverSynced = true;
  try {
    neverSynced = (await getLibraryStatus()).lastFullSyncAt === null;
  } catch {
    // The first sync screen shows any database error raised by the sync.
  }
  app.view = neverSynced ? { name: "firstSync", session } : { name: "connected", session };
  startSync(false);
}

export function finishFirstSync(): void {
  if (app.view.name === "firstSync") app.view = { name: "connected", session: app.view.session };
}

/** Deletes the saved login. The next sign in starts at Home. */
export async function signOut(): Promise<void> {
  await logout();
  clearHome();
  clearPages();
  clearAlbums();
  clearSongCache();
  clearSongs();
  app.view = { name: "login" };
  await goto("/", { replaceState: true });
}
