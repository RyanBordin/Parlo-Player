<script lang="ts">
  // Player Bar (spec 4.3): track | transport and progress | song actions | audio and queue.
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronUp from "@lucide/svelte/icons/chevron-up";
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
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import {
    cycleRepeat,
    next,
    previous,
    seek,
    setShuffle,
    setSongStarred,
    setVolume,
    toggleMute,
    togglePause,
  } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { coverMode, setCoverExpanded } from "#lib/cover-mode.svelte.ts";
  import { nowPlaying, toggleNowPlaying } from "#lib/now-playing.svelte.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player, queue } from "#lib/player.svelte.ts";

  // While dragging, the bar follows the mouse instead of the playback position.
  let dragging = $state<number | null>(null);
  let commandError = $state<ErrorMessage | null>(null);
  let starring = $state(false);

  let track = $derived(player.track);
  let shownPosition = $derived(dragging ?? player.position);
  let canControl = $derived(track !== null && player.status !== "idle");
  let hasQueue = $derived(queue.tracks.length > 0);
  let progress = $derived(player.duration ? Math.min(shownPosition / player.duration, 1) * 100 : 0);
  let silent = $derived(player.muted || player.volume === 0);
  let repeatLabel = $derived(
    queue.repeat === "one"
      ? m.player_repeat_one()
      : queue.repeat === "all"
        ? m.player_repeat_all()
        : m.player_repeat_off(),
  );
  let playbackError = $derived(
    player.error ? { message: m.error_playback(), detail: player.error.detail } : null,
  );

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

  function onVolumeInput(event: Event) {
    const volume = Number((event.currentTarget as HTMLInputElement).value);
    player.volume = volume;
    player.muted = false;
    run(() => setVolume(volume));
  }

  let canOpenNowPlaying = $derived(track !== null || hasQueue);

  // Spec 4.3: a click on an empty part of the bar opens / closes Now Playing.
  function onBarClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (canOpenNowPlaying && !target.closest("button, a, input, label")) toggleNowPlaying();
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
</script>

