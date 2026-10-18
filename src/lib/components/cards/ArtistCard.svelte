<script lang="ts">
  // Artist card (square, spec 6.6): photo, or the latest album's cover when
  // the server has no photo. A click opens the artist.
  import type { ArtistCard } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";

  let { artist, size = 160 }: { artist: ArtistCard; size?: number } = $props();
</script>

<a class="card" href={`/artists/${encodeURIComponent(artist.id)}`} style:width={`${size}px`} title={artist.name}>
  <span class="cover">
    <Cover id={artist.coverArt} fallback={artist.fallbackCoverArt} {size} />
  </span>
  <span class="name">{artist.name}</span>
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
    display: block;
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
