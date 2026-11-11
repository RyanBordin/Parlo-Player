<script lang="ts">
  // Now Playing on the phone (spec 4.5): full screen, over the tabs too. ˅ and
  // the Back gesture close it: opening adds a history entry for the same page,
  // so Back takes it away. The big cover over its color (spec 4.5), then the
  // song, the progress, the transport and the song's actions.
  //
  // Up next, Lyrics and Related (spec 4.4) are named at the bottom: a tap on
  // a name, or dragging it up, opens them in a drawer over almost the whole
  // screen. Dragging down, a tap on the open tab or Back close it.
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Heart from "@lucide/svelte/icons/heart";
  import ListPlus from "@lucide/svelte/icons/list-plus";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import Repeat from "@lucide/svelte/icons/repeat";
  import Repeat1 from "@lucide/svelte/icons/repeat-1";
  import Shuffle from "@lucide/svelte/icons/shuffle";
  import SkipBack from "@lucide/svelte/icons/skip-back";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import { onMount } from "svelte";
  import { cubicOut } from "svelte/easing";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { cycleRepeat, next, previous, seek, setShuffle, setSongStarred, togglePause } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import CoverTint from "#lib/components/CoverTint.svelte";
  import PhoneBrand from "#lib/shell/PhoneBrand.svelte";
  import Lyrics from "#lib/components/now-playing/Lyrics.svelte";
  import Related from "#lib/components/now-playing/Related.svelte";
  import UpNext from "#lib/components/now-playing/UpNext.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { addToPlaylistMenu, openSongsMenu } from "#lib/menu/menus.ts";
  import {
    closeNowPlaying,
    NOW_PLAYING_TABS,
    nowPlaying,
    setNowPlayingTab,
    type NowPlayingTab,
  } from "#lib/now-playing.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player, queue } from "#lib/player.svelte.ts";
  import { appSettings, reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  /** The cover is about the screen's width: asked in high resolution. */
  const COVER_SIZE = 600;
  /** A swipe down further than this, or faster (px/ms), closes the screen. */
  const CLOSE_PX = 120;
  const CLOSE_SPEED = 0.5;
  /** How far a finger moves before a sideways swipe or a scroll is told apart. */
  const SWIPE_SLOP_PX = 10;
  /** A sideways swipe in the drawer further than this, or faster, changes tab. */
  const TAB_SWIPE_PX = 70;
  const TAB_SWIPE_SPEED = 0.4;
  /** Past the first and last tab, the content follows the finger this much. */
  const TAB_SWIPE_EDGE = 0.25;
  /** A swipe up from below the cover further than this, or faster, opens the drawer. */
  const OPEN_PX = 80;
  const OPEN_SPEED = 0.5;
  /** How far a finger moves on the tab names before it counts as a drag. */
  const DRAG_PX = 30;

  const LABELS: Record<NowPlayingTab, () => string> = {
    upNext: m.now_playing_up_next,
    lyrics: m.now_playing_lyrics,
    related: m.now_playing_related,
  };

  let sheetOpen = $state(false);
  /** ˅ with the drawer open goes back two entries at once: close everything. */
  let closingAll = false;
  /** Where a drag on the tab names started; a drag is not also a tap. */
  let dragStart: number | null = null;
  let dragged = false;

  let dragging = $state<number | null>(null);
  let commandError = $state<ErrorMessage | null>(null);
  let starring = $state(false);

  let track = $derived(player.track);
  let shownPosition = $derived(dragging ?? player.position);
  let canControl = $derived(track !== null && player.status !== "idle");
  let hasQueue = $derived(queue.tracks.length > 0);
  let progress = $derived(player.duration ? Math.min(shownPosition / player.duration, 1) * 100 : 0);
  let glowing = $derived(appSettings.nowPlayingBackdrop && Boolean(track?.coverArt));

  let repeatLabel = $derived(
    queue.repeat === "one"
      ? m.player_repeat_one()
      : queue.repeat === "all"
        ? m.player_repeat_all()
        : m.player_repeat_off(),
  );
  let error = $derived(
    commandError ?? (player.error ? { message: m.error_playback(), detail: player.error.detail } : null),
  );

  // One entry per page: one left by an earlier opening (closed by a link)
  // serves again.
  onMount(() => {
    if (!page.state.nowPlaying) addEntry({ nowPlaying: true });
  });

  function close() {
    if (sheetOpen && page.state.sheet) {
      closingAll = true;
      history.go(-2);
    } else if (page.state.nowPlaying) {
      history.back();
    } else {
      closeNowPlaying();
    }
  }

  /** Back: the drawer first, then the screen. */
  function onpopstate() {
    if (sheetOpen && !closingAll) sheetOpen = false;
    else closeNowPlaying();
  }

  function openSheet(tab: NowPlayingTab) {
    setNowPlayingTab(tab);
    if (sheetOpen) return;
    sheetOpen = true;
    addEntry({ nowPlaying: true, sheet: true });
  }

  /** A history entry for the same page (a shallow one), for Back to take away. */
  function addEntry(state: App.PageState) {
    goto(page.url.href, { state, shallow: true });
  }

  function closeSheet() {
    if (page.state.sheet) history.back();
    else sheetOpen = false;
  }

  function onTabClick(tab: NowPlayingTab) {
    if (dragged) return;
    if (sheetOpen && nowPlaying.tab === tab) closeSheet();
    else openSheet(tab);
  }

  function onTabsPointerDown(event: PointerEvent) {
    dragStart = event.clientY;
    dragged = false;
  }

  function onTabsPointerUp(event: PointerEvent) {
    if (dragStart === null) return;
    const moved = event.clientY - dragStart;
    dragStart = null;
    if (Math.abs(moved) < DRAG_PX) return;
    dragged = true;
    if (moved < 0 && !sheetOpen) openSheet(nowPlaying.tab);
    else if (moved > 0 && sheetOpen) closeSheet();
  }

  /** The artist's page replaces the entry, so Back does not land here again. */
  function openArtist(event: MouseEvent, href: string) {
    event.preventDefault();
    closeNowPlaying();
    goto(href, { replace: true });
  }

  async function run(action: () => Promise<unknown>) {
    commandError = null;
    try {
      await action();
    } catch (e) {
      commandError = describeError(e);
    }
  }

  function onSeekInput(event: Event) {
    dragging = Number((event.currentTarget as HTMLInputElement).value);
  }

  function onSeekChange(event: Event) {
    const target = Number((event.currentTarget as HTMLInputElement).value);
    player.position = target;
    dragging = null;
    run(() => seek(target));
  }

  // The heart changes at once and goes back if the server refuses.
  async function toggleStar() {
    const current = player.track;
    if (!current || starring) return;
    const starred = !current.starred;
    current.starred = starred;
    starring = true;
    commandError = null;
    try {
      await setSongStarred(current.id, starred);
    } catch (e) {
      if (player.track?.id === current.id) player.track.starred = !starred;
      commandError = describeError(e);
    } finally {
      starring = false;
    }
  }

  function openTrackMenu(event: MouseEvent) {
    if (track) openSongsMenu(event, [track], { label: queue.label });
  }

  function openAddToPlaylist(event: MouseEvent) {
    const current = track;
    if (current) openMenu(event, () => addToPlaylistMenu({ kind: "songs", ids: [current.id] }));
  }

  // Swipe on the screen (spec 4.5): down closes it, as ˅ does, following the
  // finger; up opens the drawer. Not on the progress bar, nor with the drawer
  // open (its tab names have their own drag).
  let screen = $state<HTMLElement>();
  /** How far the screen follows the finger down, in pixels. */
  let pull = $state(0);
  let pulling = $state(false);

  $effect(() => {
    const element = screen;
    if (!element) return;
    let start: { x: number; y: number; time: number; belowCover: boolean } | null = null;
    let direction: "down" | "up" | null = null;

    function onTouchStart(event: TouchEvent) {
      const target = event.target as Element;
      start = null;
      if (sheetOpen || event.touches.length > 1 || target.closest(".slider, .sheet-tabs")) return;
      const touch = event.touches[0];
      // Only from the song, the controls or the tab names: on the cover, a
      // swipe up does nothing.
      const cover = element?.querySelector(".frame")?.getBoundingClientRect();
      const belowCover = !cover || touch.clientY >= cover.bottom;
      start = { x: touch.clientX, y: touch.clientY, time: event.timeStamp, belowCover };
      direction = null;
    }

    function onTouchMove(event: TouchEvent) {
      if (!start || !element) return;
      const touch = event.touches[0];
      const dx = touch.clientX - start.x;
      const dy = touch.clientY - start.y;
      if (direction === null) {
        if (Math.abs(dy) < DRAG_PX || Math.abs(dy) < Math.abs(dx)) return;
        const atTop = element.scrollTop <= 0;
        const atBottom = element.scrollTop + element.clientHeight >= element.scrollHeight - 1;
        if (dy > 0 && atTop) direction = "down";
        else if (dy < 0 && atBottom && start.belowCover) direction = "up";
        else {
          start = null;
          return;
        }
      }
      if (direction === "down") {
        event.preventDefault();
        pulling = true;
        pull = Math.max(0, dy);
      }
    }

    function onTouchEnd(event: TouchEvent) {
      if (!start) return;
      const dy = (event.changedTouches[0]?.clientY ?? start.y) - start.y;
      const speed = dy / Math.max(1, event.timeStamp - start.time);
      start = null;
      pulling = false;
      if (direction === "down") {
        if (dy > CLOSE_PX || (speed > CLOSE_SPEED && dy > DRAG_PX)) close();
        else pull = 0;
      } else if (direction === "up" && (-dy > OPEN_PX || (-speed > OPEN_SPEED && -dy > DRAG_PX))) {
        openSheet(nowPlaying.tab);
      }
    }

    function onTouchCancel() {
      start = null;
      pulling = false;
      pull = 0;
    }

    element.addEventListener("touchstart", onTouchStart, { passive: true });
    element.addEventListener("touchmove", onTouchMove, { passive: false });
    element.addEventListener("touchend", onTouchEnd);
    element.addEventListener("touchcancel", onTouchCancel);
    return () => {
      element.removeEventListener("touchstart", onTouchStart);
      element.removeEventListener("touchmove", onTouchMove);
      element.removeEventListener("touchend", onTouchEnd);
      element.removeEventListener("touchcancel", onTouchCancel);
    };
  });

  // Sideways swipe in the drawer (spec 4.5): the content follows the finger
  // and the next tab slides in from that side. Not from the handle that
  // reorders Up next, and a row that scrolls sideways (Related) keeps the
  // gesture, unless it is at its start and the swipe goes back a tab.
  let sheetContent = $state<HTMLElement>();
  /** How far the drawer's content follows the finger sideways, in pixels. */
  let swipeX = $state(0);
  let swiping = $state(false);

  /** The tab `steps` away from the open one, if there is one. */
  function tabAt(steps: number): NowPlayingTab | undefined {
    return NOW_PLAYING_TABS[NOW_PLAYING_TABS.indexOf(nowPlaying.tab) + steps];
  }

  /** Opens the tab; it slides in from the side the finger came from. */
  function slideToTab(tab: NowPlayingTab, from: number) {
    setNowPlayingTab(tab);
    if (reducedMotion()) {
      swipeX = 0;
      return;
    }
    swiping = true;
    swipeX = from;
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        swiping = false;
        swipeX = 0;
      }),
    );
  }

  $effect(() => {
    const element = sheetContent;
    if (!element) return;
    let start: { x: number; y: number; time: number; row: Element | null } | null = null;
    let sideways = false;

    function onTouchStart(event: TouchEvent) {
      const target = event.target as Element;
      start = null;
      sideways = false;
      if (event.touches.length > 1 || target.closest(".handle")) return;
      const touch = event.touches[0];
      start = { x: touch.clientX, y: touch.clientY, time: event.timeStamp, row: target.closest(".track") };
    }

    function onTouchMove(event: TouchEvent) {
      if (!start) return;
      const touch = event.touches[0];
      const dx = touch.clientX - start.x;
      const dy = touch.clientY - start.y;
      if (!sideways) {
        if (Math.abs(dx) < SWIPE_SLOP_PX && Math.abs(dy) < SWIPE_SLOP_PX) return;
        const backFromRowStart = dx > 0 && (start.row?.scrollLeft ?? 0) <= 1;
        if (Math.abs(dx) < Math.abs(dy) * 1.5 || (start.row && !backFromRowStart)) {
          start = null;
          return;
        }
        sideways = true;
        swiping = true;
      }
      if (event.cancelable) event.preventDefault();
      swipeX = tabAt(dx < 0 ? 1 : -1) ? dx : dx * TAB_SWIPE_EDGE;
    }

    function onTouchEnd(event: TouchEvent) {
      if (!start || !sideways) {
        start = null;
        return;
      }
      const dx = (event.changedTouches[0]?.clientX ?? start.x) - start.x;
      const speed = Math.abs(dx) / Math.max(1, event.timeStamp - start.time);
      start = null;
      sideways = false;
      swiping = false;
      const tab = tabAt(dx < 0 ? 1 : -1);
      const far = Math.abs(dx) > TAB_SWIPE_PX || (speed > TAB_SWIPE_SPEED && Math.abs(dx) > DRAG_PX);
      if (tab && far) slideToTab(tab, dx < 0 ? element!.clientWidth : -element!.clientWidth);
      else swipeX = 0;
    }

    function onTouchCancel() {
      start = null;
      sideways = false;
      swiping = false;
      swipeX = 0;
    }

    element.addEventListener("touchstart", onTouchStart, { passive: true });
    element.addEventListener("touchmove", onTouchMove, { passive: false });
    element.addEventListener("touchend", onTouchEnd);
    element.addEventListener("touchcancel", onTouchCancel);
    return () => {
      element.removeEventListener("touchstart", onTouchStart);
      element.removeEventListener("touchmove", onTouchMove);
      element.removeEventListener("touchend", onTouchEnd);
      element.removeEventListener("touchcancel", onTouchCancel);
    };
  });

  /**
   * Rises from the bottom (and goes back down); at once with reduced motion.
   * Closing after a swipe goes on from where the finger left the screen.
   */
  function rise(_node: Element) {
    const from = pull;
    return {
      duration: reducedMotion() ? 0 : 280,
      easing: cubicOut,
      css: (t: number) => `transform: translateY(calc(${(1 - t) * 100}% + ${t * from}px))`,
    };
  }
