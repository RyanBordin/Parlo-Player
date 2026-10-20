<script lang="ts">
  // One option (spec 6.10): its name, a short description in a secondary
  // tone when it is not obvious, and its control. Name and description come
  // from the registry, the same text the search looks at. Hidden when the
  // search leaves it out.
  import type { Snippet } from "svelte";
  import { findOption, getSettingsFilter, setRowLabel } from "#lib/settings/registry.ts";

  let {
    option,
    stacked = false,
    children,
  }: {
    /** ID in the registry. */
    option: string;
    /** The control goes under the text, full width (lists). */
    stacked?: boolean;
    /** The control. */
    children: Snippet;
  } = $props();

  const filter = getSettingsFilter();
  let info = $derived(findOption(option));
  const id = $props.id();
  setRowLabel(`${id}-label`);
</script>

{#if filter.shows(option)}
  <div class="row" class:stacked role="group" aria-labelledby={`${id}-label`} aria-describedby={info.description ? `${id}-text` : undefined}>
    <div class="text">
      <span class="label" id={`${id}-label`}>{info.label()}</span>
      {#if info.description}
        <span class="description" id={`${id}-text`}>{info.description()}</span>
      {/if}
    </div>
    <div class="control">
      {@render children()}
    </div>
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-5);
    padding: var(--space-4) var(--space-5);
  }

  .row + :global(.row) {
    border-top: 1px solid var(--color-border);
  }

  .row.stacked {
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-3);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .label {
    font-size: 14px;
    font-weight: var(--font-weight-medium);
  }

  .description {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .control {
    flex: none;
  }

  .stacked .control {
    flex: auto;
  }
</style>
