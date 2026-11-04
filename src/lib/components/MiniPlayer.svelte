<script lang="ts">
  // Mini Player (spec 4.5): the phone's Player Bar, above the tabs. Cover,
  // title and artist, Play/Pause and Next, with a thin progress line at the
  // bottom; a tap anywhere else opens Now Playing. Volume is on the phone's
  // own buttons, so there is no volume control here.
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import { next, togglePause } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { toggleNowPlaying } from "#lib/now-playing.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player, queue } from "#lib/player.svelte.ts";

  const COVER_SIZE = 51;

  let commandError = $state<ErrorMessage | null>(null);

  let track = $derived(player.track);
  let canControl = $derived(track !== null && player.status !== "idle");
  let progress = $derived(player.duration ? Math.min(player.position / player.duration, 1) * 100 : 0);
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
    if (!(event.target as HTMLElement).closest("button")) toggleNowPlaying();
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

  <div class="line" aria-hidden="true">
    <div class="fill" style:width={`${progress}%`}></div>
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
  .line {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 2px;
    background: var(--color-border);
  }

  .fill {
    height: 100%;
    background: var(--color-text);
  }
</style>
