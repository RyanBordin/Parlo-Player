<script lang="ts">
  // Compact song line: small cover, title, "artist • album". A click plays
  // (spec 5.1); the current song is in the accent color (spec 5.5).
  import Play from "@lucide/svelte/icons/play";
  import type { SongSummary } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  const COVER = 48;

  let { song, onplay }: { song: SongSummary; onplay: () => void } = $props();

  let current = $derived(player.track?.id === song.id);
  let subtitle = $derived([song.artist, song.album].filter(Boolean).join(" • "));
</script>

<button class="row" class:current type="button" onclick={onplay} aria-label={m.card_play({ name: song.title })}>
  <span class="cover">
    <Cover id={song.coverArt} size={COVER} radius="var(--radius-sm)" />
    <span class="play" aria-hidden="true"><Play size={18} fill="currentColor" /></span>
  </span>
  <span class="text">
    <span class="title" title={song.title}>{song.title}</span>
    {#if subtitle}
      <span class="subtitle" title={subtitle}>{subtitle}</span>
    {/if}
  </span>
</button>

<style>
  .row {
    width: 100%;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .row:hover {
    background: var(--color-surface);
  }

  .cover {
    position: relative;
    flex: none;
    width: 48px;
  }

  .play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    background: rgb(0 0 0 / 55%);
    color: var(--color-text);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .row:hover .play,
  .row:focus-visible .play {
    opacity: 1;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
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
