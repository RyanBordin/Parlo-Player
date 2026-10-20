<script lang="ts">
  // Artist card (square, spec 6.6): photo, or the latest album's cover when
  // the server has no photo. A click opens the artist.
  import Heart from "@lucide/svelte/icons/heart";
  import type { ArtistCard } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
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
</script>

<a class="card" href={`/artists/${encodeURIComponent(artist.id)}`} style:width={fluid ? "100%" : `${size}px`} title={artist.name}>
  <span class="cover">
    <Cover id={artist.coverArt} fallback={artist.fallbackCoverArt} {size} />
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

<style>
  .card {
    flex: none;
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

  /* Discreet favorite badge, as on album cards (spec 6.4). */
  .badge {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
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
    gap: 2px;
  }

  .label {
    color: var(--color-accent-text);
    font-size: 11px;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .name {
    overflow: hidden;
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .card:hover .name {
    text-decoration: underline;
  }
</style>