<!-- The click on empty parts is a mouse shortcut; the arrow button does the same. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<footer class="bar" onclick={onBarClick}>
  <!-- Zone 1: the song -->
  <div class="zone track">
    {#if track && !coverMode.expanded}
      <div class="thumb">
        <Cover id={track.coverArt} size={56} radius="var(--radius-sm)" />
        <button
          class="cover-toggle"
          type="button"
          onclick={() => setCoverExpanded(true)}
          aria-label={m.player_expand_cover()}
          title={m.player_expand_cover()}
        >
          <ChevronUp />
        </button>
      </div>
    {/if}
    <div class="meta">
      {#if track}
        <p class="title" title={track.title}>{track.title}</p>
        {#if player.status === "loading"}
          <p class="artist">{m.player_loading()}</p>
        {:else}
          {#if track.artist}
            {#if track.artistId}
              <a class="artist" href={`/artists/${encodeURIComponent(track.artistId)}`}>{track.artist}</a>
            {:else}
              <p class="artist">{track.artist}</p>
            {/if}
          {/if}
          {#if track.album}
            {#if track.albumId}
              <a class="album" href={`/albums/${encodeURIComponent(track.albumId)}`}>{track.album}</a>
            {:else}
              <p class="album">{track.album}</p>
            {/if}
          {/if}
        {/if}
      {:else}
        <p class="artist">{m.player_nothing_playing()}</p>
      {/if}
      {#if playbackError ?? commandError}
        {@const error = (playbackError ?? commandError)!}
        <p class="error" title={error.detail}>{error.message}</p>
      {/if}
    </div>
  </div>

  <!-- Zone 2: transport and progress -->
  <div class="zone transport">
    <div class="controls">
      <button class="icon" onclick={() => run(previous)} disabled={!hasQueue} aria-label={m.player_previous()} title={m.player_previous()}>
        <SkipBack size={20} />
      </button>
      <button
        class="play"
        onclick={() => run(togglePause)}
        disabled={!canControl}
        aria-label={player.status === "playing" ? m.player_pause() : m.player_play()}
        title={player.status === "playing" ? m.player_pause() : m.player_play()}
      >
        {#if player.status === "playing"}
          <Pause size={20} fill="currentColor" />
        {:else}
          <Play size={20} fill="currentColor" />
        {/if}
      </button>
      <button class="icon" onclick={() => run(next)} disabled={!hasQueue} aria-label={m.player_next()} title={m.player_next()}>
        <SkipForward size={20} />
      </button>
    </div>

    <div class="progress">
      <span class="time">{formatDuration(canControl ? shownPosition : null)}</span>
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
      <span class="time">{formatDuration(canControl ? player.duration : null)}</span>
    </div>
  </div>

  <div class="zone right">
    <!-- Zone 3: actions on the current song (menus come in step 8) -->
    <div class="actions">
      <button
        class="icon favorite"
        class:active={track?.starred}
        onclick={toggleStar}
        disabled={!track || starring}
        aria-label={track?.starred ? m.player_unfavorite() : m.player_favorite()}
        aria-pressed={track?.starred ?? false}
        title={track?.starred ? m.player_unfavorite() : m.player_favorite()}
      >
        <Heart fill={track?.starred ? "currentColor" : "none"} />
      </button>
      <button class="icon add" type="button" disabled={!track} aria-label={m.player_add_to_playlist()} title={m.player_add_to_playlist()}>
        <ListPlus />
      </button>
      <button class="icon" type="button" disabled={!track} aria-label={m.player_more()} title={m.player_more()}>
        <Ellipsis />
      </button>
    </div>

    <!-- Zone 4: audio, queue modes and Now Playing -->
    <div class="audio">
      <div class="volume">
        <button
          class="icon"
          onclick={() => run(toggleMute)}
          aria-label={player.muted ? m.player_unmute() : m.player_mute()}
          aria-pressed={player.muted}
          title={player.muted ? m.player_unmute() : m.player_mute()}
        >
          {#if silent}
            <VolumeX />
          {:else if player.volume < 50}
            <Volume1 />
          {:else}
            <Volume2 />
          {/if}
        </button>
        <input
          class="slider volume-slider"
          type="range"
          min="0"
          max="100"
          step="1"
          value={player.volume}
          style:--fill={`${player.muted ? 0 : player.volume}%`}
          aria-label={m.player_volume()}
          oninput={onVolumeInput}
        />
      </div>
      <button
        class="icon mode"
        class:active={queue.repeat !== "off"}
        onclick={() => run(cycleRepeat)}
        disabled={!hasQueue}
        aria-label={repeatLabel}
        title={repeatLabel}
      >
        {#if queue.repeat === "one"}
          <Repeat1 />
        {:else}
          <Repeat />
        {/if}
      </button>
      <button
        class="icon mode"
        class:active={queue.shuffle}
        onclick={() => run(() => setShuffle(!queue.shuffle))}
        disabled={!hasQueue}
        aria-label={m.player_shuffle()}
        aria-pressed={queue.shuffle}
        title={m.player_shuffle()}
      >
        <Shuffle />
      </button>
      <button
        class="icon"
        type="button"
        onclick={toggleNowPlaying}
        disabled={!canOpenNowPlaying}
        aria-expanded={nowPlaying.open}
        aria-label={nowPlaying.open ? m.now_playing_close() : m.player_now_playing()}
        title={nowPlaying.open ? m.now_playing_close() : m.player_now_playing()}
      >
        {#if nowPlaying.open}<ChevronDown />{:else}<ChevronUp />{/if}
      </button>
    </div>
  </div>
</footer>

<style>
  .bar {
    height: 100%;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 560px) minmax(0, 1fr);
    align-items: center;
    gap: var(--space-5);
    padding: 0 var(--space-4);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  .zone {
    min-width: 0;
  }

  /* Zone 1 */
  .track {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .thumb {
    position: relative;
    flex: none;
    width: 56px;
  }

  /* Spec 4.3: the ˄ arrow shows on hover. */
  .cover-toggle {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--color-scrim);
    color: var(--color-text);
    opacity: 0;
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .thumb:hover .cover-toggle,
  .cover-toggle:focus-visible {
    opacity: 1;
  }

  .meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta > * {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: var(--font-weight-bold);
  }

  .artist,
  .album {
    width: fit-content;
    max-width: 100%;
    font-size: var(--font-size-sm);
    text-decoration: none;
  }

  .artist {
    color: var(--color-text-muted);
  }

  .album {
    color: var(--color-text-subtle);
  }

  a.artist:hover,
  a.album:hover {
    color: var(--color-text);
    text-decoration: underline;
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-xs);
  }

  /* Zone 2 */
  .transport {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .play {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    color: var(--color-text-inverse);
    cursor: pointer;
    transition: transform var(--transition-fast);
  }

  .play:not(:disabled):hover {
    transform: scale(1.05);
  }

  .play:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .progress {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .progress .slider {
    flex: 1;
  }

  .time {
    min-width: 3.5em;
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  /* Zones 3 and 4 */
  .right {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .actions,
  .audio,
  .volume {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .audio {
    margin-left: auto;
  }

  .volume-slider {
    width: 96px;
    margin-right: var(--space-2);
  }

  .icon {
    position: relative;
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .icon:not(:disabled):hover {
    color: var(--color-text);
  }

  .icon:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .favorite.active {
    color: var(--color-text);
  }

  /* Repeat / shuffle on: accent color and a dot below (spec 1.1). */
  .mode.active {
    color: var(--color-accent-text);
  }

  .mode.active::after {
    content: "";
    position: absolute;
    bottom: 2px;
    width: 4px;
    height: 4px;
    border-radius: var(--radius-pill);
    background: currentColor;
  }

  /* Thin bars: white up to the current value, grey after it. */
  .slider {
    height: 16px;
    margin: 0;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  .slider:disabled {
    cursor: default;
  }

  .slider::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--radius-pill);
    background: linear-gradient(
      to right,
      var(--color-text) var(--fill, 0%),
      var(--color-border) var(--fill, 0%)
    );
  }

  .slider::-webkit-slider-thumb {
    width: 12px;
    height: 12px;
    margin-top: -4px;
    appearance: none;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .slider:not(:disabled):hover::-webkit-slider-thumb,
  .slider:focus-visible::-webkit-slider-thumb {
    opacity: 1;
  }

  /* Spec 4.3, narrow window: first the volume slider goes, then the song
     actions (they will be in the "..." menu, step 8). */
  @media (max-width: 1199px) {
    .volume-slider {
      display: none;
    }
  }

  @media (max-width: 1099px) {
    .favorite,
    .add {
      display: none;
    }
  }
</style>
