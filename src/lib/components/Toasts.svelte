<script lang="ts">
  // Discreet confirmations above the Player Bar (see #lib/toasts.svelte.ts).
  import { fly } from "svelte/transition";
  import { toasts } from "#lib/toasts.svelte.ts";
</script>

<div class="toasts" role="status" aria-live="polite">
  {#each toasts as toast (toast.id)}
    <p class="toast" transition:fly={{ y: 12, duration: 180 }}>{toast.text}</p>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: var(--space-5);
    bottom: calc(var(--player-bar-height) + var(--space-4));
    z-index: 20;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: var(--space-2);
    pointer-events: none;
  }

  .toast {
    max-width: 360px;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: 0 12px 32px rgb(0 0 0 / 50%);
    font-size: 14px;
  }
</style>
