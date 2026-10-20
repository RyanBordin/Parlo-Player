<script lang="ts">
  // A number in a range (e.g. the ReplayGain pre-amplification), with its
  // value shown next to the slider. Saved when released, not at every step.
  // Named by the setting row around it.
  import { getRowLabel } from "#lib/settings/registry.ts";

  let {
    value,
    min,
    max,
    step = 1,
    format,
    disabled = false,
    onchange,
  }: {
    value: number;
    min: number;
    max: number;
    step?: number;
    /** The value as shown, e.g. "+3 dB". */
    format: (value: number) => string;
    disabled?: boolean;
    onchange: (value: number) => void;
  } = $props();

  const labelledBy = getRowLabel();
  /** Follows the slider while it is dragged. */
  let shown = $state<number | null>(null);
</script>

<div class="slider">
  <input
    type="range"
    {min}
    {max}
    {step}
    {value}
    {disabled}
    aria-labelledby={labelledBy}
    aria-valuetext={format(shown ?? value)}
    oninput={(event) => (shown = Number(event.currentTarget.value))}
    onchange={(event) => {
      shown = null;
      onchange(Number(event.currentTarget.value));
    }}
  />
  <output class:disabled>{format(shown ?? value)}</output>
</div>

<style>
  .slider {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  input {
    width: 200px;
    accent-color: var(--color-accent-text);
    cursor: pointer;
  }

  input:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  output {
    min-width: 56px;
    color: var(--color-text);
    font-size: var(--font-size-sm);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  output.disabled {
    color: var(--color-text-muted);
  }
</style>
