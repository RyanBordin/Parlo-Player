<script lang="ts">
  // Mini Player (spec 4.5): the phone's Player Bar, above the tabs. Cover,
  // title and artist, Play/Pause and Next, with a thin progress line at the
  // bottom that a tap or a drag moves (a taller invisible strip takes the
  // finger); a tap anywhere else opens Now Playing. Volume is on the phone's
  // own buttons, so there is no volume control here.
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import { next, seek, togglePause } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { toggleNowPlaying } from "#lib/now-playing.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player, queue } from "#lib/player.svelte.ts";

  const COVER_SIZE = 51;

  let commandError = $state<ErrorMessage | null>(null);

  let track = $derived(player.track);
  let canControl = $derived(track !== null && player.status !== "idle");
  /** Where the finger is while it drags the line, in seconds. */
  let dragging = $state<number | null>(null);
  let shownPosition = $derived(dragging ?? player.position);
  let progress = $derived(player.duration ? Math.min(shownPosition / player.duration, 1) * 100 : 0);
  let canSeek = $derived(canControl && Boolean(player.duration));
  let error = $derived(
    commandError ?? (player.error ? { message: m.error_playback(), detail: player.error.detail } : null),
  );

  async function run(action: () => Promise<unknown>) {
    commandError = null;
    try {
      await action();
    } catch (e) {
      commandError = describeError(e);
    }
  }

  function onclick(event: MouseEvent) {
    if (!(event.target as HTMLElement).closest("button, .seek")) toggleNowPlaying();
  }

  /** The position under the finger, in seconds. */
  function positionAt(event: PointerEvent): number {
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = Math.min(Math.max((event.clientX - box.left) / box.width, 0), 1);
    return ratio * (player.duration ?? 0);
  }

  function onSeekDown(event: PointerEvent) {
    if (!canSeek) return;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dragging = positionAt(event);
  }

  function onSeekMove(event: PointerEvent) {
    if (dragging !== null) dragging = positionAt(event);
  }

  function onSeekUp() {
    if (dragging === null) return;
    const target = dragging;
    player.position = target;
    dragging = null;
    run(() => seek(target));
  }
</script>

<!-- The whole bar opens Now Playing; its buttons do their own thing. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<footer class="mini" {onclick}>
  <div class="row">
    <div class="thumb" style:width={`${COVER_SIZE}px`}>
      <Cover id={track?.coverArt ?? null} size={COVER_SIZE} radius="var(--radius-sm)" />
    </div>

    <div class="meta">
      {#if track}
        <p class="title">{track.title}</p>
        {#if error}
          <p class="error">{error.message}</p>
        {:else if player.status === "loading"}
          <p class="artist">{m.player_loading()}</p>
        {:else if track.artist}
          <p class="artist">{track.artist}</p>
        {/if}
      {:else}
        <p class="artist">{error?.message ?? m.player_nothing_playing()}</p>
      {/if}
    </div>

    <button
      class="icon"
      type="button"
      onclick={() => run(togglePause)}
      disabled={!canControl}
      aria-label={player.status === "playing" ? m.player_pause() : m.player_play()}
    >
      {#if player.status === "playing"}
        <Pause size={25} fill="currentColor" />
      {:else}
        <Play size={25} fill="currentColor" />
      {/if}
    </button>
    <button class="icon" type="button" onclick={() => run(next)} disabled={queue.tracks.length === 0} aria-label={m.player_next()}>
      <SkipForward size={25} />
    </button>
  </div>

  <!-- A taller strip than the line takes the finger: a tap or a drag seeks. -->
  <div
    class="seek"
    class:active={dragging !== null}
    role="slider"
    tabindex="-1"
    aria-label={m.player_seek()}
    aria-valuemin={0}
    aria-valuemax={Math.round(player.duration ?? 0)}
    aria-valuenow={Math.round(shownPosition)}
    aria-disabled={!canSeek}
    onpointerdown={onSeekDown}
    onpointermove={onSeekMove}
    onpointerup={onSeekUp}
    onpointercancel={() => (dragging = null)}
  >
    <div class="line">
      <div class="fill" style:width={`${progress}%`}></div>
      {#if dragging !== null}
        <span class="knob" style:left={`${progress}%`}></span>
      {/if}
    </div>
  </div>
</footer>

<style>
  .mini {
    position: relative;
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
    -webkit-tap-highlight-color: transparent;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 10px var(--space-3) 10px var(--page-gutter);
  }

  /* The cover fills its box. */
  .thumb {
    flex: none;
  }

  .meta {
    flex: 1;
    min-width: 0;
  }

  p {
    margin: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 16px;
    font-weight: var(--font-weight-semibold);
  }

  .artist {
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-xs);
  }

  .icon {
    flex: none;
    width: 50px;
    height: 50px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
  }

  .icon:disabled {
    opacity: 0.4;
  }

  /* Thin progress line along the bottom edge. */
  /* Over the line and a little past it on both sides: about 20 px for the
     finger, the line itself stays thin. Above the tabs below. */
  .seek {
    position: absolute;
    right: 0;
    bottom: -9px;
    left: 0;
    z-index: 1;
    height: 20px;
    display: flex;
    align-items: center;
    touch-action: none;
  }

  .line {
    position: relative;
    width: 100%;
    height: 2px;
    background: var(--color-border);
    transition: height var(--transition-fast);
  }

  .seek.active .line {
    height: 4px;
  }

  .fill {
    height: 100%;
    background: var(--color-text);
  }

  .knob {
    position: absolute;
    top: 50%;
    width: 14px;
    height: 14px;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    transform: translate(-50%, -50%);
  }
</style>
