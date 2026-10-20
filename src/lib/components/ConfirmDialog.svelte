<script lang="ts">
  // Asks before an action (e.g. "Restore defaults", spec 6.10). Esc and
  // Cancel close it without doing anything.
  import { m } from "#lib/paraglide/messages.js";

  let {
    title,
    message,
    confirmLabel,
    onconfirm,
    onclose,
  }: {
    title: string;
    message: string;
    confirmLabel: string;
    onconfirm: () => void;
    /** Called once the dialog is closed, confirmed or not. */
    onclose: () => void;
  } = $props();

  let dialog = $state<HTMLDialogElement>();
  const id = $props.id();

  $effect(() => {
    dialog?.showModal();
  });

  function confirm() {
    onconfirm();
    dialog?.close();
  }
</script>

<dialog bind:this={dialog} {onclose} aria-labelledby={`${id}-title`} aria-describedby={`${id}-text`}>
  <h2 id={`${id}-title`}>{title}</h2>
  <p id={`${id}-text`} class="muted">{message}</p>
  <div class="buttons">
    <button type="button" class="button button-secondary" onclick={() => dialog?.close()}>{m.settings_cancel()}</button>
    <button type="button" class="button button-primary" onclick={confirm}>{confirmLabel}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(440px, calc(100vw - 48px));
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--color-backdrop);
  }

  h2 {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .muted {
    margin-top: var(--space-2);
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }
</style>
