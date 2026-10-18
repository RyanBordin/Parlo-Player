<script lang="ts">
  // Horizontal row of cards (spec 6.1): title on the left, < > on the right,
  // smooth scrolling by one "page" of cards. Mouse wheel with Shift and
  // touchpads also scroll it.
  import type { Snippet } from "svelte";
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import { m } from "#lib/paraglide/messages.js";

  let {
    title,
    count,
    loaded = true,
    empty,
    children,
  }: {
    title: string;
    /** Number of cards, to know when the row is empty. */
    count: number;
    /** False while loading: no "empty" message yet. */
    loaded?: boolean;
    /** Short text explaining how to fill the row (spec 6.1). */
    empty: string;
    children: Snippet;
  } = $props();

  let track = $state<HTMLElement>();
  let atStart = $state(true);
  let atEnd = $state(true);

  function update() {
    if (!track) return;
    atStart = track.scrollLeft <= 1;
    atEnd = track.scrollLeft + track.clientWidth >= track.scrollWidth - 1;
  }

  function scrollPage(direction: 1 | -1) {
    // A bit less than the visible width, so one card stays in view.
    track?.scrollBy({ left: direction * track.clientWidth * 0.9, behavior: "smooth" });
  }

  // Again when the cards or the window size change.
  $effect(() => {
    void count;
    if (!track) return;
    update();
    const observer = new ResizeObserver(update);
    observer.observe(track);
    return () => observer.disconnect();
  });
</script>

<section class="carousel" aria-label={title}>
  <header>
    <h2 class="section-title">{title}</h2>
    {#if count > 0}
      <div class="arrows">
        <button type="button" onclick={() => scrollPage(-1)} disabled={atStart} aria-label={m.carousel_back()} title={m.carousel_back()}>
          <ChevronLeft />
        </button>
        <button type="button" onclick={() => scrollPage(1)} disabled={atEnd} aria-label={m.carousel_forward()} title={m.carousel_forward()}>
          <ChevronRight />
        </button>
      </div>
    {/if}
  </header>

  {#if count > 0}
    <div class="track" bind:this={track} onscroll={update}>
      {@render children()}
    </div>
  {:else if loaded}
    <p class="empty">{empty}</p>
  {:else}
    <div class="loading" aria-hidden="true"></div>
  {/if}
</section>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }

  header .section-title {
    margin: 0;
  }

  .arrows {
    display: flex;
    gap: var(--space-2);
  }

  .arrows button {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
  }

  .arrows button:not(:disabled):hover {
    background: var(--color-surface-hover);
  }

  .arrows button:disabled {
    color: var(--color-text-muted);
    opacity: 0.4;
    cursor: default;
  }

  .track {
    display: flex;
    gap: var(--space-5);
    overflow-x: auto;
    overscroll-behavior-x: contain;
    scroll-snap-type: x proximity;
    scrollbar-width: none;
    /* Room for the Play button's shadow and the focus ring. */
    padding: var(--space-1) 0 var(--space-2);
  }

  .track::-webkit-scrollbar {
    display: none;
  }

  .track > :global(*) {
    scroll-snap-align: start;
  }

  .empty {
    color: var(--color-text-muted);
    font-size: 14px;
  }

  /* Keeps the height while loading, so the page does not jump. */
  .loading {
    height: 120px;
  }
</style>
