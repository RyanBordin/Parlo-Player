<script lang="ts">
  // Temporary player bar (step 5A). The real Player Bar comes with the app shell.
  import { seek, setVolume, togglePause } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  // While dragging, the bar follows the mouse instead of the playback position.
  let dragging = $state<number | null>(null);
  let commandError = $state<ErrorMessage | null>(null);

  let shownPosition = $derived(dragging ?? player.position);
  let canControl = $derived(player.track !== null && player.status !== "idle");
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
