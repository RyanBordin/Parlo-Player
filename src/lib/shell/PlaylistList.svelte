<script lang="ts">
  // Lower zone of the sidebar (spec 4.2): "+ New playlist" and every playlist.
  // A right click on a playlist opens its menu (spec 8.1).
  import Plus from "@lucide/svelte/icons/plus";
  import { page } from "$app/state";
  import { showDialog } from "#lib/menu/dialogs.svelte.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { playlistMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { loadPlaylists, playlistSubtitle, playlists } from "#lib/playlists.svelte.ts";
  import { syncState } from "#lib/sync.svelte.ts";

  // On open and after every sync (playlists come with the light sync).
  $effect(() => {
    void syncState.finishedCount;
    loadPlaylists();
  });
</script>

<section class="playlists" aria-label={m.playlists_label()}>
  <button class="new" type="button" onclick={() => showDialog({ kind: "newPlaylist", target: null })}>
    <Plus />
    <span>{m.playlists_new()}</span>
  </button>

  {#if playlists.loaded && playlists.items.length === 0}
    <p class="empty">{m.playlists_empty()}</p>
  {:else}
    <ul>
      {#each playlists.items as playlist (playlist.id)}
        {@const href = `/playlists/${encodeURIComponent(playlist.id)}`}
        <li>
          <a
            {href}
            class:active={page.url.pathname === href}
            aria-current={page.url.pathname === href ? "page" : undefined}
            oncontextmenu={(event) => openMenu(event, () => playlistMenu(playlist))}
          >
            <span class="name">{playlist.name}</span>
            <span class="subtitle">{playlistSubtitle(playlist)}</span>
          </a>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .playlists {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-height: 0;
  }

  .new {
    align-self: flex-start;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
  }

  .new:hover {
    background: var(--color-surface-hover);
  }

  .empty {
    padding: 0 var(--space-3);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  ul {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }

  a {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: var(--color-text);
    text-decoration: none;
  }

  a:hover {
    background: var(--color-surface);
  }

  a.active {
    background: var(--color-surface-active);
  }

  span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .name {
    font-size: 14px;
    font-weight: var(--font-weight-medium);
  }

  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
  }
</style>
