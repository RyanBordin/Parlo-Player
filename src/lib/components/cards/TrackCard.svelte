<script lang="ts">
  // Song card: a click plays it in its row's list (spec 5.1). The current
  // song is in the accent color (spec 5.5).
  import Play from "@lucide/svelte/icons/play";
  import type { SongSummary } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  let {
    song,
    size = 160,
    large = false,
    label,
    fluid = false,
    onplay,
  }: {
    song: SongSummary;
    /** Cover width in CSS pixels. */
    size?: number;
    /** "Listen again" style: bigger Play icon. */
    large?: boolean;
    label?: string;
    /** Fills its grid column instead of `size` pixels. */
    fluid?: boolean;
    onplay: () => void;
  } = $props();

  let current = $derived(player.track?.id === song.id);
</script>

<button
  class="card"
  class:current
  type="button"
  style:width={fluid ? "100%" : `${size}px`}
  onclick={onplay}
  aria-label={m.card_play({ name: song.title })}
>
  <span class="cover">
    <Cover id={song.coverArt} {size} />
    <span class="play" class:large aria-hidden="true">
      <Play size={large ? 24 : 18} fill="currentColor" />
    </span>
  </span>
  {#if label}
    <span class="label">{label}</span>
  {/if}
  <span class="title" title={song.title}>{song.title}</span>
  {#if song.artist}
    <span class="subtitle" title={song.artist}>{song.artist}</span>
  {/if}
</button>

<style>
  .card {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .cover {
    position: relative;
    display: block;
    margin-bottom: var(--space-2);
  }

  /* A visual cue only: the whole card plays. */
  .play {
    position: absolute;
    right: var(--space-2);
    bottom: var(--space-2);
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    color: var(--color-text-inverse);
    box-shadow: 0 6px 16px rgb(0 0 0 / 50%);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .play.large {
    inset: 50% auto auto 50%;
    width: 56px;
    height: 56px;
    transform: translate(-50%, -50%);
  }

  .card:hover .play,
  .card:focus-visible .play {
    opacity: 1;
  }

  .label {
    color: var(--color-accent-text);
    font-size: 11px;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .title,
  .subtitle {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
  }

  .current .title {
    color: var(--color-accent-text);
  }

  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }
</style>
