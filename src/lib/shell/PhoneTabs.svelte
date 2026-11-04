<script lang="ts">
  // Phone tabs (spec 4.5): exactly Home, Search, Library and Favorites, in
  // place of the sidebar. Tapping the tab already open goes back to the top
  // of its page.
  import { page } from "$app/state";
  import { m } from "#lib/paraglide/messages.js";
  import { activeTab, libraryHref, PHONE_TABS } from "#lib/shell/phone.svelte.ts";

  let { onReselect }: { onReselect: () => void } = $props();

  let active = $derived(activeTab(page.url.pathname));

  function onclick(event: MouseEvent, id: string) {
    if (id !== active) return;
    event.preventDefault();
    onReselect();
  }
</script>

<nav class="tabs" aria-label={m.nav_label()}>
  {#each PHONE_TABS as tab (tab.id)}
    {@const current = tab.id === active}
    <a
      href={tab.id === "library" ? libraryHref() : tab.href}
      class:active={current}
      aria-current={current ? "page" : undefined}
      onclick={(event) => onclick(event, tab.id)}
    >
      <span class="pill"><tab.icon /></span>
      <span class="label">{tab.label()}</span>
    </a>
  {/each}
</nav>

<style>
  .tabs {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    padding: var(--space-2) var(--space-2) var(--space-3);
    border-top: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  a {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    text-decoration: none;
    -webkit-tap-highlight-color: transparent;
  }

  .pill {
    display: grid;
    place-items: center;
    width: 56px;
    height: 30px;
    border-radius: var(--radius-pill);
    transition: background-color var(--transition-fast);
  }

  /* Spec 4.5: the active tab looks like the sidebar's active item (spec 4.2):
     dark grey pill, icon and text bright white. */
  a.active {
    color: var(--color-text);
  }

  a.active .pill {
    background: var(--color-surface-active);
  }
</style>
