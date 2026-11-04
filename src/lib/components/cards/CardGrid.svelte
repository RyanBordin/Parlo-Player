<script module lang="ts">
  // Width of the last grid drawn. A grid drawn again (Back) starts with it,
  // so it has its full height at once and the scroll position fits.
  let lastWidth = 0;
</script>

<script lang="ts" generics="T">
  // Grid of square cards (Albums, Artists): as many columns as fit the
  // width, and only the rows in view are drawn, so libraries with thousands
  // of items scroll smoothly.
  import type { Snippet } from "svelte";
  import VirtualList from "#lib/components/VirtualList.svelte";
  import { layout } from "#lib/shell/phone.svelte.ts";

  /** Narrowest card, and the space between cards and between rows. */
  const MIN_CARD = 160;
  const DESKTOP_GAP = 24;
  const ROW_GAP = 32;
  /** Phone (spec 4.5): always two cards side by side, a little closer. */
  const PHONE_COLUMNS = 2;
  const PHONE_GAP = 16;

  let {
    items,
    key,
    textHeight,
    card,
  }: {
    items: T[];
    key: (item: T) => string;
    /** Height of what is under the cover (margins and text lines). */
    textHeight: number;
    /** A card, given its cover width in pixels. */
    card: Snippet<[T, number]>;
  } = $props();

  let width = $state(lastWidth);
  $effect(() => {
    if (width > 0) lastWidth = width;
  });

  let GAP = $derived(layout.phone ? PHONE_GAP : DESKTOP_GAP);
  let columns = $derived(
    layout.phone ? PHONE_COLUMNS : Math.max(1, Math.floor((width + GAP) / (MIN_CARD + GAP))),
  );
  let cardWidth = $derived(width > 0 ? (width - GAP * (columns - 1)) / columns : MIN_CARD);
  let rowHeight = $derived(Math.ceil(cardWidth + textHeight + ROW_GAP));
  let rows = $derived(
    Array.from({ length: Math.ceil(items.length / columns) }, (_, index) => ({
      index,
      items: items.slice(index * columns, (index + 1) * columns),
    })),
  );
</script>

<div class="grid" bind:clientWidth={width}>
  {#if width > 0}
    <VirtualList items={rows} height={() => rowHeight} key={(row) => row.index}>
      {#snippet row(row: { index: number; items: T[] })}
        <div class="row" style:grid-template-columns={`repeat(${columns}, minmax(0, 1fr))`} style:column-gap={`${GAP}px`}>
          {#each row.items as item (key(item))}
            {@render card(item, Math.ceil(cardWidth))}
          {/each}
        </div>
      {/snippet}
    </VirtualList>
  {/if}
</div>

<style>
  .row {
    display: grid;
    align-items: start;
  }
</style>
