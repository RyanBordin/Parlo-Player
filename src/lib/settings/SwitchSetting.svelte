<script lang="ts">
  // On / off switch. Inside a setting row it is named by the row's label;
  // elsewhere (list items) by `label`.
  import { getRowLabel } from "#lib/settings/registry.ts";

  let {
    checked,
    onchange,
    disabled = false,
    label,
    title,
  }: {
    checked: boolean;
    onchange: (checked: boolean) => void;
    disabled?: boolean;
    /** Accessible name when not inside a setting row. */
    label?: string;
    /** Tooltip, e.g. why it is disabled. */
    title?: string;
  } = $props();

  const rowLabel = getRowLabel();
</script>

<button
  type="button"
  class="switch"
  role="switch"
  aria-checked={checked}
  aria-label={label}
  aria-labelledby={label ? undefined : rowLabel}
  {disabled}
  {title}
  onclick={() => onchange(!checked)}
>
  <span class="thumb"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 40px;
    height: 22px;
    padding: 0;
    border: 1px solid var(--color-border-focus);
    border-radius: var(--radius-pill);
    background: var(--color-surface-active);
    cursor: pointer;
    transition:
      background-color var(--transition-fast),
      border-color var(--transition-fast);
  }

  .switch[aria-checked="true"] {
    border-color: var(--color-accent);
    background: var(--color-accent);
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: var(--radius-pill);
    background: var(--color-text-muted);
    transition:
      transform var(--transition-fast),
      background-color var(--transition-fast);
  }

  .switch[aria-checked="true"] .thumb {
    background: var(--color-text);
    transform: translateX(18px);
  }

  .switch:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
