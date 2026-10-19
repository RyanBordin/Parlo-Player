<script lang="ts">
  // Grid of album cards (spec 6.4), virtualized.
  import type { AlbumCard as AlbumCardData } from "#lib/api.ts";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import CardGrid from "#lib/components/cards/CardGrid.svelte";

  /** Under the cover: margin, title and artist lines. */
  const TEXT_HEIGHT = 48;

  let {
    albums,
    onerror,
  }: {
    albums: (AlbumCardData & { starred?: boolean })[];
    onerror?: (error: unknown) => void;
  } = $props();
</script>

<CardGrid items={albums} key={(album) => album.id} textHeight={TEXT_HEIGHT}>
  {#snippet card(album, size)}
    <AlbumCard {album} {size} fluid {onerror} />
  {/snippet}
</CardGrid>
