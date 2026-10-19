<script lang="ts">
  // Header of album and playlist pages (spec 6.5): the cover enlarged and
  // heavily blurred behind a dark gradient, the sharp cover on the left, a big
  // title and the metadata lines.
  import type { Snippet } from "svelte";
  import Cover from "#lib/components/Cover.svelte";
  import { coverUrl } from "#lib/covers.ts";

  /** Width of the sharp cover. */
  const COVER = 208;

  let {
    coverArt,
    title,
    meta,
    children,
  }: {
    coverArt: string | null;
    title: string;
    /** Lines under the title. */
    meta: Snippet;
    /** Under the header content: the action bar. */
    children?: Snippet;
  } = $props();

  // A small image is enough: it is blurred anyway.
  let backdrop = $derived(coverArt ? coverUrl(coverArt, 128) : null);
</script>

<header class="hero">
  {#if backdrop}
    <img class="backdrop" src={backdrop} alt="" aria-hidden="true" />
  {/if}
  <div class="shade"></div>

  <div class="content">
    <div class="art">
      <Cover id={coverArt} size={COVER} radius="var(--radius-lg)" />
    </div>
    <div class="info">
      <h1 class="title">{title}</h1>
      {@render meta()}
    </div>
  </div>
  {#if children}
    <div class="below">{@render children()}</div>
  {/if}
</header>

<style>
  .hero {
    position: relative;
    overflow: hidden;
    padding: var(--space-6) var(--page-gutter) var(--space-5);
  }

  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(64px) saturate(1.3);
    opacity: 0.6;
    transform: scale(1.4);
  }

  /* Dark gradient down to the page's black. */
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, rgb(0 0 0 / 15%), rgb(0 0 0 / 55%) 60%, var(--color-bg));
  }

  .content {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: var(--space-6);
  }

  .art {
    flex: none;
    width: 208px;
    border-radius: var(--radius-lg);
    box-shadow: 0 16px 40px rgb(0 0 0 / 45%);
  }

  .info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding-bottom: var(--space-2);
  }

  .title {
    display: -webkit-box;
    overflow: hidden;
    font-size: var(--font-size-title);
    font-weight: var(--font-weight-bold);
    letter-spacing: -0.02em;
    line-height: 1.1;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .below {
    position: relative;
  }

  /* Metadata lines given by the page. */
  .info :global(.meta) {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text);
    font-size: 14px;
    font-weight: var(--font-weight-medium);
  }

  .info :global(.meta.secondary) {
    color: var(--color-text-muted);
  }

  .info :global(.meta a) {
    color: inherit;
    font-weight: var(--font-weight-semibold);
    text-decoration: none;
  }

  .info :global(.meta a:hover) {
    text-decoration: underline;
  }

  /* Type label in the accent color (spec 1.1). */
  .info :global(.kind) {
    color: var(--color-accent-text);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .info :global(.pill) {
    padding: 2px var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: rgb(0 0 0 / 30%);
    color: var(--color-text);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }

  .info :global(a.pill:hover) {
    background: var(--color-surface-hover);
    text-decoration: none;
  }
</style>
