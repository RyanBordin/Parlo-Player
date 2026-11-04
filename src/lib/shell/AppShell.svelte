<script lang="ts">
  // App shell (spec 4): sidebar, Topbar over the scrolling content, and the
  // Player Bar, always visible on every page. On the phone (spec 4.5): phone
  // Topbar, the Mini Player and the tabs at the bottom.
  import { tick, type Snippet } from "svelte";
  import { afterNavigate, beforeNavigate } from "$app/navigation";
  import { restoreQueue, type SavedSession } from "#lib/api.ts";
  import NowPlaying from "#lib/components/now-playing/NowPlaying.svelte";
  import PhoneNowPlaying from "#lib/components/now-playing/PhoneNowPlaying.svelte";
  import MiniPlayer from "#lib/components/MiniPlayer.svelte";
  import PerfMeter from "#lib/components/PerfMeter.svelte";
  import PlayerBar from "#lib/components/PlayerBar.svelte";
  import SaveProgress from "#lib/components/SaveProgress.svelte";
  import Toasts from "#lib/components/Toasts.svelte";
  import Menu from "#lib/menu/Menu.svelte";
  import { closeMenu } from "#lib/menu/menu.svelte.ts";
  import MenuDialogs from "#lib/menu/MenuDialogs.svelte";
  import { closeNowPlaying, nowPlaying } from "#lib/now-playing.svelte.ts";
  import { listenToPlayer, queue } from "#lib/player.svelte.ts";
  import { listenToSaves } from "#lib/saves.svelte.ts";
  import { checkServer } from "#lib/server-info.svelte.ts";
  import { handleShortcut } from "#lib/shortcuts.ts";
  import { page } from "$app/state";
  import { navHistory, trackNavigation } from "#lib/shell/history.svelte.ts";
  import LibraryTabs from "#lib/shell/LibraryTabs.svelte";
  import { isTabPage, layout, libraryTabOf } from "#lib/shell/phone.svelte.ts";
  import PhoneTabs from "#lib/shell/PhoneTabs.svelte";
  import PhoneTopbar from "#lib/shell/PhoneTopbar.svelte";
  import Sidebar from "#lib/shell/Sidebar.svelte";
  import Topbar from "#lib/shell/Topbar.svelte";
  import { syncState } from "#lib/sync.svelte.ts";
  import { checkTestLibrary, testLibrary } from "#lib/test-library.svelte.ts";

  let { session, children }: { session: SavedSession; children: Snippet } = $props();

  let content = $state<HTMLElement>();

  // Phone (spec 4.5): the Library's sections show their tabs at the top, not
  // their sub-pages (an album, a playlist).
  let libraryTabs = $derived(layout.phone && libraryTabOf(page.url.pathname)?.href === page.url.pathname);
  // The phone Topbar already shows the tab pages' title: not twice.
  let tabPage = $derived(layout.phone && isTabPage(page.url.pathname));

  /** The tab already open was tapped again: back to the top of its page. */
  function scrollToTop() {
    content?.scrollTo({ top: 0, behavior: "smooth" });
  }

  // A new sign in starts a new history.
  navHistory.index = 0;
  navHistory.length = 1;

  // The page scrolls inside the content area, not the window, so the
  // browser does not remember where each page was: keep it per history entry.
  const scrollPositions: number[] = [];

  beforeNavigate((navigation) => {
    // A shallow entry (phone Now Playing, spec 4.5) stays on the same page.
    if (navigation.shallow) return;
    if (content) scrollPositions[navHistory.index] = content.scrollTop;
    // Spec 4.4: going to another page closes Now Playing.
    closeNowPlaying();
    closeMenu();
  });

  afterNavigate(async (navigation) => {
    trackNavigation(navigation);
    if (!content || navigation.shallow) return;
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
  listenToSaves();

  // "Open in Navidrome" (spec 8.2): asked again after a sync if the server was unreachable.
  $effect(() => {
    void syncState.finishedCount;
    checkServer();
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
<svelte:window onkeydown={onKeydown} />

<div class="shell" class:phone={layout.phone} class:tab-page={tabPage}>
  {#if !layout.phone}
    <Sidebar />
  {/if}
  <div class="main">
    {#if layout.phone}
      <PhoneTopbar {session} />
      {#if libraryTabs}
        <LibraryTabs />
      {/if}
    {:else}
      <Topbar {session} />
    {/if}
    <!-- Now Playing rises over the main viewport; the Topbar stays (spec 4, 4.4). -->
    <div class="viewport">
      <main class="content" bind:this={content}>
        {@render children()}
      </main>
      {#if nowPlaying.open && !layout.phone}
        <NowPlaying />
      {/if}
      {#if testLibrary.enabled}
        <PerfMeter />
      {/if}
    </div>
  </div>
  {#if layout.phone}
    <!-- Spec 4.5: the Mini Player shows only when there is a queue. -->
    {#if queue.tracks.length > 0}
      <MiniPlayer />
    {/if}
    <PhoneTabs onReselect={scrollToTop} />
    <!-- Spec 4.5: full screen, over the tabs too. -->
    {#if nowPlaying.open}
      <PhoneNowPlaying />
    {/if}
  {:else}
    <div class="player">
      <PlayerBar />
    </div>
  {/if}
</div>

<Toasts />
<SaveProgress />
<Menu />
<MenuDialogs />

<style>
  .shell {
    /* The body's height, without Android's system bars (global.css). */
    height: 100%;
    min-width: 1000px;
    display: grid;
    grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) var(--player-bar-height);
    overflow: hidden;
  }

  /* Phone (spec 4.5): one column, the tabs at the bottom in place of the
     sidebar. The desktop's 1000px minimum does not apply. */
  .shell.phone {
    min-width: 0;
    grid-template-columns: minmax(0, 1fr);
    /* Content, then the Mini Player (when there is a queue) and the tabs. */
    grid-template-rows: minmax(0, 1fr);
    grid-auto-rows: auto;
  }

  /* Spec 4.5: on the tab pages the title is the phone Topbar's only. */
  .shell.tab-page :global(.content .page-title) {
    display: none;
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
