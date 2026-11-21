<script lang="ts">
  // Remote control (spec 12.8): over the phone's Now Playing, what a computer
  // on the same network is playing, its transport and volume, and its next
  // songs (a tap plays one there). With several computers found, the list
  // comes first.
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import Monitor from "@lucide/svelte/icons/monitor";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import SkipBack from "@lucide/svelte/icons/skip-back";
  import SkipForward from "@lucide/svelte/icons/skip-forward";
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import { cubicOut } from "svelte/easing";
  import type { RemoteCommand, RemoteDevice } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { connectRemote, remote, sendRemote } from "#lib/remote.svelte.ts";
  import { reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  let { onclose }: { onclose: () => void } = $props();

  /** While the finger is on the volume, it goes this often to the computer. */
  const VOLUME_SEND_MS = 150;
  const COVER_SIZE = 160;
  const ROW_COVER_SIZE = 48;

  let commandError = $state<ErrorMessage | null>(null);
  /** The volume under the finger, ahead of what the computer reports. */
  let draggingVolume = $state<number | null>(null);
  let lastVolumeSent = 0;

  let status = $derived(remote.status);
  let connected = $derived(status?.status === "connected");
  let playing = $derived(remote.player);
  let track = $derived(playing?.track ?? null);
  let canControl = $derived(connected && track !== null && playing?.status !== "idle");
  let duration = $derived(remote.position.duration ?? track?.duration ?? null);
  let progress = $derived(duration ? Math.min(remote.position.position / duration, 1) * 100 : 0);
  let volume = $derived(draggingVolume ?? playing?.volume ?? 0);
  /** The device list: nothing chosen yet, or the chosen one failed. */
  /** "Switch computer" tapped: the list, over the connected one. */
  let switching = $state(false);
  let choosing = $derived(
    switching || status === null || (status.status === "closed" && remote.devices.length > 1),
  );
  let canSwitch = $derived(!choosing && remote.devices.length > 1);
  let problem = $derived.by(() => {
    if (status?.status !== "closed" || !status.problem) return null;
    const name = status.name;
    switch (status.problem) {
      case "unreachable":
        return m.remote_problem_unreachable({ name });
      case "wrongAccount":
        return m.remote_problem_wrong_account({ name });
      case "signedOut":
        return m.remote_problem_signed_out({ name });
      case "version":
        return m.remote_problem_version({ name });
      case "lost":
        return m.remote_problem_lost({ name });
    }
  });
  // The current song first, then the next ones.
  let rows = $derived(
    (remote.queue?.tracks ?? []).map((song, i) => ({ song, index: (remote.queue?.offset ?? 0) + i })),
  );
  let more = $derived(remote.queue ? remote.queue.total - remote.queue.offset - remote.queue.tracks.length : 0);
  let origin = $derived(
    remote.queue
      ? remote.queue.continuingFrom === null
        ? m.queue_playing_from({ label: remote.queue.label })
        : remote.queue.continuingFrom
          ? m.queue_continuing_from({ app: remote.queue.continuingFrom })
          : m.queue_continuing_from_other()
      : "",
  );

  async function run(command: RemoteCommand) {
    commandError = null;
    try {
      await sendRemote(command);
    } catch (e) {
      commandError = describeError(e);
    }
  }

  async function choose(device: RemoteDevice) {
    commandError = null;
    switching = false;
    // Already connected to it: only back to its controls.
    if (status?.deviceId === device.id && status.status !== "closed") return;
    try {
      await connectRemote(device);
    } catch (e) {
      commandError = describeError(e);
    }
  }

  function retry() {
    const device = remote.devices.find((d) => d.id === status?.deviceId);
    if (device) choose(device);
    else commandError = { message: m.remote_problem_unreachable({ name: status?.name ?? "" }) };
  }

  function onVolumeInput(event: Event) {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    draggingVolume = value;
    const now = Date.now();
    if (now - lastVolumeSent >= VOLUME_SEND_MS) {
      lastVolumeSent = now;
      run({ action: "setVolume", volume: value });
    }
  }

  function onVolumeChange(event: Event) {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    run({ action: "setVolume", volume: value }).finally(() => (draggingVolume = null));
  }

  function rise(_node: Element) {
    return { duration: reducedMotion() ? 0 : 250, easing: cubicOut, css: (t: number) => `transform: translateY(${(1 - t) * 100}%)` };
  }
</script>

<section class="remote" transition:rise aria-label={m.remote_open()}>
  <header class="top">
    <button class="icon" type="button" onclick={onclose} aria-label={m.remote_close()}>
      <ChevronDown size={26} />
    </button>
    <p class="heading">
      {#if choosing}
        {m.remote_choose()}
      {:else if status}
        {m.remote_playing_on({ name: status.name })}
      {/if}
    </p>
    <!-- Several computers on the network: another one can be chosen. -->
    {#if canSwitch}
      <button type="button" class="switch" onclick={() => (switching = true)}>{m.remote_switch()}</button>
    {/if}
  </header>

  {#if commandError}
    <p class="error">{commandError.message}</p>
  {/if}

  {#if choosing}
    {#if problem}
      <p class="message">{problem}</p>
    {/if}
    <ul class="devices">
      {#each remote.devices as device (device.id)}
        <li>
          <button type="button" class="device" onclick={() => choose(device)}>
            <Monitor size={22} />
            <span>{device.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  {:else if status?.status === "connecting"}
    <p class="message">{m.remote_connecting({ name: status.name })}</p>
  {:else if status?.status === "closed"}
    <p class="message">{problem ?? m.remote_problem_lost({ name: status.name })}</p>
    <button type="button" class="retry" onclick={retry}>{m.remote_retry()}</button>
  {:else if connected}
    <div class="song">
      <div class="art">
        <Cover id={track?.coverArt} size={COVER_SIZE} radius="var(--radius-md)" alt={track?.title ?? ""} />
      </div>
      <div class="names">
        {#if track}
          <p class="title">{track.title}</p>
          {#if track.artist}<p class="artist">{track.artist}</p>{/if}
        {:else}
          <p class="artist">{m.player_nothing_playing()}</p>
        {/if}
      </div>
    </div>

    <div class="progress">
      <div class="bar" aria-hidden="true"><div class="fill" style:width={`${progress}%`}></div></div>
      <div class="times">
        <span>{formatDuration(canControl ? remote.position.position : null)}</span>
        <span>{formatDuration(canControl ? duration : null)}</span>
      </div>
    </div>

    <div class="transport">
      <button class="icon" type="button" onclick={() => run({ action: "previous" })} disabled={rows.length === 0} aria-label={m.player_previous()}>
        <SkipBack size={28} fill="currentColor" />
      </button>
      <button
        class="play"
        type="button"
        onclick={() => run({ action: "togglePause" })}
        disabled={!canControl}
        aria-label={playing?.status === "playing" ? m.player_pause() : m.player_play()}
      >
        {#if playing?.status === "playing"}
          <Pause size={30} fill="currentColor" />
        {:else}
          <Play size={30} fill="currentColor" />
        {/if}
      </button>
      <button class="icon" type="button" onclick={() => run({ action: "next" })} disabled={rows.length === 0} aria-label={m.player_next()}>
        <SkipForward size={28} fill="currentColor" />
      </button>
    </div>

    <div class="volume">
      <Volume1 size={20} />
      <input
        class="slider"
        type="range"
        min="0"
        max="100"
        step="1"
        value={volume}
        style:--fill={`${volume}%`}
        aria-label={m.player_volume()}
        aria-valuetext={m.player_volume_value({ value: Math.round(volume) })}
        oninput={onVolumeInput}
        onchange={onVolumeChange}
      />
      <Volume2 size={20} />
    </div>

    <div class="queue">
      <p class="queue-heading">{m.now_playing_up_next()}</p>
      {#if origin}<p class="origin">{origin}</p>{/if}
      <ul class="rows">
        {#each rows as { song, index } (index)}
          <li>
            <button
              type="button"
              class="row"
              class:current={index === remote.queue?.currentIndex}
              onclick={() => run({ action: "jumpTo", index, songId: song.id })}
            >
              <span class="row-cover">
                <Cover id={song.coverArt} size={ROW_COVER_SIZE} radius="var(--radius-sm)" />
              </span>
              <span class="text">
                <span class="row-title">{song.title}</span>
                {#if song.artist}<span class="row-artist">{song.artist}</span>{/if}
              </span>
              <span class="duration">{formatDuration(song.duration)}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if more > 0}
        <p class="more">{m.remote_more_songs({ count: more })}</p>
      {/if}
    </div>
  {/if}
</section>

<style>
  /* Over the whole Now Playing screen, clear of the system bars. */
  .remote {
    position: fixed;
    inset: 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: calc(env(safe-area-inset-top) + var(--space-2)) var(--page-gutter)
      calc(env(safe-area-inset-bottom) + var(--space-4));
    overflow-y: auto;
    background: var(--color-bg);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: calc(-1 * var(--space-2));
  }

  .heading {
    margin: 0;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-weight: var(--font-weight-bold);
  }

  .switch {
    margin-left: auto;
    flex: none;
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--color-border-focus);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--font-size-sm);
  }

  .message {
    margin: 0;
    color: var(--color-text-muted);
  }

  .error {
    margin: 0;
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .devices,
  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .device {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
  }

  .retry {
    align-self: flex-start;
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border-focus);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    font: inherit;
  }

  .song {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .art {
    width: 96px;
    flex: none;
  }

  .names {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .names p {
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
  }

  /* Only shows the position: the song is moved on the computer. */
  .bar {
    height: 4px;
    border-radius: var(--radius-pill);
    background: rgb(255 255 255 / 22%);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    background: var(--color-text);
  }

  .times {
    display: flex;
    justify-content: space-between;
    margin-top: var(--space-1);
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-variant-numeric: tabular-nums;
  }

  .transport {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-6);
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

  .icon:disabled,
  .play:disabled {
    opacity: 0.4;
  }

  .volume {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--color-text-muted);
  }

  .slider {
    flex: 1;
    height: 24px;
    margin: 0;
    appearance: none;
    background: transparent;
  }

  .slider::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--radius-pill);
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

  .queue-heading {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .origin,
  .more {
    margin: var(--space-1) 0 var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
  }

  .row-cover {
    width: 48px;
    flex: none;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .row-title,
  .row-artist {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .row-artist,
  .duration {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }

  /* The song playing on the computer. */
  .row.current .row-title {
    color: var(--color-accent-text);
    font-weight: var(--font-weight-bold);
  }
</style>