</script>

<!-- Any Back while open (the gesture, or ˅ below) closes the drawer, then the screen. -->
<svelte:window {onpopstate} />

{#snippet tabNames()}
  <!-- Tap a name, or drag the names up (down to close). -->
  <div
    class="sheet-tabs"
    role="tablist"
    tabindex="-1"
    aria-label={m.now_playing_tabs()}
    onpointerdown={onTabsPointerDown}
    onpointerup={onTabsPointerUp}
    onpointercancel={() => (dragStart = null)}
  >
    <span class="grabber" aria-hidden="true"></span>
    <div class="names-row">
      {#each NOW_PLAYING_TABS as tab (tab)}
        <button
          type="button"
          role="tab"
          aria-selected={sheetOpen && nowPlaying.tab === tab}
          class:active={sheetOpen && nowPlaying.tab === tab}
          onclick={() => onTabClick(tab)}
        >
          {LABELS[tab]()}
        </button>
      {/each}
    </div>
  </div>
{/snippet}

<section
  class="now-playing"
  class:glowing
  class:pulling
  bind:this={screen}
  style:transform={pull ? `translateY(${pull}px)` : undefined}
  transition:rise
  aria-label={m.now_playing_tabs()}
>
  <!-- Background (spec 4.5): the cover's color over the whole screen. -->
  {#if glowing}
    <CoverTint coverId={track?.coverArt} fixed />
  {/if}

  <!-- The app's brand where the Home Topbar has it (spec 4.5). Swiping down
       and Back close the screen; the close button is there for screen
       readers only. -->
  <header class="top">
    <button class="visually-hidden" type="button" onclick={close}>{m.now_playing_close()}</button>
    <PhoneBrand />
  </header>

  <div class="frame">
    <div class="art">
      <Cover id={track?.coverArt} size={COVER_SIZE} radius="var(--radius-lg)" alt={track?.title ?? ""} />
    </div>
  </div>

  <div class="song">
    <div class="names">
      {#if track}
        <p class="title">{track.title}</p>
        {#if track.artist && track.artistId}
          {@const href = `/artists/${encodeURIComponent(track.artistId)}`}
          <a class="artist" {href} onclick={(event) => openArtist(event, href)}>{track.artist}</a>
        {:else if track.artist}
          <p class="artist">{track.artist}</p>
        {/if}
      {:else}
        <p class="artist">{m.player_nothing_playing()}</p>
      {/if}
    </div>
    <button
      class="icon favorite"
      class:active={track?.starred}
      type="button"
      onclick={toggleStar}
      disabled={!track || starring}
      aria-label={track?.starred ? m.player_unfavorite() : m.player_favorite()}
      aria-pressed={track?.starred ?? false}
    >
      <Heart fill={track?.starred ? "currentColor" : "none"} />
    </button>
  </div>

  {#if error}
    <p class="error">{error.message}</p>
  {/if}

  <div class="progress">
    <input
      class="slider"
      type="range"
      min="0"
      max={player.duration ?? 0}
      step="1"
      value={shownPosition}
      style:--fill={`${progress}%`}
      disabled={!canControl || !player.duration}
      aria-label={m.player_seek()}
      oninput={onSeekInput}
      onchange={onSeekChange}
    />
    <div class="times">
      <span>{formatDuration(canControl ? shownPosition : null)}</span>
      <span>{formatDuration(canControl ? player.duration : null)}</span>
    </div>
  </div>

  <div class="transport">
    <button
      class="icon mode"
      class:active={queue.shuffle}
      type="button"
      onclick={() => run(() => setShuffle(!queue.shuffle))}
      disabled={!hasQueue}
      aria-label={m.player_shuffle()}
      aria-pressed={queue.shuffle}
    >
      <Shuffle />
    </button>
    <button class="icon" type="button" onclick={() => run(previous)} disabled={!hasQueue} aria-label={m.player_previous()}>
      <SkipBack size={28} fill="currentColor" />
    </button>
    <button
      class="play"
      type="button"
      onclick={() => run(togglePause)}
      disabled={!canControl}
      aria-label={player.status === "playing" ? m.player_pause() : m.player_play()}
    >
      {#if player.status === "playing"}
        <Pause size={30} fill="currentColor" />
      {:else}
        <Play size={30} fill="currentColor" />
      {/if}
    </button>
    <button class="icon" type="button" onclick={() => run(next)} disabled={!hasQueue} aria-label={m.player_next()}>
      <SkipForward size={28} fill="currentColor" />
    </button>
    <button
      class="icon mode"
      class:active={queue.repeat !== "off"}
      type="button"
      onclick={() => run(cycleRepeat)}
      disabled={!hasQueue}
      aria-label={repeatLabel}
    >
      {#if queue.repeat === "one"}
        <Repeat1 />
      {:else}
        <Repeat />
      {/if}
    </button>
  </div>

  <div class="actions">
    <button
      class="icon"
      type="button"
      onclick={openAddToPlaylist}
      disabled={!track}
      aria-label={m.player_add_to_playlist()}
      aria-haspopup="menu"
    >
      <ListPlus />
    </button>
    <button class="icon" type="button" onclick={openTrackMenu} disabled={!track} aria-label={m.player_more()} aria-haspopup="menu">
      <Ellipsis />
    </button>
  </div>

  <div class="dock">
    {@render tabNames()}
  </div>

  {#if sheetOpen}
    <div class="sheet" transition:rise>
      {@render tabNames()}
      <div
        class="sheet-content"
        class:swiping
        role="tabpanel"
        bind:this={sheetContent}
        style:transform={swipeX ? `translateX(${swipeX}px)` : undefined}
      >
        <!-- Each tab loads only when it is opened. -->
        {#if nowPlaying.tab === "upNext"}
          <UpNext />
        {:else if nowPlaying.tab === "lyrics"}
          <Lyrics />
        {:else}
          <Related />
        {/if}
      </div>
    </div>
  {/if}
</section>

<style>
  /* Over everything, tabs included; clear of the system bars like the app. */
  .now-playing {
    position: fixed;
    inset: 0;
    z-index: 25;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: calc(env(safe-area-inset-top) + var(--space-2)) var(--page-gutter)
      calc(env(safe-area-inset-bottom) + var(--space-4));
    overflow-y: auto;
    background: var(--color-bg);
    -webkit-tap-highlight-color: transparent;
    /* Back in place after a short swipe; while the finger moves, at once. */
    transition: transform 200ms ease-out;
  }

  .now-playing.pulling {
    transition: none;
  }

  /* Same place as on the Home Topbar: that bar starts 12 px below the
     system bar and the screen 8 px below it, hence the 4 px here; the side
     margin is the page gutter on both. */
  .top {
    display: flex;
    align-items: center;
    min-height: 60px;
    padding-top: var(--space-1);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Square, as wide as the screen allows without pushing the controls away. */
  /*
   * The cover takes the room the rest leaves: the screen always fits, with
   * the tab names above the system's gesture bar, on any phone height. The
   * square is as big as the box allows (the box is a size container).
   */
  .frame {
    flex: 1 1 0;
    min-height: 120px;
    display: grid;
    place-items: center;
    margin-top: var(--space-4);
    container-type: size;
  }

  .art {
    width: min(100cqw, 100cqh);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-cover);
  }

  .song {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-4);
  }

  .names {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .names p,
  .names a {
    margin: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .artist {
    color: var(--color-text-muted);
    text-decoration: none;
  }

  /* Over the cover color the muted gray is too faint (spec 4.4). */
  .glowing .artist,
  .glowing .times {
    color: var(--color-text-on-image);
  }

  .error {
    margin: 0;
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .times {
    display: flex;
    justify-content: space-between;
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-variant-numeric: tabular-nums;
  }

  .transport {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .actions {
    display: flex;
    justify-content: space-between;
  }

  .icon {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
  }

  .icon:disabled,
  .play:disabled {
    opacity: 0.4;
  }

  .mode,
  .favorite {
    color: var(--color-text-muted);
  }

  .mode.active,
  .favorite.active {
    color: var(--color-accent);
  }

  /* Spec 4.5: Play/Pause stands out. */
  .play {
    width: 68px;
    height: 68px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    color: var(--color-bg);
  }

  /* The tab names at the bottom of the screen. */
  .dock {
    margin-top: auto;
  }

  /* The drawer: from just below the top bar ("˅ Playing from") to the bottom. */
  .sheet {
    position: fixed;
    top: calc(env(safe-area-inset-top) + 64px);
    right: 0;
    bottom: 0;
    left: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    /* A sideways swipe moves the content past the edges. */
    overflow: hidden;
    padding-bottom: env(safe-area-inset-bottom);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
  }

  .sheet .sheet-tabs {
    padding: 0 var(--page-gutter);
  }

  /* The desktop panels have no side margin of their own. */
  .sheet-content {
    flex: 1;
    min-height: 0;
    padding: 0 var(--space-4);
    /* Into place after a sideways swipe; while the finger moves, at once. */
    transition: transform 200ms ease-out;
  }

  .sheet-content.swiping {
    transition: none;
  }

  .sheet-tabs {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding-top: var(--space-2);
    touch-action: none;
  }

  .grabber {
    width: 36px;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--color-border-focus);
  }

  .names-row {
    width: 100%;
    display: flex;
  }

  .names-row button {
    flex: 1;
    padding: var(--space-3) var(--space-2);
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .names-row button.active {
    border-bottom-color: var(--color-text);
    color: var(--color-text);
  }

  .glowing .names-row button:not(.active) {
    color: var(--color-text-on-image);
  }

  .slider {
    width: 100%;
    height: 24px;
    margin: 0;
    appearance: none;
    background: transparent;
  }

  .slider::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--radius-pill);
    /* The rest of the song: a soft gray that shows on any cover color. */
    background: linear-gradient(to right, var(--color-text) var(--fill, 0%), rgb(255 255 255 / 22%) var(--fill, 0%));
  }

  .slider::-webkit-slider-thumb {
    width: 14px;
    height: 14px;
    margin-top: -5px;
    appearance: none;
    border-radius: var(--radius-pill);
    background: var(--color-text);
  }
</style>
