<script lang="ts">
  // The open "..." menu, once for the whole app (spec 8). A click outside,
  // scrolling, resizing or leaving the window closes it.
  import { closeMenu, menuState } from "#lib/menu/menu.svelte.ts";
  import MenuPanel from "#lib/menu/MenuPanel.svelte";

  function isInside(target: EventTarget | null): boolean {
    return target instanceof Element && target.closest(".menu-panel") !== null;
  }

  function onPointerDown(event: PointerEvent) {
    // The "..." button that opened it toggles it on its own click.
    if (!menuState.spec || isInside(event.target)) return;
    if (menuState.trigger && menuState.trigger.contains(event.target as Node)) return;
    closeMenu();
  }

  function onScroll(event: Event) {
    if (menuState.spec && !isInside(event.target)) closeMenu();
  }
</script>

<svelte:window
  onpointerdown={onPointerDown}
  onresize={() => closeMenu()}
  onblur={() => closeMenu()}
/>
<svelte:document onscrollcapture={onScroll} />

{#if menuState.spec}
  {#key menuState.spec}
    <MenuPanel
      title={menuState.spec.title}
      groups={menuState.spec.groups}
      search={menuState.spec.search}
      placement={menuState.placement}
      autofocus={menuState.keyboard}
    />
  {/key}
{/if}
