<script lang="ts">
  // Compact song line: small cover, title, "artist • album". A click plays
  // (spec 5.1); the artist's name opens the artist. The current song is in
  // the accent color (spec 5.5).
  import Play from "@lucide/svelte/icons/play";
  import type { SongSummary } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  const COVER = 48;

  let { song, onplay }: { song: SongSummary; onplay: () => void } = $props();

  let current = $derived(player.track?.id === song.id);
  let subtitle = $derived([song.artist, song.album].filter(Boolean).join(" • "));
  /** What follows the artist link. */
  let albumPart = $derived(song.album ? ` • ${song.album}` : "");
</script>

<div class="row" class:current>
  <span class="cover">
    <Cover id={song.coverArt} size={COVER} radius="var(--radius-sm)" />
    <span class="play" aria-hidden="true"><Play size={18} fill="currentColor" /></span>
  </span>
  <span class="text">
    <!-- Its ::after covers the whole line, so a click anywhere plays. -->
    <button type="button" class="title" onclick={onplay} aria-label={m.card_play({ name: song.title })} title={song.title}>
      {song.title}
    </button>
    {#if subtitle}
      <span class="subtitle" title={subtitle}>
        {#if song.artist && song.artistId}
          <a href={`/artists/${encodeURIComponent(song.artistId)}`}>{song.artist}</a>{albumPart}
        {:else}
          {subtitle}
        {/if}
      </span>
    {/if}
  </span>
</div>

<style>
  .row {
    position: relative;
    width: 100%;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2);
    border-radius: var(--radius-md);
    color: var(--color-text);
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
  .row:has(.title:focus-visible) .play {
    opacity: 1;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .title,
  .subtitle {
    max-width: 100%;
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

  /* The focus ring goes around the whole line, not only the title. */
  .title:focus-visible {
    outline: none;
  }

  .title:focus-visible::after {
    outline: 2px solid var(--color-accent-text);
    outline-offset: -2px;
  }

  .current .title {
    color: var(--color-accent-text);
  }

  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  /* Above the title's ::after, so it opens the artist instead of playing. */
  .subtitle a {
    position: relative;
    z-index: 1;
    color: inherit;
    text-decoration: none;
  }

  .subtitle a:hover {
    color: var(--color-text);
    text-decoration: underline;
  }
</style>
