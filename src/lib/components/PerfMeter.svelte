<script lang="ts">
  // Meter of the performance test mode (development only, spec 12.3): the
  // last opening, search and sort of the Songs page, and how smooth the last
  // scroll was. Not part of the app.
  import Gauge from "@lucide/svelte/icons/gauge";
  import X from "@lucide/svelte/icons/x";
  import { m } from "#lib/paraglide/messages.js";
  import { songsTiming, type ListChange } from "#lib/songs.svelte.ts";

  /** A scroll ends after this long without scroll events. */
  const SCROLL_IDLE_MS = 250;

  let open = $state(true);
  let scroll = $state<{ fps: number; worst: number } | null>(null);

  const ROWS: { kind: ListChange; label: () => string }[] = [
    { kind: "open", label: m.perf_open },
    { kind: "search", label: m.perf_search },
    { kind: "sort", label: m.perf_sort },
  ];

  // Frames drawn while scrolling: the average rate, and the slowest one.
  $effect(() => {
    let scrolling = false;
    let lastScroll = 0;
    let first = 0;
    let previous = 0;
    let frames = 0;
    let worst = 0;

    const frame = (now: number) => {
      if (now - lastScroll < SCROLL_IDLE_MS) {
        frames++;
        worst = Math.max(worst, now - previous);
        previous = now;
        requestAnimationFrame(frame);
        return;
      }
      scrolling = false;
      const duration = previous - first;
      if (frames > 1 && duration > 0) scroll = { fps: Math.round(((frames - 1) * 1000) / duration), worst: Math.round(worst) };
    };

    const onScroll = () => {
      lastScroll = performance.now();
      if (scrolling) return;
      scrolling = true;
      frames = 0;
      worst = 0;
      first = previous = lastScroll;
      requestAnimationFrame(frame);
    };

    // Scroll events do not bubble: listen while they go down (capture).
    document.addEventListener("scroll", onScroll, { capture: true, passive: true });
    return () => document.removeEventListener("scroll", onScroll, { capture: true });
  });

  function ms(value: number | null | undefined) {
    return value == null ? m.perf_none() : `${value} ms`;
  }
</script>

{#if open}
  <aside class="meter" aria-label={m.perf_title()}>
    <header>
      <span class="title"><Gauge size={14} /> {m.perf_title()}</span>
      <button type="button" onclick={() => (open = false)} aria-label={m.perf_hide()} title={m.perf_hide()}>
        <X size={14} />
      </button>
    </header>
    <table>
      <thead>
        <tr>
          <th></th>
          <th>{m.perf_query()}</th>
          <th>{m.perf_shown()}</th>
        </tr>
      </thead>
      <tbody>
        {#each ROWS as row (row.kind)}
          {@const timing = songsTiming[row.kind]}
          <tr>
            <th scope="row">{row.label()}</th>
            <td>{ms(timing?.queryMs)}</td>
            <td>{ms(timing?.shownMs)}</td>
          </tr>
        {/each}
        <tr>
          <th scope="row">{m.perf_scroll()}</th>
          <td colspan="2">{scroll ? m.perf_fps({ fps: scroll.fps, worst: scroll.worst }) : m.perf_none()}</td>
        </tr>
      </tbody>
    </table>
  </aside>
{:else}
  <button type="button" class="reopen" onclick={() => (open = true)} aria-label={m.perf_show()} title={m.perf_show()}>
    <Gauge size={16} />
  </button>
{/if}

<style>
  .meter,
  .reopen {
    position: absolute;
    right: var(--space-4);
    bottom: var(--space-4);
    z-index: 20;
    border: 1px solid var(--color-accent);
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
  }

  .meter {
    width: 300px;
    padding: var(--space-3);
    border-radius: var(--radius-lg);
    font-size: var(--font-size-sm);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }

  .title {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-accent-text);
    font-weight: var(--font-weight-semibold);
  }

  header button,
  .reopen {
    display: grid;
    place-items: center;
    padding: var(--space-1);
    border-radius: var(--radius-pill);
    color: var(--color-text);
    cursor: pointer;
  }

  header button {
    border: none;
    background: transparent;
  }

  .reopen {
    padding: var(--space-2);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-variant-numeric: tabular-nums;
  }

  th,
  td {
    padding: 2px 0;
    text-align: right;
    font-weight: normal;
  }

  thead th {
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
  }

  tbody th {
    color: var(--color-text-muted);
    text-align: left;
  }
</style>
