<script lang="ts">
  // Round Play button over a card's cover, shown on hover (spec 5.2).
  import Play from "@lucide/svelte/icons/play";

  let {
    label,
    onplay,
    centered = false,
  }: {
    label: string;
    onplay: () => void;
    /** Large cards: in the middle of the cover (spec 6.1). Others: bottom right. */
    centered?: boolean;
  } = $props();
</script>

<button
  class="play"
  class:centered
  type="button"
  aria-label={label}
  title={label}
  onclick={(event) => {
    event.preventDefault();
    event.stopPropagation();
    onplay();
  }}
>
  <Play size={centered ? 24 : 18} fill="currentColor" />
</button>

<style>
  .play {
    position: absolute;
    right: var(--space-2);
    bottom: var(--space-2);
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    color: var(--color-text-inverse);
    box-shadow: var(--shadow-button);
    opacity: 0;
    transform: translateY(4px);
    cursor: pointer;
    transition:
      opacity var(--transition-fast),
      transform var(--transition-fast);
  }

  .play.centered {
    inset: 50% auto auto 50%;
    width: 56px;
    height: 56px;
    transform: translate(-50%, -50%) scale(0.9);
  }

  /* The card (any ancestor with .card) shows the button on hover or focus. */
  :global(.card:hover) .play,
  :global(.card:focus-within) .play {
    opacity: 1;
    transform: translateY(0);
  }

  :global(.card:hover) .play.centered,
  :global(.card:focus-within) .play.centered {
    transform: translate(-50%, -50%) scale(1);
  }

  .play:hover {
    background: var(--color-accent-text);
  }
</style>
