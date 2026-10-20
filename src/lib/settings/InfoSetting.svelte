<script lang="ts">
  // A read-only value of a setting row (an address, a version, a size), with
  // an optional line under it, e.g. a warning.
  import type { Snippet } from "svelte";

  let {
    value,
    tone = "normal",
    children,
  }: {
    value: string;
    /** `ok` / `error`: a status in its color. */
    tone?: "normal" | "ok" | "error";
    /** Under the value. */
    children?: Snippet;
  } = $props();
</script>

<div class="info">
  <span class="value {tone}">{value}</span>
  {#if children}<div class="extra">{@render children()}</div>{/if}
</div>

<style>
  .info {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--space-1);
    text-align: right;
  }

  .value {
    color: var(--color-text);
    font-size: 14px;
    font-weight: var(--font-weight-medium);
    overflow-wrap: anywhere;
  }

  .value.ok {
    color: var(--color-success);
  }

  .value.error {
    color: var(--color-error);
  }

  .extra {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }
</style>
