<script lang="ts">
  // The right card for an item of a grid: album or artist.
  import type { HomeItem } from "#lib/api.ts";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";

  let {
    item,
    size = 160,
    fluid = false,
    onerror,
  }: {
    item: HomeItem;
    size?: number;
    fluid?: boolean;
    onerror?: (error: unknown) => void;
  } = $props();
</script>

{#if item.kind === "album"}
  <AlbumCard album={item} {size} {fluid} {onerror} />
{:else}
  <ArtistCard artist={item} {size} {fluid} />
{/if}
