<script lang="ts" generics="T">
  // Draws only the lines in view (plus a margin), so lists of tens of
  // thousands of lines stay fast (spec 6.5, 12.3). It uses the scrolling of
  // the page around it (the app's content area), so a header above the list
  // scrolls away normally. Lines can have different heights.
  import type { Snippet } from "svelte";

  let {
    items,
    height,
    key,
    row,
    overscan = 600,
  }: {
    items: T[];
    /** Height of a line, in pixels. */
    height: (item: T) => number;
    key: (item: T) => string | number;
    row: Snippet<[T, number]>;
    /** Extra pixels drawn above and below the view. */
    overscan?: number;
  } = $props();

  let container = $state<HTMLElement>();
  let scroller: HTMLElement | null = null;
  /** Visible window, in pixels from the top of the list. */
  let viewTop = $state(0);
  let viewHeight = $state(0);

  // Top of each line, and the total height.
  let offsets = $derived.by(() => {
    const tops = new Array<number>(items.length);
    let y = 0;
    for (let i = 0; i < items.length; i++) {
      tops[i] = y;
      y += height(items[i]);
    }
    return { tops, total: y };
  });

  let visible = $derived.by(() => {
    const { tops } = offsets;
    const from = viewTop - overscan;
    const to = viewTop + viewHeight + overscan;
    // First line that ends below `from` (binary search).
    let low = 0;
    let high = items.length;
    while (low < high) {
      const mid = (low + high) >> 1;
      if (tops[mid] + height(items[mid]) <= from) low = mid + 1;
      else high = mid;
    }
    const lines: { item: T; index: number; top: number }[] = [];
    for (let i = low; i < items.length && tops[i] < to; i++) {
      lines.push({ item: items[i], index: i, top: tops[i] });
    }
    return lines;
  });

  function scrollParent(node: HTMLElement): HTMLElement {
    for (let parent = node.parentElement; parent; parent = parent.parentElement) {
      const overflow = getComputedStyle(parent).overflowY;
      if (overflow === "auto" || overflow === "scroll") return parent;
    }
    return document.documentElement;
  }

  function measure() {
    if (!container || !scroller) return;
    const listTop = container.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    viewTop = -listTop;
    viewHeight = scroller.clientHeight;
  }

  $effect(() => {
    if (!container) return;
    scroller = scrollParent(container);
    let frame = 0;
    const onScroll = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(measure);
    };
    measure();
    scroller.addEventListener("scroll", onScroll, { passive: true });
    const observer = new ResizeObserver(measure);
    observer.observe(scroller);
    observer.observe(container);
    return () => {
      cancelAnimationFrame(frame);
      scroller?.removeEventListener("scroll", onScroll);
      observer.disconnect();
    };
  });
</script>

<div class="virtual" bind:this={container} style:height={`${offsets.total}px`}>
  {#each visible as line (key(line.item))}
    <div class="line" style:transform={`translateY(${line.top}px)`} style:height={`${height(line.item)}px`}>
      {@render row(line.item, line.index)}
    </div>
  {/each}
</div>

<style>
  .virtual {
    position: relative;
  }

  .line {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }
</style>
