<script lang="ts">
  // App shell (spec 4): sidebar, Topbar over the scrolling content, and the
  // Player Bar, always visible on every page.
  import type { Snippet } from "svelte";
  import { afterNavigate } from "$app/navigation";
  import { restoreQueue, type SavedSession } from "#lib/api.ts";
  import PlayerBar from "#lib/components/PlayerBar.svelte";
  import { listenToPlayer } from "#lib/player.svelte.ts";
  import { handleShortcut } from "#lib/shortcuts.ts";
  import { navHistory, trackNavigation } from "#lib/shell/history.svelte.ts";
  import Sidebar from "#lib/shell/Sidebar.svelte";
  import Topbar from "#lib/shell/Topbar.svelte";

  let { session, children }: { session: SavedSession; children: Snippet } = $props();

  let content = $state<HTMLElement>();

  // A new sign in starts a new history.
  navHistory.index = 0;
  navHistory.length = 1;

  afterNavigate((navigation) => {
    trackNavigation(navigation);
    // The page scrolls inside the content area, not the window: start a new
    // page at the top (Back / Forward keep the browser's behavior).
    if (navigation.type !== "popstate" && content) content.scrollTop = 0;
  });

  // Listen first, so the resumed queue (spec 5.6) reaches the player bar.
  $effect(() => {
    listenToPlayer()
      .then(restoreQueue)
      .catch(() => {
        // Player unavailable: the player bar reports it when used.
      });
  });
</script>

<!-- Spec 12.2: playback shortcuts on every page. -->
<svelte:window onkeydown={handleShortcut} />

<div class="shell">
  <Sidebar />
  <div class="main">
    <Topbar {session} />
    <main class="content" bind:this={content}>
      {@render children()}
    </main>
  </div>
  <div class="player">
    <PlayerBar />
  </div>
</div>

<style>
  .shell {
    height: 100vh;
    min-width: 1000px;
    display: grid;
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) var(--player-bar-height);
    overflow: hidden;
  }

  .main {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .player {
    grid-column: 1 / -1;
    min-width: 0;
  }
</style>
