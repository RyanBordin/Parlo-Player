<script lang="ts">
  // Persistent sidebar (spec 4.2): brand, main menu (arranged in Settings >
  // Appearance, spec 6.10) and playlists.
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import { page } from "$app/state";
  import symbol from "#lib/assets/logo-symbol.png";
  import Cover from "#lib/components/Cover.svelte";
  import { coverMode, setCoverExpanded } from "#lib/cover-mode.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import PlaylistList from "#lib/shell/PlaylistList.svelte";
  import { isActive, sidebarItems } from "#lib/shell/nav-items.ts";
  import { player } from "#lib/player.svelte.ts";

  /** Sidebar width minus its padding. */
  const EXPANDED_COVER_SIZE = 208;

  // In the order of the preferences, without the hidden ones (spec 6.10).
  let items = $derived(sidebarItems());
</script>

<aside class="sidebar">
  <!-- Spec 1.1: the symbol with the app name, like Swing Music. -->
  <div class="brand">
    <img src={symbol} alt="" width="25" height="32" />
    <span>{m.app_name()}</span>
  </div>

  <nav aria-label={m.nav_label()}>
    <ul>
      {#each items as item (item.id)}
        {@const active = isActive(item, page.url.pathname)}
        <li>
          <a href={item.href} class:active aria-current={active ? "page" : undefined}>
            <item.icon />
            <span>{item.label()}</span>
          </a>
        </li>
      {/each}
    </ul>
  </nav>

  <hr />

  <PlaylistList />

  <!-- Spec 4.3: expanded cover at the bottom; the playlists make room for it. -->
  {#if coverMode.expanded && player.track}
    <div class="expanded-cover">
      <Cover id={player.track.coverArt} size={EXPANDED_COVER_SIZE} radius="var(--radius-lg)" />
      <button
        class="cover-toggle"
        type="button"
        onclick={() => setCoverExpanded(false)}
        aria-label={m.player_collapse_cover()}
        title={m.player_collapse_cover()}
      >
        <ChevronDown />
      </button>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-height: 0;
    padding: var(--space-4);
    border-right: 1px solid var(--color-border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    font-size: var(--font-size-md);
    font-weight: var(--font-weight-bold);
  }

  .brand img {
    flex: none;
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  nav a {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-pill);
    color: var(--color-text-muted);
    font-size: 14px;
    font-weight: var(--font-weight-medium);
    text-decoration: none;
    transition:
      background-color var(--transition-fast),
      color var(--transition-fast);
  }

  nav a:hover {
    color: var(--color-text);
  }

  /* Spec 4.2: active item on a dark grey pill, icon and text bright white. */
  nav a.active {
    background: var(--color-surface-active);
    color: var(--color-text);
  }

  .expanded-cover {
    position: relative;
    flex: none;
  }

  /* The ˅ arrow shows on hover. */
  .cover-toggle {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    width: 32px;
    height: 32px;
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

  .expanded-cover:hover .cover-toggle,
  .cover-toggle:focus-visible {
    opacity: 1;
  }

  hr {
    width: 100%;
    margin: 0;
    border: none;
    border-top: 1px solid var(--color-border);
  }
</style>
