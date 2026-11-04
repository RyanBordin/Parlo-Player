<script lang="ts">
  // Phone selection bar (spec 4.5): in place of the Topbar while songs are
  // selected with long presses. X ends the selection; "Add to playlist" and
  // "..." (the selection's menu, spec 8.1) act on every song selected.
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import ListPlus from "@lucide/svelte/icons/list-plus";
  import X from "@lucide/svelte/icons/x";
  import { formatNumber } from "#lib/format.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { addToPlaylistMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";
  import type { TouchSelection } from "#lib/touch-selection.svelte.ts";

  let { selection }: { selection: TouchSelection } = $props();

  function addToPlaylist(event: MouseEvent) {
    const ids = selection.ids();
    if (ids.length) openMenu(event, () => addToPlaylistMenu({ kind: "songs", ids }));
  }
</script>

<header class="bar">
  <button class="icon" type="button" onclick={selection.close} aria-label={m.selection_close()}>
    <X />
  </button>
  <p class="count">{m.menu_selected({ count: selection.count, countText: formatNumber(selection.count) })}</p>
  <button class="icon" type="button" onclick={addToPlaylist} aria-label={m.player_add_to_playlist()} aria-haspopup="menu">
    <ListPlus />
  </button>
  <button class="icon" type="button" onclick={selection.openMenu} aria-label={m.player_more()} aria-haspopup="menu">
    <Ellipsis />
  </button>
</header>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 60px;
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-2);
    background: var(--color-surface-active);
  }

  .count {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-semibold);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .icon {
    flex: none;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    -webkit-tap-highlight-color: transparent;
  }
</style>
