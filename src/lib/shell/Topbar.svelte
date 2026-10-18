<script lang="ts">
  // Topbar (spec 4.1): Back / Forward, global search and profile.
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Search from "@lucide/svelte/icons/search";
  import type { SavedSession } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { canGoBack, canGoForward } from "#lib/shell/history.svelte.ts";
  import ProfileMenu from "#lib/shell/ProfileMenu.svelte";

  let { session }: { session: SavedSession } = $props();

  let searchInput = $state<HTMLInputElement>();

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

  <!-- Visual only for now: the search itself comes later (spec 6.9). -->
  <label class="search">
    <Search />
    <span class="visually-hidden">{m.search_label()}</span>
    <input
      bind:this={searchInput}
      type="search"
      placeholder={m.search_placeholder()}
      autocomplete="off"
      spellcheck="false"
    />
    <kbd aria-hidden="true">{m.search_shortcut()}</kbd>
  </label>

  <div class="spacer"></div>

  <ProfileMenu {session} />
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4) var(--page-gutter);
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
