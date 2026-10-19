<script lang="ts">
  // App shell (spec 4): sidebar, Topbar over the scrolling content, and the
  // Player Bar, always visible on every page.
  import { tick, type Snippet } from "svelte";
  import { afterNavigate, beforeNavigate } from "$app/navigation";
  import { restoreQueue, type SavedSession } from "#lib/api.ts";
  import NowPlaying from "#lib/components/now-playing/NowPlaying.svelte";
  import PerfMeter from "#lib/components/PerfMeter.svelte";
  import PlayerBar from "#lib/components/PlayerBar.svelte";
  import Toasts from "#lib/components/Toasts.svelte";
  import { closeNowPlaying, nowPlaying } from "#lib/now-playing.svelte.ts";
  import { listenToPlayer } from "#lib/player.svelte.ts";
  import { handleShortcut } from "#lib/shortcuts.ts";
  import { navHistory, trackNavigation } from "#lib/shell/history.svelte.ts";
  import Sidebar from "#lib/shell/Sidebar.svelte";
  import Topbar from "#lib/shell/Topbar.svelte";
  import { checkTestLibrary, testLibrary } from "#lib/test-library.svelte.ts";

  let { session, children }: { session: SavedSession; children: Snippet } = $props();

  let content = $state<HTMLElement>();

  // A new sign in starts a new history.
  navHistory.index = 0;
  navHistory.length = 1;

  // The page scrolls inside the content area, not the window, so the
  // browser does not remember where each page was: keep it per history entry.
  const scrollPositions: number[] = [];

  beforeNavigate(() => {
    if (content) scrollPositions[navHistory.index] = content.scrollTop;
    // Spec 4.4: going to another page closes Now Playing.
    closeNowPlaying();
  });

  afterNavigate(async (navigation) => {
    trackNavigation(navigation);
    if (!content) return;
    if (navigation.type === "popstate") {
      // Back / Forward: where that page was. Pages with kept data (Home,
      // "See more") are already drawn, so the position fits.
      await tick();
      content.scrollTop = scrollPositions[navHistory.index] ?? 0;
    } else {
      content.scrollTop = 0;
    }
  });

  function onKeydown(event: KeyboardEvent) {
    // Esc closes Now Playing (spec 4.4); an open dialog takes it first.
    if (event.key === "Escape" && nowPlaying.open && !event.defaultPrevented && !document.querySelector("dialog[open]")) {
      closeNowPlaying();
      return;
    }
    handleShortcut(event);
  }

  checkTestLibrary();

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
<svelte:window onkeydown={onKeydown} />

<div class="shell">
  <Sidebar />
  <div class="main">
    <Topbar {session} />
    <!-- Now Playing rises over the main viewport; the Topbar stays (spec 4, 4.4). -->
    <div class="viewport">
      <main class="content" bind:this={content}>
        {@render children()}
      </main>
      {#if nowPlaying.open}
        <NowPlaying />
      {/if}
      {#if testLibrary.enabled}
        <PerfMeter />
      {/if}
    </div>
  </div>
  <div class="player">
    <PlayerBar />
  </div>
</div>

<Toasts />

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

  .viewport {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
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
