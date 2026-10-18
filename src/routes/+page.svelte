<script lang="ts">
  import { onMount } from "svelte";
  import { getLibraryStatus, getSession, type SavedSession } from "#lib/api.ts";
  import ConnectedScreen from "#lib/screens/ConnectedScreen.svelte";
  import FirstSyncScreen from "#lib/screens/FirstSyncScreen.svelte";
  import LoginScreen from "#lib/screens/LoginScreen.svelte";
  import { startSync } from "#lib/sync.svelte.ts";

  type View =
    | { name: "loading" }
    | { name: "login" }
    | { name: "firstSync"; session: SavedSession }
    | { name: "connected"; session: SavedSession };

  // "loading" renders nothing, so the login screen doesn't flash on startup.
  let view = $state<View>({ name: "loading" });

  onMount(async () => {
    try {
      const session = await getSession();
      if (session) await openLibrary(session);
      else view = { name: "login" };
    } catch {
      // Unreadable config or credential store: ask to sign in again.
      view = { name: "login" };
    }
  });

  /** Never synced: show the first sync progress. Then sync by the rules (spec 12.4). */
  async function openLibrary(session: SavedSession) {
    let neverSynced = true;
    try {
      neverSynced = (await getLibraryStatus()).lastFullSyncAt === null;
    } catch {
      // The first sync screen shows any database error raised by the sync.
    }
    view = neverSynced ? { name: "firstSync", session } : { name: "connected", session };
    startSync(false);
  }
</script>

{#if view.name === "connected"}
  <ConnectedScreen session={view.session} onLogout={() => (view = { name: "login" })} />
{:else if view.name === "firstSync"}
  {@const session = view.session}
  <FirstSyncScreen onDone={() => (view = { name: "connected", session })} />
{:else if view.name === "login"}
  <LoginScreen onLogin={openLibrary} />
{/if}
