<script lang="ts">
  // Pull to refresh (spec 4.5): at the top of the page, pulling down brings
  // the Parlo logo, which grows with the finger and pulses while `onrefresh`
  // runs. Only a clearly vertical pull counts, so the rows that scroll
  // sideways and the normal scrolling are left alone.
  import logo from "#lib/assets/logo-symbol.png";
  import { reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  /** How far the logo has to come down to refresh, and how far it can. */
  const THRESHOLD = 64;
  const MAX_PULL = 96;
  /** The logo moves this much for each pixel of the finger. */
  const RESISTANCE = 0.5;
  /** How far a finger moves before a pull and a scroll are told apart. */
  const SLOP = 10;
  /** The logo pulses at least this long, so a quick refresh is still seen. */
  const MIN_PULSE_MS = 700;

  let { onrefresh }: { onrefresh: () => Promise<unknown> } = $props();

  let anchor = $state<HTMLElement>();
  let pull = $state(0);
  let pulling = $state(false);
  let refreshing = $state(false);

  let progress = $derived(Math.min(pull / THRESHOLD, 1));

  async function refresh() {
    refreshing = true;
    pull = THRESHOLD;
    const pulse = new Promise((done) => setTimeout(done, MIN_PULSE_MS));
    await Promise.allSettled([onrefresh(), pulse]);
    refreshing = false;
    pull = 0;
  }

  $effect(() => {
    // The page scrolls inside the shell's content area.
    const scroller = anchor?.closest("main");
    if (!scroller) return;
    let start: { x: number; y: number } | null = null;

    function onTouchStart(event: TouchEvent) {
      start = null;
      pulling = false;
      if (refreshing || event.touches.length > 1 || scroller!.scrollTop > 0) return;
      start = { x: event.touches[0].clientX, y: event.touches[0].clientY };
    }

    function onTouchMove(event: TouchEvent) {
      if (!start) return;
      const dx = event.touches[0].clientX - start.x;
      const dy = event.touches[0].clientY - start.y;
      if (!pulling) {
        if (Math.abs(dx) < SLOP && Math.abs(dy) < SLOP) return;
        if (dy <= 0 || dy < Math.abs(dx) * 1.5) {
          start = null;
          return;
        }
        pulling = true;
      }
      if (event.cancelable) event.preventDefault();
      pull = Math.min(MAX_PULL, Math.max(0, (dy - SLOP) * RESISTANCE));
    }

    function onTouchEnd() {
      if (!start) return;
      start = null;
      if (!pulling) return;
      pulling = false;
      if (pull >= THRESHOLD) refresh();
      else pull = 0;
    }

    function onTouchCancel() {
      start = null;
      pulling = false;
      if (!refreshing) pull = 0;
    }

    scroller.addEventListener("touchstart", onTouchStart, { passive: true });
    scroller.addEventListener("touchmove", onTouchMove, { passive: false });
    scroller.addEventListener("touchend", onTouchEnd);
    scroller.addEventListener("touchcancel", onTouchCancel);
    return () => {
      scroller.removeEventListener("touchstart", onTouchStart);
      scroller.removeEventListener("touchmove", onTouchMove);
      scroller.removeEventListener("touchend", onTouchEnd);
      scroller.removeEventListener("touchcancel", onTouchCancel);
    };
  });
</script>

<div class="anchor" bind:this={anchor} aria-hidden="true">
  {#if pull > 0 || refreshing}
    <div
      class="mover"
      class:pulling
      style:transform={`translateY(${pull - 44}px) scale(${0.5 + progress / 2})`}
      style:opacity={progress}
    >
      <img src={logo} alt="" class:pulse={refreshing && !reducedMotion()} />
    </div>
  {/if}
</div>

<style>
  /* No height of its own: the logo comes down over the page. */
  .anchor {
    position: sticky;
    top: 0;
    z-index: 3;
    height: 0;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .mover {
    width: 44px;
    height: 44px;
    transition:
      transform 200ms ease-out,
      opacity 200ms ease-out;
  }

  .mover.pulling {
    transition: none;
  }

  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .pulse {
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.55;
      transform: scale(0.88);
    }

    50% {
      opacity: 1;
      transform: scale(1.1);
    }
  }
</style>
