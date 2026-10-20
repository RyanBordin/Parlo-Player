<script lang="ts" generics="T extends string | number">
  // Pick one of a few values: segmented buttons (native radios, so the arrow
  // keys move the choice), or a drop-down list when there are many (e.g. the
  // audio devices, later). Named by the setting row around it.
  import { getRowLabel } from "#lib/settings/registry.ts";

  /** More choices than this: a drop-down list. */
  const MAX_SEGMENTS = 5;

  let {
    value,
    choices,
    disabled = false,
    onchange,
  }: {
    value: T;
    choices: { value: T; label: string }[];
    /** Shown but off, e.g. the bitrate of an original stream. */
    disabled?: boolean;
    onchange: (value: T) => void;
  } = $props();

  const labelledBy = getRowLabel();
  const name = $props.id();
  let segmented = $derived(choices.length <= MAX_SEGMENTS);

  function onselect(event: Event) {
    const index = (event.currentTarget as HTMLSelectElement).selectedIndex;
    onchange(choices[index].value);
  }
</script>

{#if segmented}
  <div class="segments" class:disabled role="radiogroup" aria-labelledby={labelledBy} aria-disabled={disabled}>
    {#each choices as choice (choice.value)}
      <label class:checked={choice.value === value}>
        <input type="radio" {name} {disabled} checked={choice.value === value} onchange={() => onchange(choice.value)} />
        {choice.label}
      </label>
    {/each}
  </div>
{:else}
  <select class="select" aria-labelledby={labelledBy} {disabled} onchange={onselect}>
    {#each choices as choice (choice.value)}
      <option selected={choice.value === value}>{choice.label}</option>
    {/each}
  </select>
{/if}

<style>
  .segments {
    display: inline-flex;
    padding: 3px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-bg);
  }

  label {
    position: relative;
    padding: var(--space-1) var(--space-4);
    border-radius: var(--radius-pill);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    white-space: nowrap;
    cursor: pointer;
    transition:
      background-color var(--transition-fast),
      color var(--transition-fast);
  }

  label:hover {
    color: var(--color-text);
  }

  label.checked {
    background: var(--color-surface-active);
    color: var(--color-text);
  }

  /* The radio itself is hidden; its label is the button. */
  input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .segments.disabled {
    opacity: 0.4;
  }

  .segments.disabled label,
  .segments.disabled input {
    cursor: not-allowed;
  }

  .segments.disabled label:hover {
    color: var(--color-text-muted);
  }

  .select:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  label:has(input:focus-visible) {
    outline: 2px solid var(--color-accent-text);
    outline-offset: 2px;
  }

  .select {
    min-width: 200px;
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-bg);
    color: var(--color-text);
    font: inherit;
    font-size: var(--font-size-sm);
    cursor: pointer;
  }
</style>
