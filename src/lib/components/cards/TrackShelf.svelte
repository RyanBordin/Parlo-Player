<script lang="ts">
  // Song section (spec 6.1): pages of 3 columns × 4 lines, < > one page at a
  // time, "Play all" plays the section's whole list from the start.
  import { playContext, type SongRow } from "#lib/api.ts";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import PlayAllButton from "#lib/components/cards/PlayAllButton.svelte";
  import TrackRow from "#lib/components/TrackRow.svelte";

  /** Songs per page: 3 columns × 4 lines. */
  const PAGE_SIZE = 12;

  let {
    title,
    row,
    loaded,
    empty,
    onerror,
  }: {
    title: string;
    row: SongRow | null;
    loaded: boolean;
    empty: string;
    onerror?: (error: unknown) => void;
  } = $props();

  let songs = $derived(row?.songs ?? []);
  let pages = $derived(
    Array.from({ length: Math.ceil(songs.length / PAGE_SIZE) }, (_, i) =>
      songs.slice(i * PAGE_SIZE, (i + 1) * PAGE_SIZE).map((song, j) => ({ song, index: i * PAGE_SIZE + j })),
    ),
  );

  // "Playing from" is the section's title (spec 6.1).
  function play(index: number) {
    if (row) playContext(row.allIds, index, title).catch((e) => onerror?.(e));
  }
</script>

<Carousel {title} count={songs.length} {loaded} {empty} paged>
  {#snippet actions()}
    <PlayAllButton onplay={() => play(0)} />
  {/snippet}

  {#each pages as page, p (p)}
    <ul class="page">
      {#each page as { song, index } (song.id)}
        <li><TrackRow {song} onplay={() => play(index)} /></li>
      {/each}
    </ul>
  {/each}
</Carousel>

<style>
  /* Filled column by column, like YouTube Music's song shelves. */
  .page {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    grid-template-rows: repeat(4, auto);
    grid-auto-flow: column;
    align-content: start;
    gap: var(--space-1) var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    min-width: 0;
  }
</style>
