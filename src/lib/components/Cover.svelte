<script lang="ts">
  // Album / song cover, used on every screen. Fills its container's width as
  // a square; without a cover, or if it fails to load, shows a neutral
  // placeholder (spec 12.6).
  import Disc3 from "@lucide/svelte/icons/disc-3";
  import { coverUrl } from "#lib/covers.ts";

  let {
    id,
    size,
    alt = "",
    radius = "var(--radius-md)",
  }: {
    /** `coverArt` from the library; empty shows the placeholder. */
    id: string | null | undefined;
    /** Width shown, in CSS pixels (picks the image size to ask for). */
    size: number;
    alt?: string;
    radius?: string;
  } = $props();

  let src = $derived(id ? coverUrl(id, size) : null);
  // The URL that failed, so a new cover starts without the old error.
  let failedSrc = $state<string | null>(null);
  let showImage = $derived(src !== null && src !== failedSrc);
</script>

<div class="cover" style:border-radius={radius}>
  {#if showImage}
    <img {src} {alt} loading="lazy" decoding="async" draggable="false" onerror={() => (failedSrc = src)} />
  {:else}
    <div class="placeholder" role={alt ? "img" : undefined} aria-label={alt || undefined}>
      <Disc3 />
    </div>
  {/if}
</div>

<style>
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 1;
    overflow: hidden;
    background: var(--color-surface);
  }

  img {
    width: 100%;
    height: 100%;
    display: block;
    object-fit: cover;
  }

  .placeholder {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--color-border-focus);
  }

  /* The icon grows with the cover. */
  .placeholder :global(svg) {
    width: 40%;
    height: 40%;
  }
</style>
