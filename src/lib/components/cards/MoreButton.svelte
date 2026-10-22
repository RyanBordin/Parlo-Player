<script lang="ts">
  // "..." on a card's cover, shown on hover or focus (any ancestor with
  // .card): opens the item's menu (spec 8). It takes the corner of the
  // favorite badge, which hides meanwhile.
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import { m } from "#lib/paraglide/messages.js";

  let {
    onmenu,
    round = false,
  }: {
    onmenu: (event: MouseEvent) => void;
    /** Round covers (artists): inside the circle. */
    round?: boolean;
  } = $props();
</script>

<button
  class="more"
  class:round
  type="button"
  aria-label={m.player_more()}
  title={m.player_more()}
  aria-haspopup="menu"
  onclick={onmenu}
>
  <Ellipsis size={18} />
</button>

<style>
  .more {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    z-index: 1;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-scrim);
    color: var(--color-text);
    opacity: 0;
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .more.round {
    top: 6%;
    right: 6%;
  }

  :global(.card:hover) .more,
  :global(.card:focus-within) .more {
    opacity: 1;
  }

  .more:hover {
    background: var(--color-surface-active);
  }

  /* The badge gives its corner to the button. */
  :global(.card:hover .badge),
  :global(.card:focus-within .badge) {
    opacity: 0;
  }
</style>
