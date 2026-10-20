<script lang="ts">
  // List to reorder and hide items (spec 6.10: Home sections, sidebar): a
  // handle to drag, or the arrow keys on the handle; a switch per item.
  // While dragging, the list shows the new order live. `fixed` items close
  // the list: they cannot move nor be hidden (Settings in the sidebar).
  import GripVertical from "@lucide/svelte/icons/grip-vertical";
  import Lock from "@lucide/svelte/icons/lock";
  import { tick } from "svelte";
  import { flip } from "svelte/animate";
  import type { LayoutItem } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { reducedMotion } from "#lib/settings/app-settings.svelte.ts";
  import SwitchSetting from "#lib/settings/SwitchSetting.svelte";

  type Icon = typeof GripVertical;
  type Item = { id: string; label: string; icon?: Icon; visible: boolean };

  let {
    items,
    onchange,
    minVisible = 0,
    minVisibleHint,
    fixed = [],
  }: {
    items: Item[];
    onchange: (items: LayoutItem[]) => void;
    /** Visible items that must stay: their switch is off-limits. */
    minVisible?: number;
    /** Tooltip of a switch that cannot be turned off. */
    minVisibleHint?: string;
    fixed?: { id: string; label: string; icon?: Icon }[];
  } = $props();

  let list = $state<HTMLElement>();
  /** The item dragged: where it was and where it is now. */
  let drag = $state<{ from: number; to: number } | null>(null);
  /** Said by screen readers after a move. */
  let announcement = $state("");
  const handles: Record<string, HTMLButtonElement> = {};

  let shown = $derived(drag ? moved(items, drag.from, drag.to) : items);
  let visibleCount = $derived(items.filter((item) => item.visible).length);

  function moved(list: Item[], from: number, to: number): Item[] {
    const next = [...list];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    return next;
  }

  function emit(next: Item[]) {
    onchange(next.map(({ id, visible }) => ({ id, visible })));
  }

  function move(from: number, to: number) {
    if (to < 0 || to >= items.length || to === from) return;
    const item = items[from];
    emit(moved(items, from, to));
    announcement = m.settings_moved({ name: item.label, position: to + 1, total: items.length });
    // The row moved in the page: its handle keeps the focus.
    tick().then(() => handles[item.id]?.focus());
  }

  function toggle(id: string, visible: boolean) {
    emit(items.map((item) => (item.id === id ? { ...item, visible } : item)));
  }

  function onHandleKeydown(event: KeyboardEvent, index: number) {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      move(index, index + (event.key === "ArrowUp" ? -1 : 1));
    }
  }

  // --- Dragging with the pointer. Every row has the same height, so the
  // pointer's position tells the place directly.

  function startDrag(event: PointerEvent, index: number) {
    if (event.button !== 0) return;
    event.preventDefault();
    drag = { from: index, to: index };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onDrop);
    window.addEventListener("pointercancel", onCancel);
  }

  function onMove(event: PointerEvent) {
    const first = list?.querySelector<HTMLElement>(".item");
    if (!drag || !list || !first) return;
    const index = Math.floor((event.clientY - list.getBoundingClientRect().top) / first.offsetHeight);
    drag.to = Math.max(0, Math.min(items.length - 1, index));
  }

  function endDrag(apply: boolean) {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onDrop);
    window.removeEventListener("pointercancel", onCancel);
    const done = drag;
    drag = null;
    if (apply && done) move(done.from, done.to);
  }

  const onDrop = () => endDrag(true);
  const onCancel = () => endDrag(false);

  function onWindowKeydown(event: KeyboardEvent) {
    if (drag && event.key === "Escape") {
      event.preventDefault();
      endDrag(false);
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<ul class="list" class:dragging={drag !== null} bind:this={list}>
  {#each shown as item, index (item.id)}
    {@const locked = item.visible && visibleCount <= minVisible}
    <li class="item" class:lifted={drag !== null && item.id === items[drag.from].id} animate:flip={{ duration: reducedMotion() ? 0 : 150 }}>
      <button
        type="button"
        class="handle"
        bind:this={handles[item.id]}
        onpointerdown={(event) => startDrag(event, index)}
        onkeydown={(event) => onHandleKeydown(event, index)}
        aria-label={m.settings_drag({ name: item.label })}
        title={m.settings_drag({ name: item.label })}
      >
        <GripVertical size={16} />
      </button>
      {#if item.icon}<item.icon size={16} />{/if}
      <span class="label" class:hidden-item={!item.visible}>{item.label}</span>
      <SwitchSetting
        checked={item.visible}
        disabled={locked}
        title={locked ? minVisibleHint : undefined}
        label={m.settings_show_item({ name: item.label })}
        onchange={(visible) => toggle(item.id, visible)}
      />
    </li>
  {/each}
  {#each fixed as item (item.id)}
    <li class="item fixed">
      <span class="handle" aria-hidden="true"><Lock size={14} /></span>
      {#if item.icon}<item.icon size={16} />{/if}
      <span class="label">{item.label}</span>
      <span class="note">{m.settings_always_visible()}</span>
    </li>
  {/each}
</ul>
<p class="visually-hidden" aria-live="polite">{announcement}</p>

<style>
  .list {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    list-style: none;
  }

  .list.dragging {
    cursor: grabbing;
    user-select: none;
  }

  .item {
    height: 48px;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0 var(--space-4) 0 var(--space-2);
    color: var(--color-text);
    font-size: 14px;
  }

  .item + .item {
    border-top: 1px solid var(--color-border);
  }

  .item.lifted {
    border-radius: var(--radius-md);
    background: var(--color-surface-active);
  }

  .handle {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text-muted);
    cursor: grab;
    touch-action: none;
  }

  button.handle:hover {
    background: var(--color-surface-hover);
    color: var(--color-text);
  }

  .fixed .handle {
    cursor: default;
  }

  .label {
    flex: auto;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .label.hidden-item {
    color: var(--color-text-muted);
  }

  .note {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
