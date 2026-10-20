<script lang="ts">
  // Song card: a click plays it in its row's list (spec 5.1); the artist's
  // name opens the artist. The current song is in the accent color (spec 5.5).
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

<div class="card" class:current style:width={fluid ? "100%" : `${size}px`}>
  <span class="cover">
    <Cover id={song.coverArt} {size} />
    <span class="play" class:large aria-hidden="true">
      <Play size={large ? 24 : 18} fill="currentColor" />
    </span>
  </span>
  {#if label}
    <span class="label">{label}</span>
  {/if}
  <!-- Its ::after covers the whole card, so a click anywhere plays. -->
  <button type="button" class="title" onclick={onplay} aria-label={m.card_play({ name: song.title })} title={song.title}>
    {song.title}
  </button>
  {#if song.artist && song.artistId}
    <a class="subtitle" href={`/artists/${encodeURIComponent(song.artistId)}`} title={song.artist}>{song.artist}</a>
  {:else if song.artist}
    <span class="subtitle" title={song.artist}>{song.artist}</span>
  {/if}
</div>

<style>
  .card {
    position: relative;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    color: var(--color-text);
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
    box-shadow: var(--shadow-button);
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
  .card:has(.title:focus-visible) .play {
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
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    text-align: left;
    cursor: pointer;
  }

  .title::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: var(--radius-md);
  }

  /* The focus ring goes around the whole card, not only the title. */
  .title:focus-visible {
    outline: none;
  }

  .title:focus-visible::after {
    outline: 2px solid var(--color-accent-text);
    outline-offset: 2px;
  }

  .current .title {
    color: var(--color-accent-text);
  }

  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    text-decoration: none;
  }

  /* Above the title's ::after, so it opens the artist instead of playing. */
  a.subtitle {
    position: relative;
    z-index: 1;
    align-self: flex-start;
    max-width: 100%;
  }

  a.subtitle:hover {
    color: var(--color-text);
    text-decoration: underline;
  }
</style>
