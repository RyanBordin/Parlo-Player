<script lang="ts">
  // Artist card (spec 6.6): round photo, the size of an album card, or the
  // latest album's cover when the server has no photo; the name centered
  // under it. A click opens the artist; "..." and a right click open its
  // menu (spec 8.1).
  import Heart from "@lucide/svelte/icons/heart";
  import type { ArtistCard } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import MoreButton from "#lib/components/cards/MoreButton.svelte";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { artistMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";

  let {
    artist,
    size = 160,
    label,
    fluid = false,
  }: {
    /** `starred` shows the favorite badge (Artists grid). */
    artist: ArtistCard & { starred?: boolean };
    size?: number;
    /** Small type label above the name (rows that mix kinds). */
    label?: string;
    /** Fills its grid column instead of `size` pixels. */
    fluid?: boolean;
  } = $props();

  function onmenu(event: MouseEvent) {
    openMenu(event, () => artistMenu(artist));
  }
</script>

<!-- The link and "..." side by side: a button cannot be inside a link. -->
<div class="card" style:width={fluid ? "100%" : `${size}px`} oncontextmenu={onmenu} role="group" aria-label={artist.name}>
  <a class="link" href={`/artists/${encodeURIComponent(artist.id)}`} title={artist.name}>
  <span class="cover">
    <Cover id={artist.coverArt} fallback={artist.fallbackCoverArt} {size} radius="50%" />
    {#if artist.starred}
      <span class="badge" role="img" aria-label={m.card_starred()} title={m.card_starred()}>
        <Heart size={12} fill="currentColor" />
      </span>
    {/if}
  </span>
  <span class="text">
    {#if label}
      <span class="label">{label}</span>
    {/if}
    <span class="name">{artist.name}</span>
  </span>
  </a>
  <!-- A square over the cover, so the button sits inside the circle. -->
  <span class="over-cover"><MoreButton {onmenu} round /></span>
</div>

<style>
  .card {
    position: relative;
    flex: none;
    min-width: 0;
  }

  .over-cover {
    position: absolute;
    inset: 0 0 auto 0;
    aspect-ratio: 1;
    pointer-events: none;
  }

  .over-cover :global(.more) {
    pointer-events: auto;
  }

  .link {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
    color: var(--color-text);
    text-decoration: none;
  }

  .cover {
    position: relative;
    display: block;
  }

  /* Discreet favorite badge, as on album cards (spec 6.4), inside the circle. */
  .badge {
    position: absolute;
    top: 8%;
    right: 8%;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    background: var(--color-scrim);
    color: var(--color-text);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    text-align: center;
  }

  .label {
    color: var(--color-accent-text);
    font-size: 11px;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .name {
    max-width: 100%;
    overflow: hidden;
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .link:hover .name {
    text-decoration: underline;
  }
</style>
