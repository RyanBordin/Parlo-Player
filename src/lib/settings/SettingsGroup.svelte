<script lang="ts">
  // Options under a subtitle (spec 6.10). Hidden when the search leaves none
  // of its options.
  import type { Snippet } from "svelte";
  import { getSettingsFilter } from "#lib/settings/registry.ts";

  let {
    title,
    options,
    children,
  }: {
    title: string;
    /** IDs of the options inside, to know whether any is shown. */
    options: string[];
    children: Snippet;
  } = $props();

  const filter = getSettingsFilter();
  let shown = $derived(options.some((id) => filter.shows(id)));
</script>

{#if shown}
  <section class="group" aria-label={title}>
    <h3 class="title">{title}</h3>
    <div class="rows">
      {@render children()}
    </div>
  </section>
{/if}

<style>
  .group + :global(.group) {
    margin-top: var(--space-6);
  }

  .title {
    margin-bottom: var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .rows {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
  }
</style>
