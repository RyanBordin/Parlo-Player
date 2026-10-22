<script lang="ts">
  // One panel of a "..." menu (spec 8): groups between thin lines, shortcuts
  // in a secondary tone, and submenus "›" that open beside their item, on
  // the other side when they do not fit. Keyboard: arrows move and open,
  // Enter runs, Esc closes.
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Search from "@lucide/svelte/icons/search";
  import { closeMenu, runAction, type MenuEntry, type MenuGroup, type MenuSubmenu, type Placement } from "#lib/menu/menu.svelte.ts";
  import Self from "#lib/menu/MenuPanel.svelte";

  /** Space kept between a panel and the window's edges. */
  const EDGE = 8;
  /** Hovering another item waits this long before switching submenus. */
  const HOVER_DELAY_MS = 150;

  let {
    title,
    groups,
    placement,
    autofocus,
    search,
    level = 0,
    onback,
    onenter,
  }: {
    title?: string;
    groups: MenuGroup[];
    placement: Placement;
    /** The first item (or the search field) takes the focus. */
    autofocus: boolean;
    search?: MenuSubmenu["search"];
    /** 0: the root panel. */
    level?: number;
    /** Left arrow in a submenu: back to its item. */
    onback?: () => void;
    /** The mouse reached this submenu: the parent keeps it open. */
    onenter?: () => void;
  } = $props();

  let panel = $state<HTMLElement>();
  let searchInput = $state<HTMLInputElement>();
  // Off screen until measured, so it never shows at the wrong place.
  let left = $state(-9999);
  let top = $state(-9999);
  let query = $state("");
  let open = $state<{ entry: MenuSubmenu; placement: Placement; focus: boolean; opener: HTMLElement } | null>(null);
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  let shownGroups = $derived(filterGroups(groups, query));
  let nothingFound = $derived(query.trim() !== "" && !shownGroups.some((group) => group.some((e) => !isPinned(e))));

  function normalize(text: string): string {
    return text.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
  }

  function isPinned(entry: MenuEntry): boolean {
    return entry.kind === "action" && entry.pinned === true;
  }

  function filterGroups(all: MenuGroup[], text: string): MenuGroup[] {
    const words = normalize(text).split(/\s+/).filter(Boolean);
    const kept = words.length
      ? all.map((group) => group.filter((e) => isPinned(e) || words.every((w) => normalize(e.label).includes(w))))
      : all;
    return kept.filter((group) => group.length > 0);
  }

  // Placed once its size is known: below / right of the anchor, or flipped.
  $effect(() => {
    if (!panel) return;
    const { width, height } = panel.getBoundingClientRect();
    const maxX = window.innerWidth - EDGE;
    const maxY = window.innerHeight - EDGE;
    const x = placement.left + width <= maxX ? placement.left : placement.flipLeft - width;
    const y = placement.top + height <= maxY ? placement.top : placement.flipTop - height;
    left = Math.max(EDGE, Math.min(x, maxX - width));
    top = Math.max(EDGE, Math.min(y, maxY - height));
    // A submenu opened by the mouse leaves the focus where it was.
    if (!autofocus) {
      if (level === 0) panel.focus({ preventScroll: true });
    } else if (searchInput) searchInput.focus({ preventScroll: true });
    else items()[0]?.focus({ preventScroll: true });
  });

  function items(): HTMLElement[] {
    return panel ? [...panel.querySelectorAll<HTMLElement>("[role='menuitem']")] : [];
  }

  function moveFocus(step: number | "first" | "last") {
    const list = items();
    if (list.length === 0) return;
    const at = list.indexOf(document.activeElement as HTMLElement);
    const next =
      step === "first" ? 0 : step === "last" ? list.length - 1 : at === -1 ? (step > 0 ? 0 : list.length - 1) : (at + step + list.length) % list.length;
    list[next].focus();
  }

  function openSubmenu(entry: MenuSubmenu, opener: HTMLElement, focus: boolean) {
    clearTimeout(hoverTimer);
    if (open?.entry === entry && !focus) return;
    const rect = opener.getBoundingClientRect();
    open = {
      entry,
      opener,
      focus,
      placement: { left: rect.right + 2, top: rect.top - 5, flipLeft: rect.left - 2, flipTop: rect.bottom + 5 },
    };
  }

  function closeSubmenu(focusOpener: boolean) {
    const opener = open?.opener;
    open = null;
    if (focusOpener) opener?.focus();
  }

  function onItemClick(event: MouseEvent, entry: MenuEntry) {
    // A click on a submenu the mouse already opened keeps it; Enter goes in.
    if (entry.kind === "action") runAction(entry);
    else if (open?.entry !== entry || event.detail === 0) {
      openSubmenu(entry, event.currentTarget as HTMLElement, event.detail === 0);
    }
  }

  function onItemEnter(event: PointerEvent, entry: MenuEntry) {
    clearTimeout(hoverTimer);
    const opener = event.currentTarget as HTMLElement;
    hoverTimer = setTimeout(
      () => (entry.kind === "submenu" ? openSubmenu(entry, opener, false) : (open = null)),
      open || entry.kind === "action" ? HOVER_DELAY_MS : HOVER_DELAY_MS / 2,
    );
  }

  function onKeydown(event: KeyboardEvent) {
    // The global shortcuts (Space, arrows) must not act behind the menu.
    event.stopPropagation();
    const typing = event.target === searchInput;
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        moveFocus(1);
        break;
      case "ArrowUp":
        event.preventDefault();
        moveFocus(-1);
        break;
      case "Home":
      case "End":
        if (typing) return;
        event.preventDefault();
        moveFocus(event.key === "Home" ? "first" : "last");
        break;
      case "ArrowRight": {
        if (typing) return;
        const target = event.target as HTMLElement;
        const index = Number(target.dataset.index ?? -1);
        const entry = flatEntries()[index];
        if (entry?.kind === "submenu") {
          event.preventDefault();
          openSubmenu(entry, target, true);
        }
        break;
      }
      case "ArrowLeft":
        if (typing || level === 0) return;
        event.preventDefault();
        onback?.();
        break;
      case "Escape":
        // Taken here, so Now Playing and the selection stay as they are.
        event.preventDefault();
        closeMenu(true);
        break;
      case "Tab":
        event.preventDefault();
        closeMenu(true);
        break;
    }
  }

  function flatEntries(): MenuEntry[] {
    return shownGroups.flat();
  }

  function indexOf(groupIndex: number, entryIndex: number): number {
    return shownGroups.slice(0, groupIndex).reduce((sum, group) => sum + group.length, 0) + entryIndex;
  }
