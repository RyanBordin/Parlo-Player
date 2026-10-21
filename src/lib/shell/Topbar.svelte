<script lang="ts">
  // Topbar (spec 4.1): Back / Forward, global search and profile.
  //
  // The search field drives the results page (spec 6.9): the first letter
  // typed opens it, then each pause in typing updates its address in place,
  // so Back leaves the search instead of going through every letter. Focusing
  // the empty field opens the page with the recent searches; Esc clears the
  // field and goes back to the page the search was opened from.
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Search from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";
  import { afterNavigate, goto } from "$app/navigation";
  import { page } from "$app/state";
  import type { SavedSession } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { parseTab, rememberSearch, SEARCH_DELAY_MS, searchHref } from "#lib/search.svelte.ts";
  import { canGoBack, canGoForward, navHistory, replaceUrl } from "#lib/shell/history.svelte.ts";
  import ProfileMenu from "#lib/shell/ProfileMenu.svelte";
  import { testLibrary } from "#lib/test-library.svelte.ts";

  let { session }: { session: SavedSession } = $props();

  let searchInput = $state<HTMLInputElement>();
  let value = $state("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** The field is moving to another address: the field leads, not the address. */
  let fromField = false;
  /** History entry the search was opened from, for Esc. */
  let openedFrom: number | null = null;

  let onSearchPage = $derived(page.url.pathname === "/search");

  // Back, Forward and links: the field shows the search of the page (empty
  // elsewhere), and a pending update is dropped.
  afterNavigate((navigation) => {
    if (fromField) return;
    clearTimeout(timer);
    const url = navigation.to?.url;
    value = url?.pathname === "/search" ? (url.searchParams.get("q") ?? "") : "";
  });

  async function moveTo(update: () => Promise<void>) {
    fromField = true;
    try {
      await update();
    } finally {
      fromField = false;
    }
  }

  function openSearch() {
    openedFrom = navHistory.index;
    // `reset: false`: the field keeps the focus.
    moveTo(() => goto(searchHref(value), { reset: false }));
  }

  /** The address follows the field (same tab). */
  function updateSearch() {
    clearTimeout(timer);
    if (fromField) {
      // Still opening the page: once it is there.
      timer = setTimeout(updateSearch, SEARCH_DELAY_MS);
      return;
    }
    if (!onSearchPage || (page.url.searchParams.get("q") ?? "") === value) return;
    moveTo(() => replaceUrl(searchHref(value, parseTab(page.url.searchParams.get("tab")))));
  }

  function oninput(event?: Event) {
    // The text typed, whichever of this and `bind:value` runs first.
    if (event?.currentTarget instanceof HTMLInputElement) value = event.currentTarget.value;
    clearTimeout(timer);
    if (!onSearchPage && !fromField) openSearch();
    else timer = setTimeout(updateSearch, SEARCH_DELAY_MS);
  }

  function onfocus() {
    if (!onSearchPage && !value) openSearch();
  }

  function clearField() {
    value = "";
    oninput();
    searchInput?.focus();
  }

  /** Esc: back to where the search was opened from. */
  function leaveSearch() {
    clearTimeout(timer);
    value = "";
    searchInput?.blur();
    if (!onSearchPage) return;
    if (openedFrom !== null && openedFrom < navHistory.index) history.go(openedFrom - navHistory.index);
    else if (canGoBack()) history.back();
    else goto("/");
    openedFrom = null;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      // Before the app shell's Esc (Now Playing).
      event.preventDefault();
      leaveSearch();
    } else if (event.key === "Enter") {
      updateSearch();
      rememberSearch(value);
    }
  }

  // Spec 4.1: Ctrl+K focuses the search from any screen.
  function onWindowKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "k") {
      event.preventDefault();
      searchInput?.focus();
      searchInput?.select();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<header class="topbar">
  <div class="history">
    <button
      class="round"
      type="button"
      onclick={() => history.back()}
      disabled={!canGoBack()}
      aria-label={m.topbar_back()}
      title={m.topbar_back()}
    >
      <ChevronLeft />
    </button>
    <button
      class="round"
      type="button"
      onclick={() => history.forward()}
      disabled={!canGoForward()}
      aria-label={m.topbar_forward()}
      title={m.topbar_forward()}
    >
      <ChevronRight />
    </button>
    <span class="divider" aria-hidden="true"></span>
  </div>

  <label class="search">
    <Search />
    <span class="visually-hidden">{m.search_label()}</span>
    <input
      bind:this={searchInput}
      bind:value
      type="search"
      placeholder={m.search_placeholder()}
      autocomplete="off"
      spellcheck="false"
      {oninput}
      {onfocus}
      {onkeydown}
    />
    {#if value}
      <button type="button" class="clear" onclick={clearField} aria-label={m.tracklist_clear()} title={m.tracklist_clear()}>
        <X size={16} />
      </button>
    {:else}
      <kbd aria-hidden="true">{m.search_shortcut()}</kbd>
    {/if}
  </label>

  <div class="spacer"></div>

  {#if testLibrary.enabled}
    <span class="test-badge" title={m.perf_badge_title()}>{m.perf_badge()}</span>
  {/if}
  <ProfileMenu {session} />
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4) var(--page-gutter);
  }

  /* Performance test mode only: this is not the real library. */
  .test-badge {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-pill);
    background: var(--color-accent);
    color: var(--color-text);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .history {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .round {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
  }

  .round:not(:disabled):hover {
    background: var(--color-surface-hover);
  }

  .round:disabled {
    color: var(--color-text-muted);
    opacity: 0.4;
    cursor: default;
  }

  .divider {
    width: 1px;
    height: 24px;
    margin-left: var(--space-1);
    background: var(--color-border);
  }

  .search {
    flex: 0 1 380px;
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border: 1px solid transparent;
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text-muted);
    cursor: text;
    transition: border-color var(--transition-fast);
  }

  .search:focus-within {
    border-color: var(--color-border-focus);
    color: var(--color-text);
  }

  .search input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    outline: none;
  }

  .search input::placeholder {
    color: var(--color-text-muted);
  }

  /* The field's own outline replaces the global focus ring. */
  .search input:focus-visible {
    outline: none;
  }

  /* The X button below replaces the native one. */
  .search input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    display: grid;
    place-items: center;
    padding: 2px;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .clear:hover {
    color: var(--color-text);
  }

  kbd {
    padding: 2px var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    font-family: inherit;
    font-size: var(--font-size-xs);
  }

  .spacer {
    flex: 1;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
