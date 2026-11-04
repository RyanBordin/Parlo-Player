<script lang="ts">
  // Library sections on the phone (spec 4.5): Songs, Albums, Artists and
  // Playlists as tabs at the top of their pages. The one open is remembered,
  // so the Library tab comes back to it.
  import { page } from "$app/state";
  import { m } from "#lib/paraglide/messages.js";
  import { LIBRARY_TABS, libraryTabOf, rememberLibraryTab } from "#lib/shell/phone.svelte.ts";

  let current = $derived(libraryTabOf(page.url.pathname));

  $effect(() => {
    if (current) rememberLibraryTab(current.href);
  });
</script>

<nav class="library-tabs" aria-label={m.library_tabs_label()}>
  {#each LIBRARY_TABS as tab (tab.id)}
    {@const active = tab.id === current?.id}
    <a href={tab.href} class:active aria-current={active ? "page" : undefined}>{tab.label()}</a>
  {/each}
</nav>

<style>
  .library-tabs {
    display: flex;
    gap: var(--space-2);
    padding: 0 var(--page-gutter) var(--space-3);
    overflow-x: auto;
    scrollbar-width: none;
  }

  a {
    flex: none;
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    color: var(--color-text-muted);
    font-size: 14px;
    font-weight: var(--font-weight-medium);
    text-decoration: none;
    -webkit-tap-highlight-color: transparent;
  }

  a.active {
    border-color: transparent;
    background: var(--color-surface-active);
    color: var(--color-text);
  }
</style>