</script>

<!-- Keys are handled for the whole panel, whichever item has the focus. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="menu-panel"
  role="menu"
  tabindex="-1"
  aria-label={title}
  style:left={`${left}px`}
  style:top={`${top}px`}
  bind:this={panel}
  onkeydown={onKeydown}
  onpointerenter={() => onenter?.()}
>
  {#if title}
    <p class="title">{title}</p>
    <div class="separator" role="separator"></div>
  {/if}
  {#if search}
    <label class="search">
      <Search size={16} />
      <input
        bind:this={searchInput}
        bind:value={query}
        type="search"
        placeholder={search.placeholder}
        aria-label={search.placeholder}
        autocomplete="off"
        spellcheck="false"
      />
    </label>
  {/if}

  {#each shownGroups as group, groupIndex (groupIndex)}
    {#if groupIndex > 0}
      <div class="separator" role="separator"></div>
    {/if}
    {#each group as entry, entryIndex (entryIndex)}
      {@const Icon = entry.icon}
      <button
        type="button"
        class="item"
        class:expanded={open?.entry === entry}
        role="menuitem"
        tabindex="-1"
        data-index={indexOf(groupIndex, entryIndex)}
        aria-haspopup={entry.kind === "submenu" ? "menu" : undefined}
        aria-expanded={entry.kind === "submenu" ? open?.entry === entry : undefined}
        onclick={(event) => onItemClick(event, entry)}
        onpointerenter={(event) => onItemEnter(event, entry)}
      >
        <span class="icon">{#if Icon}<Icon size={16} />{/if}</span>
        <span class="label">{entry.label}</span>
        {#if entry.kind === "submenu"}
          <ChevronRight size={16} class="more" />
        {:else if entry.shortcut}
          <kbd class="shortcut">{entry.shortcut}</kbd>
        {/if}
      </button>
    {/each}
  {/each}
  {#if nothingFound && search}
    <p class="empty">{search.empty}</p>
  {/if}
</div>

{#if open}
  {#key open.entry}
    <Self
      groups={open.entry.groups}
      search={open.entry.search}
      placement={open.placement}
      autofocus={open.focus}
      level={level + 1}
      onback={() => closeSubmenu(true)}
      onenter={() => clearTimeout(hoverTimer)}
    />
  {/key}
{/if}

<style>
  .menu-panel {
    position: fixed;
    z-index: 30;
    min-width: 220px;
    max-width: 320px;
    max-height: calc(100vh - 16px);
    overflow-y: auto;
    padding: var(--space-1);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
    outline: none;
  }

  .title {
    padding: var(--space-2) var(--space-3);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
  }

  .separator {
    height: 1px;
    margin: var(--space-1) var(--space-2);
    background: var(--color-border);
  }

  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }

  .item:hover,
  .item:focus-visible,
  .item.expanded {
    background: var(--color-surface-hover);
    outline: none;
  }

  .icon {
    width: 16px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
    color: var(--color-text-muted);
  }

  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .item :global(.more) {
    flex-shrink: 0;
    color: var(--color-text-muted);
  }

  .shortcut {
    flex-shrink: 0;
    color: var(--color-text-subtle);
    font-family: inherit;
    font-size: var(--font-size-xs);
  }

  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-1) var(--space-1) var(--space-2);
    padding: 0 var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-muted);
  }

  .search:focus-within {
    border-color: var(--color-border-focus);
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    outline: none;
  }

  .empty {
    padding: var(--space-2) var(--space-3);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }
</style>
