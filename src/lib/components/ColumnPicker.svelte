<script lang="ts">
  // "Columns" button of the Songs page (spec 6.3): shows or hides the
  // optional columns.
  import Columns3 from "@lucide/svelte/icons/columns-3";
  import { m } from "#lib/paraglide/messages.js";
  import { OPTIONAL_COLUMN_LABELS, OPTIONAL_COLUMNS, type OptionalColumn } from "#lib/track-columns.ts";

  let { columns, onchange }: { columns: OptionalColumn[]; onchange: (columns: OptionalColumn[]) => void } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();

  function toggle(column: OptionalColumn, shown: boolean) {
    onchange(shown ? [...columns, column] : columns.filter((c) => c !== column));
  }

  // Closes on a click outside the menu or on Escape.
  function onWindowClick(event: MouseEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeydown} />

<div class="picker" bind:this={root}>
  <button
    type="button"
    class="toggle"
    aria-haspopup="true"
    aria-expanded={open}
    aria-controls="column-picker"
    onclick={() => (open = !open)}
  >
    <Columns3 size={16} />
    {m.songs_columns()}
  </button>

  {#if open}
    <div class="menu" id="column-picker" role="group" aria-label={m.songs_columns()}>
      {#each OPTIONAL_COLUMNS as column (column)}
        <label class="item">
          <input
            type="checkbox"
            checked={columns.includes(column)}
            onchange={(event) => toggle(column, event.currentTarget.checked)}
          />
          {OPTIONAL_COLUMN_LABELS[column]()}
        </label>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    cursor: pointer;
  }

  .toggle:hover,
  .toggle[aria-expanded="true"] {
    background: var(--color-surface-hover);
  }

  .menu {
    position: absolute;
    top: calc(100% + var(--space-2));
    left: 0;
    z-index: 10;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    font-size: 14px;
    cursor: pointer;
  }

  .item:hover {
    background: var(--color-surface-hover);
  }

  .item input {
    accent-color: var(--color-accent);
  }
</style>
