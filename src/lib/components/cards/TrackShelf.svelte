<script lang="ts">
  // Song section (spec 6.1): pages of 3 columns × 4 lines, < > one page at a
  // time, "Play all" plays the section's whole list from the start.
  import { playContext, type SongRow } from "#lib/api.ts";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import PlayAllButton from "#lib/components/cards/PlayAllButton.svelte";
  import TrackRow from "#lib/components/TrackRow.svelte";
  import { layout } from "#lib/shell/phone.svelte.ts";

  /** Lines per page (spec 6.1, 4.4). */
  const LINES = 4;

  let {
    title,
    row,
    loaded,
    empty,
    columns = 3,
    playAll = true,
    large = false,
    onerror,
  }: {
    title: string;
    row: SongRow | null;
    loaded: boolean;
    empty: string;
    /** Columns per page: 3 on the Home, fewer in a narrow column. */
    columns?: number;
    /** The "Play all" button (Home sections). */
    playAll?: boolean;
    /** The phone Now Playing drawer's bigger lines (spec 4.5). */
    large?: boolean;
    onerror?: (error: unknown) => void;
  } = $props();

  // Phone (spec 4.5): one column per page; the next page peeks at the edge.
  let shownColumns = $derived(layout.phone ? 1 : columns);
  let pageSize = $derived(shownColumns * LINES);

  let songs = $derived(row?.songs ?? []);
  let pages = $derived(
    Array.from({ length: Math.ceil(songs.length / pageSize) }, (_, i) =>
      songs.slice(i * pageSize, (i + 1) * pageSize).map((song, j) => ({ song, index: i * pageSize + j })),
    ),
  );

  // "Playing from" is the section's title (spec 6.1).
  function play(index: number) {
    if (row) playContext(row.allIds, index, title).catch((e) => onerror?.(e));
  }
</script>

<Carousel {title} count={songs.length} {loaded} {empty} paged>
  {#snippet actions()}
    {#if playAll}<PlayAllButton onplay={() => play(0)} />{/if}
  {/snippet}

  {#each pages as page, p (p)}
    <ul class="page" style:--columns={shownColumns}>
      {#each page as { song, index } (song.id)}
        <li><TrackRow {song} contextLabel={title} onplay={() => play(index)} {large} /></li>
      {/each}
    </ul>
  {/each}
</Carousel>

<style>
  /* Filled column by column, like YouTube Music's song shelves. */
  .page {
    display: grid;
    grid-template-columns: repeat(var(--columns), minmax(0, 1fr));
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
