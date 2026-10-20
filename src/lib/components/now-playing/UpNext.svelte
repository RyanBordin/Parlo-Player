<script lang="ts">
  // "Up next" tab (spec 4.4): the whole queue, songs already played included,
  // scrolled to the current song when it opens. A click only jumps to the
  // song; the handle drags it to another place; X removes it.
  import GripVertical from "@lucide/svelte/icons/grip-vertical";
  import ListPlus from "@lucide/svelte/icons/list-plus";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import X from "@lucide/svelte/icons/x";
  import { moveInQueue, playQueueIndex, removeFromQueue, togglePause } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import SavePlaylistDialog from "#lib/components/now-playing/SavePlaylistDialog.svelte";
  import VirtualList from "#lib/components/VirtualList.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { getLocale } from "#lib/paraglide/runtime.js";
  import { player, queue } from "#lib/player.svelte.ts";
  import { songCoverSize, songRowHeight } from "#lib/settings/app-settings.svelte.ts";

  /** Near the list's top or bottom edge while dragging, it scrolls. */
  const EDGE = 48;
  const EDGE_SPEED = 12;

  // List density (spec 6.10).
  let rowHeight = $derived(songRowHeight());
  let coverSize = $derived(songCoverSize());
  let scroller = $state<HTMLElement>();
  let rows = $state<HTMLElement>();
  let error = $state<ErrorMessage | null>(null);
  let saving = $state(false);
  /** The song being dragged and where it would land. */
  let drag = $state<{ from: number; to: number } | null>(null);
  let pointerY = 0;
  let autoScroll = 0;

  let origin = $derived(
    queue.continuingFrom === null
      ? m.queue_playing_from({ label: queue.label })
      : queue.continuingFrom
        ? m.queue_continuing_from({ app: queue.continuingFrom })
        : m.queue_continuing_from_other(),
  );
  // "[context], [date]", e.g. "Albums, 10/18/2026".
  let suggestedName = $derived.by(() => {
    const context = queue.continuingFrom === null ? queue.label : origin;
    const date = new Intl.DateTimeFormat(getLocale(), { day: "2-digit", month: "2-digit", year: "numeric" }).format(
      new Date(),
    );
    return context ? `${context}, ${date}` : date;
  });
  let items = $derived(queue.tracks.map((track, index) => ({ track, index })));

  // On opening only: the current song in view, a bit below the top.
  let scrolledToCurrent = false;
  $effect(() => {
    if (scrolledToCurrent || !scroller || !rows || queue.currentIndex === null) return;
    scrolledToCurrent = true;
    const top = rows.offsetTop + queue.currentIndex * rowHeight;
    scroller.scrollTop = Math.max(0, top - scroller.clientHeight / 3);
  });

  function run(action: () => Promise<unknown>) {
    error = null;
    action().catch((e) => (error = describeError(e)));
  }

  function onRowClick(index: number) {
    // The song already playing pauses / resumes instead of starting over.
    if (index === queue.currentIndex) run(togglePause);
    else run(() => playQueueIndex(index));
  }

  function onRowKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "Enter") {
      event.preventDefault();
      onRowClick(index);
    }
  }

  // --- Dragging with the pointer (the browser's own drag and drop does not
  // work in the window on Windows: Tauri takes it to receive files). The
  // window is listened to, not the line: the list may stop drawing the line
  // while it scrolls. ---

  function positionAt(clientY: number): number {
    if (!rows) return 0;
    const index = Math.floor((clientY - rows.getBoundingClientRect().top) / rowHeight);
    return Math.min(Math.max(index, 0), queue.tracks.length - 1);
  }

  function startDrag(event: PointerEvent, index: number) {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    pointerY = event.clientY;
    drag = { from: index, to: index };
    window.addEventListener("pointermove", moveDrag);
    window.addEventListener("pointerup", dropDrag);
    window.addEventListener("pointercancel", cancelDrag);
    autoScroll = requestAnimationFrame(scrollNearEdges);
  }

  function moveDrag(event: PointerEvent) {
    if (!drag) return;
    pointerY = event.clientY;
    drag.to = positionAt(pointerY);
  }

  const dropDrag = () => endDrag();
  const cancelDrag = () => endDrag(true);

  function endDrag(cancel = false) {
    cancelAnimationFrame(autoScroll);
    window.removeEventListener("pointermove", moveDrag);
    window.removeEventListener("pointerup", dropDrag);
    window.removeEventListener("pointercancel", cancelDrag);
    const done = drag;
    drag = null;
    if (!done || cancel || done.from === done.to) return;
    // Moved here at once; the backend's queue event confirms it.
    const [track] = queue.tracks.splice(done.from, 1);
    queue.tracks.splice(done.to, 0, track);
    run(() => moveInQueue(done.from, done.to));
  }

  function scrollNearEdges() {
    if (!drag || !scroller) return;
    const box = scroller.getBoundingClientRect();
    if (pointerY < box.top + EDGE) scroller.scrollTop -= EDGE_SPEED;
    else if (pointerY > box.bottom - EDGE) scroller.scrollTop += EDGE_SPEED;
    drag.to = positionAt(pointerY);
    autoScroll = requestAnimationFrame(scrollNearEdges);
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (drag && event.key === "Escape") {
      event.stopPropagation();
      endDrag(true);
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="up-next" bind:this={scroller}>
  {#if queue.tracks.length === 0}
    <p class="empty">{m.up_next_empty()}</p>
  {:else}
    <header>
      <p class="origin" title={origin}>{origin}</p>
      <button type="button" class="save" onclick={() => (saving = true)}>
        <ListPlus size={16} />
        {m.up_next_save()}
      </button>
    </header>
    {#if error}
      <p class="error">{error.message}</p>
    {/if}

    <div class="rows" class:dragging={drag !== null} bind:this={rows} style:--cover={`${coverSize}px`}>
      <VirtualList {items} height={() => rowHeight} key={(item) => `${item.index}-${item.track.id}`}>
        {#snippet row({ track, index }: (typeof items)[number])}
          {@const current = index === queue.currentIndex}
          <div
            class="row"
            class:current
            class:lifted={drag?.from === index}
            role="button"
            tabindex="0"
            onclick={() => onRowClick(index)}
            onkeydown={(event) => onRowKeydown(event, index)}
          >
            <span class="cover">
              <Cover id={track.coverArt} size={coverSize} radius="var(--radius-sm)" />
              {#if current}
                <span class="state" aria-hidden="true">
                  {#if player.status === "playing"}<Play size={16} fill="currentColor" />{:else}<Pause
                      size={16}
                      fill="currentColor"
                    />{/if}
                </span>
              {/if}
            </span>
            <span class="text">
              <span class="title" title={track.title}>{track.title}</span>
              {#if track.artist}<span class="artist" title={track.artist}>{track.artist}</span>{/if}
            </span>
            <span class="duration">{formatDuration(track.duration)}</span>
            <button
              type="button"
              class="tool"
              onclick={(event) => {
                event.stopPropagation();
                run(() => removeFromQueue(index));
              }}
              aria-label={m.up_next_remove()}
              title={m.up_next_remove()}
            >
              <X size={16} />
            </button>
            <button
              type="button"
              class="tool handle"
              onpointerdown={(event) => startDrag(event, index)}
              onclick={(event) => event.stopPropagation()}
              aria-label={m.up_next_drag()}
              title={m.up_next_drag()}
            >
              <GripVertical size={16} />
            </button>
          </div>
        {/snippet}
      </VirtualList>
      {#if drag && drag.to !== drag.from}
        <!-- Where the song would land. -->
        <div class="drop" style:top={`${(drag.to + (drag.to > drag.from ? 1 : 0)) * rowHeight}px`}></div>
      {/if}
    </div>
  {/if}
</div>

{#if saving}
  <SavePlaylistDialog
    songIds={queue.tracks.map((t) => t.id)}
    {suggestedName}
    onclose={() => (saving = false)}
  />
{/if}

<style>
  .up-next {
    height: 100%;
    overflow-y: auto;
    padding-right: var(--space-2);
  }

  header {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) 0;
    background: var(--color-bg);
  }

  .origin {
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 14px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .save {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 32px;
    padding: 0 var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .save:hover {
    background: var(--color-surface-hover);
  }

  .rows {
    position: relative;
  }

  .rows.dragging {
    cursor: grabbing;
    user-select: none;
  }

  .row {
    height: 100%;
    display: grid;
    grid-template-columns: var(--cover) minmax(0, 1fr) auto 28px 28px;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-2);
    border-radius: var(--radius-md);
    cursor: pointer;
  }

  .row:hover {
    background: var(--color-surface);
  }

  .row.lifted {
    background: var(--color-surface-active);
    opacity: 0.6;
  }

  .cover {
    position: relative;
    width: var(--cover);
  }

  /* Spec 4.4: play / pause icon over the current song's cover. */
  .state {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    background: var(--color-scrim);
    color: var(--color-accent-text);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title,
  .artist {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: var(--font-weight-medium);
  }

  .current .title {
    color: var(--color-accent-text);
  }

  .artist {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .duration {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    font-variant-numeric: tabular-nums;
  }

  .tool {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    opacity: 0;
    cursor: pointer;
  }

  .tool:hover {
    color: var(--color-text);
  }

  .handle {
    cursor: grab;
    touch-action: none;
  }

  /* Remove and drag show on hover (spec 4.4). */
  .row:hover .tool,
  .tool:focus-visible,
  .lifted .tool {
    opacity: 1;
  }

  .drop {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    margin-top: -1px;
    border-radius: var(--radius-pill);
    background: var(--color-accent-text);
    pointer-events: none;
  }

  .empty {
    padding: var(--space-5) 0;
    color: var(--color-text-muted);
  }

  .error {
    margin-bottom: var(--space-2);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
