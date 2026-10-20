<script lang="ts">
  // A button that does something (e.g. "Restore defaults"; later "Sync now",
  // "Clear cache"). With `confirm`, it asks first.
  import ConfirmDialog from "#lib/components/ConfirmDialog.svelte";

  let {
    label,
    onclick,
    confirm,
    disabled = false,
  }: {
    label: string;
    onclick: () => void;
    /** Asked before running. */
    confirm?: { title: string; message: string; confirmLabel?: string };
    disabled?: boolean;
  } = $props();

  let asking = $state(false);

  function press() {
    if (confirm) asking = true;
    else onclick();
  }
</script>

<button type="button" class="button button-secondary action" {disabled} onclick={press}>{label}</button>

{#if asking && confirm}
  <ConfirmDialog
    title={confirm.title}
    message={confirm.message}
    confirmLabel={confirm.confirmLabel ?? label}
    onconfirm={onclick}
    onclose={() => (asking = false)}
  />
{/if}

<style>
  .action {
    padding: var(--space-2) var(--space-4);
    font-size: 14px;
  }
</style>
