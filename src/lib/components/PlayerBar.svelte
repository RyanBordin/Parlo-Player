<script lang="ts">
  // Temporary player bar (step 5). The real Player Bar comes with the app shell.
  import { cycleRepeat, next, previous, seek, setShuffle, setVolume, togglePause } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player, queue } from "#lib/player.svelte.ts";

  // While dragging, the bar follows the mouse instead of the playback position.
  let dragging = $state<number | null>(null);
  let commandError = $state<ErrorMessage | null>(null);

  let shownPosition = $derived(dragging ?? player.position);
  let canControl = $derived(player.track !== null && player.status !== "idle");
  let hasQueue = $derived(queue.tracks.length > 0);
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
    run(() => setVolume(volume));
  }
</script>

<footer class="bar">
  <div class="track">
    {#if player.track}
      <p class="title">{player.track.title}</p>
      <p class="muted">
        {player.status === "loading" ? m.player_loading() : (player.track.artist ?? "")}
      </p>
    {:else}
      <p class="muted">{m.player_nothing_playing()}</p>
    {/if}
    {#if playbackError ?? commandError}
      {@const error = (playbackError ?? commandError)!}
      <p class="error" title={error.detail}>{error.message}</p>
    {/if}
  </div>

  <div class="transport">
    <div class="controls">
      <button
        class="icon"
        class:active={queue.shuffle}
        onclick={() => run(() => setShuffle(!queue.shuffle))}
        disabled={!hasQueue}
        aria-label={m.player_shuffle()}
        aria-pressed={queue.shuffle}
        title={m.player_shuffle()}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M16 4h4v4l-1.5-1.5-3.6 3.6-1.4-1.4 3.6-3.6zM4 17.6 15.6 6l1.4 1.4L5.4 19zM14.5 15.9l1.4-1.4 2.6 2.6L20 15.6V20h-4.4l1.5-1.5z" />
        </svg>
      </button>
      <button
        class="icon"
        onclick={() => run(previous)}
        disabled={!hasQueue}
        aria-label={m.player_previous()}
        title={m.player_previous()}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6h2v12H6zm3.5 6 8.5 6V6z" /></svg>
      </button>
      <button
        class="play"
        onclick={() => run(togglePause)}
        disabled={!canControl}
        aria-label={player.status === "playing" ? m.player_pause() : m.player_play()}
      >
        {#if player.status === "playing"}
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 5h4v14H7zM13 5h4v14h-4z" /></svg>
        {:else}
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14l11-7z" /></svg>
        {/if}
      </button>
      <button
        class="icon"
        onclick={() => run(next)}
        disabled={!hasQueue}
        aria-label={m.player_next()}
        title={m.player_next()}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M16 6h2v12h-2zM6 18l8.5-6L6 6z" /></svg>
      </button>
      <button
        class="icon"
        class:active={queue.repeat !== "off"}
        onclick={() => run(cycleRepeat)}
        disabled={!hasQueue}
        aria-label={repeatLabel}
        title={repeatLabel}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M7 7h10v3l4-4-4-4v3H5v6h2zm10 10H7v-3l-4 4 4 4v-3h12v-6h-2z" />
          {#if queue.repeat === "one"}
            <text x="12" y="15" text-anchor="middle" font-size="8" font-weight="700">1</text>
          {/if}
        </svg>
      </button>
    </div>

    <div class="progress">
      <span class="time">{formatDuration(shownPosition)}</span>
      <input
        type="range"
        min="0"
        max={player.duration ?? 0}
        step="1"
        value={shownPosition}
        disabled={!canControl || !player.duration}
        aria-label={m.player_seek()}
        oninput={onSeekInput}
        onchange={onSeekChange}
      />
      <span class="time">{formatDuration(player.duration)}</span>
    </div>
  </div>

  <label class="volume">
    <span class="visually-hidden">{m.player_volume()}</span>
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 9v6h4l5 4V5L8 9zm12.5 3a4.5 4.5 0 0 0-2.5-4v8a4.5 4.5 0 0 0 2.5-4z" />
    </svg>
    <input type="range" min="0" max="100" step="1" value={player.volume} oninput={onVolumeInput} />
  </label>
</footer>

<style>
  .bar {
    position: fixed;
    inset: auto 0 0 0;
    display: grid;
    grid-template-columns: 1fr 2fr 1fr;
    align-items: center;
    gap: var(--space-5);
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  .track {
    min-width: 0;
  }

  .track p {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-weight: var(--font-weight-medium);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-xs);
  }

  .transport {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .icon {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .icon:not(:disabled):hover {
    color: var(--color-text);
  }

  .icon:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon svg {
    width: 20px;
    height: 20px;
    fill: currentColor;
  }

  /* Shuffle / repeat on: accent for text and icons (spec 1.1). */
  .icon.active {
    color: var(--color-accent-text);
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
  }

  .play:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .play svg {
    width: 20px;
    height: 20px;
    fill: currentColor;
  }

  .progress {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .progress input {
    flex: 1;
  }

  .time {
    min-width: 3.5em;
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  .volume {
    justify-self: end;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .volume svg {
    width: 20px;
    height: 20px;
    fill: var(--color-text-muted);
  }

  input[type="range"] {
    accent-color: var(--color-accent-text);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
