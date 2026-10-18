<script lang="ts">
  // Temporary page body until each screen is built (step 7).
  import Construction from "@lucide/svelte/icons/construction";
  import { m } from "#lib/paraglide/messages.js";

  let { title }: { title: string } = $props();
</script>

<div class="page">
  <h1 class="page-title">{title}</h1>
  <div class="notice">
    <Construction size={32} />
    <p class="heading">{m.under_construction_title()}</p>
    <p class="muted">{m.under_construction_text()}</p>
  </div>
</div>

<style>
  .notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    margin-top: var(--space-6);
    color: var(--color-text-muted);
  }

  .heading {
    color: var(--color-text);
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }
</style>
