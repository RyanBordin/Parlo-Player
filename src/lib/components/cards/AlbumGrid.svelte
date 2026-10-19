<script module lang="ts">
  // Width of the last grid drawn. A grid drawn again (Back) starts with it,
  // so it has its full height at once and the scroll position fits.
  let lastWidth = 0;
</script>

<script lang="ts">
  // Grid of album cards (spec 6.4): as many columns as fit the width, and
  // only the rows in view are drawn, so libraries with thousands of albums
  // scroll smoothly.
  import type { AlbumCard as AlbumCardData } from "#lib/api.ts";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import VirtualList from "#lib/components/VirtualList.svelte";

  /** Narrowest card, and the space between cards and between rows. */
  const MIN_CARD = 160;
  const GAP = 24;
  const ROW_GAP = 32;
  /** Under the cover: margin, title and artist lines. */
  const TEXT_HEIGHT = 48;

  let {
    albums,
    onerror,
  }: {
    albums: (AlbumCardData & { starred?: boolean })[];
    onerror?: (error: unknown) => void;
  } = $props();

  let width = $state(lastWidth);
  $effect(() => {
    if (width > 0) lastWidth = width;
  });

  let columns = $derived(Math.max(1, Math.floor((width + GAP) / (MIN_CARD + GAP))));
  let cardWidth = $derived(width > 0 ? (width - GAP * (columns - 1)) / columns : MIN_CARD);
  let rowHeight = $derived(Math.ceil(cardWidth + TEXT_HEIGHT + ROW_GAP));
  let rows = $derived(
    Array.from({ length: Math.ceil(albums.length / columns) }, (_, index) => ({
      index,
      albums: albums.slice(index * columns, (index + 1) * columns),
    })),
  );
</script>

<div class="grid" bind:clientWidth={width}>
  {#if width > 0}
    <VirtualList items={rows} height={() => rowHeight} key={(row) => row.index}>
      {#snippet row(row: { index: number; albums: typeof albums })}
        <div class="row" style:grid-template-columns={`repeat(${columns}, minmax(0, 1fr))`}>
          {#each row.albums as album (album.id)}
            <AlbumCard {album} size={Math.ceil(cardWidth)} fluid {onerror} />
          {/each}
        </div>
      {/snippet}
    </VirtualList>
  {/if}
</div>

<style>
  .row {
    display: grid;
    column-gap: 24px;
    align-items: start;
  }
</style>
