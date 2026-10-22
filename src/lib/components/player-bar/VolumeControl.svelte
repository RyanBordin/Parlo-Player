<script lang="ts">
  // Volume of the Player Bar (spec 4.3): only the icon (mute, low, high);
  // a click mutes. Hovering it (or the keyboard focus) opens a popover with a
  // vertical slider and the value in %. `nudge` (the mouse wheel over the
  // bar) shows the popover for a moment.
  import Volume1 from "@lucide/svelte/icons/volume-1";
  import Volume2 from "@lucide/svelte/icons/volume-2";
  import VolumeX from "@lucide/svelte/icons/volume-x";
  import { setVolume, toggleMute } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  /** Volume moved by the wheel and the arrow keys (spec 4.3). */
  const STEP = 5;
  /** The pointer may cross the gap to the popover in this long. */
  const CLOSE_DELAY_MS = 300;
  /** How long the popover stays after a turn of the wheel. */
  const FLASH_MS = 1000;

  let { onerror }: { onerror: (error: unknown) => void } = $props();

  let open = $state(false);
  let hovered = $state(false);
  let focused = $state(false);
  let dragging = $state(false);
  let track = $state<HTMLElement>();
  let closeTimer: ReturnType<typeof setTimeout> | undefined;

  let silent = $derived(player.muted || player.volume === 0);
  /** What the slider shows: 0 while muted. */
  let level = $derived(player.muted ? 0 : Math.round(player.volume));
  let shown = $derived(open || hovered || focused || dragging);

  function apply(volume: number) {
    const value = Math.max(0, Math.min(100, Math.round(volume)));
    player.volume = value;
    player.muted = false;
    setVolume(value).catch(onerror);
  }

  function closeLater(delay: number) {
    clearTimeout(closeTimer);
    closeTimer = setTimeout(() => (open = false), delay);
  }

  /** The mouse wheel: `steps` of 5% up (positive) or down. */
  export function nudge(steps: number) {
    apply(level + steps * STEP);
    open = true;
    closeLater(FLASH_MS);
  }

  function onEnter() {
    clearTimeout(closeTimer);
    hovered = true;
  }

  function onLeave() {
    hovered = false;
    open = true;
    closeLater(CLOSE_DELAY_MS);
  }

  function onFocusOut(event: FocusEvent) {
    const root = event.currentTarget as HTMLElement;
    if (!root.contains(event.relatedTarget as Node | null)) focused = false;
  }

  function valueAt(clientY: number): number {
    if (!track) return level;
    const box = track.getBoundingClientRect();
    return ((box.bottom - clientY) / box.height) * 100;
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    (event.currentTarget as HTMLElement).focus();
    dragging = true;
    apply(valueAt(event.clientY));
  }

  function onPointerMove(event: PointerEvent) {
    if (dragging) apply(valueAt(event.clientY));
  }

  function onPointerUp() {
    dragging = false;
  }

  // The arrows move the slider, not the song (spec 12.2 shortcuts wait).
  function onSliderKeydown(event: KeyboardEvent) {
    const change: Record<string, number> = {
      ArrowUp: STEP,
      ArrowRight: STEP,
      ArrowDown: -STEP,
      ArrowLeft: -STEP,
      PageUp: STEP * 2,
      PageDown: -STEP * 2,
    };
    if (event.key in change) apply(level + change[event.key]);
    else if (event.key === "Home") apply(0);
    else if (event.key === "End") apply(100);
    else return;
    event.preventDefault();
  }
</script>

<div
  class="volume"
  data-bar-control
  role="group"
  aria-label={m.player_volume()}
  onpointerenter={onEnter}
  onpointerleave={onLeave}
  onfocusin={() => (focused = true)}
  onfocusout={onFocusOut}
>
  <button
    class="icon"
    type="button"
    onclick={() => toggleMute().catch(onerror)}
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

  {#if shown}
    <div class="popover">
      <span class="value">{m.player_volume_value({ value: level })}</span>
      <div
        class="slider"
        role="slider"
        tabindex="0"
        aria-label={m.player_volume()}
        aria-orientation="vertical"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={level}
        aria-valuetext={m.player_volume_value({ value: level })}
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        onkeydown={onSliderKeydown}
      >
        <div class="track" bind:this={track}>
          <div class="fill" style:height={`${level}%`}></div>
          <div class="thumb" style:bottom={`${level}%`}></div>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .volume {
    position: relative;
    display: grid;
    place-items: center;
  }

  .icon {
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

  .icon:hover {
    color: var(--color-text);
  }

  /* Above the icon; the padding below bridges the gap for the pointer. */
  .popover {
    position: absolute;
    bottom: 100%;
    left: 50%;
    z-index: 20;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    width: 48px;
    padding: var(--space-3) 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
    transform: translate(-50%, calc(-1 * var(--space-2)));
  }

  .popover::after {
    content: "";
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    height: var(--space-3);
  }

  .value {
    color: var(--color-text);
    font-size: var(--font-size-xs);
    font-variant-numeric: tabular-nums;
  }

  .slider {
    width: 24px;
    height: 112px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    cursor: pointer;
    touch-action: none;
  }

  .slider:focus-visible {
    outline: 2px solid var(--color-accent-text);
    outline-offset: 2px;
  }

  .track {
    position: relative;
    width: 4px;
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--color-border);
  }

  .fill {
    position: absolute;
    bottom: 0;
    left: 0;
    width: 100%;
    border-radius: var(--radius-pill);
    background: var(--color-text);
  }

  .thumb {
    position: absolute;
    left: 50%;
    width: 12px;
    height: 12px;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    transform: translate(-50%, 50%);
  }
</style>
