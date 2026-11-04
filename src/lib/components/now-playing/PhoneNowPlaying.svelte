<script lang="ts">
  // Now Playing on the phone (spec 4.5): full screen, over the tabs too. ˅ and
  // the Back gesture close it: opening adds a history entry for the same page,
  // so Back takes it away. The big cover sits on its glow (spec 4.4), then the
  // song, the progress, the transport and the song's actions.
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
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
  import { goto, pushState } from "$app/navigation";
  import { page } from "$app/state";
  import { cycleRepeat, next, previous, seek, setShuffle, setSongStarred, togglePause } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import CoverGlow from "#lib/components/now-playing/CoverGlow.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { addToPlaylistMenu, openSongsMenu } from "#lib/menu/menus.ts";
  import { closeNowPlaying } from "#lib/now-playing.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player, queue } from "#lib/player.svelte.ts";
  import { appSettings, reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  /** The cover is about the screen's width: asked in high resolution. */
  const COVER_SIZE = 600;

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
  let origin = $derived(
    queue.continuingFrom === null
      ? m.queue_playing_from({ label: queue.label })
      : queue.continuingFrom
        ? m.queue_continuing_from({ app: queue.continuingFrom })
        : m.queue_continuing_from_other(),
  );
  let error = $derived(
    commandError ?? (player.error ? { message: m.error_playback(), detail: player.error.detail } : null),
  );

  // One entry per page: one left by an earlier opening (closed by a link)
  // serves again.
  onMount(() => {
    if (!page.state.nowPlaying) pushState("", { nowPlaying: true });
  });

  function close() {
    if (page.state.nowPlaying) history.back();
    else closeNowPlaying();
  }

  /** The artist's page replaces the entry, so Back does not land here again. */
  function openArtist(event: MouseEvent, href: string) {
    event.preventDefault();
    closeNowPlaying();
    goto(href, { replaceState: true });
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

  /** Rises from the bottom (and goes back down); at once with reduced motion. */
  function rise(_node: Element) {
    return {
      duration: reducedMotion() ? 0 : 280,
      easing: cubicOut,
      css: (t: number) => `transform: translateY(${(1 - t) * 100}%)`,
    };
  }
</script>

<!-- Any Back while open (the gesture, or ˅ below) closes it. -->
<svelte:window onpopstate={closeNowPlaying} />

<section class="now-playing" class:glowing transition:rise aria-label={m.now_playing_tabs()}>
  {#if glowing}
    <CoverGlow coverId={track?.coverArt} />
  {/if}

  <header class="top">
    <button class="icon" type="button" onclick={close} aria-label={m.now_playing_close()}>
      <ChevronDown size={26} />
    </button>
    <p class="origin">{origin}</p>
  </header>

  <div class="frame">
    <Cover id={track?.coverArt} size={COVER_SIZE} radius="var(--radius-lg)" alt={track?.title ?? ""} />
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
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: calc(-1 * var(--space-2));
  }

  .origin {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  /* Square, as wide as the screen allows without pushing the controls away. */
  .frame {
    align-self: center;
    width: min(100%, 52vh);
    margin-top: var(--space-4);
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

  /* Over the glow the muted gray is too faint on light covers (spec 4.4). */
  .glowing .artist,
  .glowing .origin,
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
    background: linear-gradient(to right, var(--color-text) var(--fill, 0%), var(--color-border) var(--fill, 0%));
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
