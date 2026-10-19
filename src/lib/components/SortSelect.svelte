<script lang="ts" generics="T extends string">
  // "Sort by" of the Albums and Songs pages (spec 6.4, 6.3), with the
  // "Draw again" button while the order is random.
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import { m } from "#lib/paraglide/messages.js";

  let {
    value,
    options,
    onchange,
    otherLabel,
    onredraw,
  }: {
    /** The option chosen; `null` when the order comes from elsewhere. */
    value: T | null;
    options: { value: T; label: string }[];
    onchange: (value: T) => void;
    /** Shown when `value` is `null` (e.g. sorted by a column header). */
    otherLabel?: string;
    /** Shown as "Draw again" (only while the random order is chosen). */
    onredraw?: () => void;
  } = $props();
</script>

<label class="sort">
  <span>{m.albums_sort_label()}</span>
  <select value={value ?? ""} onchange={(e) => onchange(e.currentTarget.value as T)}>
    {#if value === null}
      <option value="" disabled>{otherLabel}</option>
    {/if}
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</label>
{#if onredraw}
  <button type="button" class="redraw" onclick={onredraw} aria-label={m.albums_redraw()} title={m.albums_redraw()}>
    <RefreshCw size={16} />
  </button>
{/if}

<style>
  .sort {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .sort select {
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    cursor: pointer;
  }

  .redraw {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
  }

  .redraw:hover {
    background: var(--color-surface-hover);
  }
</style>
