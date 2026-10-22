<script lang="ts">
  // Compact song line: small cover, title, "artist • album". A click plays
  // (spec 5.1); the artist's name opens the artist. The current song is in
  // the accent color (spec 5.5). "..." (on hover) and a right click open its
  // menu (spec 8.1).
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Play from "@lucide/svelte/icons/play";
  import type { SongSummary } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { openSongsMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  const COVER = 48;

  let {
    song,
    contextLabel,
    onplay,
  }: {
    song: SongSummary;
    /** "Playing from" if its menu starts a new queue. */
    contextLabel: string;
    onplay: () => void;
  } = $props();

  function onmenu(event: MouseEvent) {
    openSongsMenu(event, [song], { label: contextLabel });
  }

  let current = $derived(player.track?.id === song.id);
  let subtitle = $derived([song.artist, song.album].filter(Boolean).join(" • "));
  /** What follows the artist link. */
  let albumPart = $derived(song.album ? ` • ${song.album}` : "");
</script>

<!-- svelte-ignore a11y_no_static_element_interactions (the right click repeats the "..." button) -->
<div class="row" class:current oncontextmenu={onmenu}>
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
  <button type="button" class="more" onclick={onmenu} aria-label={m.player_more()} title={m.player_more()} aria-haspopup="menu">
    <Ellipsis size={18} />
  </button>
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

  /* Compact list density (spec 6.10). */
  :global(:root[data-density="compact"]) .row {
    padding-block: var(--space-1);
  }

  :global(:root[data-density="compact"]) .cover {
    width: 40px;
  }

  .play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    background: var(--color-scrim);
    color: var(--color-text);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .row:hover .play,
  .row:has(.title:focus-visible) .play {
    opacity: 1;
  }

  /* Above the title's ::after; shown on hover, like the Play cue. */
  .more {
    position: relative;
    z-index: 1;
    flex: none;
    margin-left: auto;
    display: grid;
    place-items: center;
    padding: var(--space-1);
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    opacity: 0;
    cursor: pointer;
  }

  .row:hover .more,
  .more:focus-visible {
    opacity: 1;
  }

  .more:hover {
    color: var(--color-text);
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
