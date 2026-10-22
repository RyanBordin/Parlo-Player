<script lang="ts">
  // Playlist page (spec 6.5): asked from the server on open, with the last
  // copy saved on this computer when it cannot be reached.
  import { page } from "$app/state";
  import { getPlaylistPage, playContext, playShuffled } from "#lib/api.ts";
  import CollectionActions from "#lib/components/collection/CollectionActions.svelte";
  import CollectionHeader from "#lib/components/collection/CollectionHeader.svelte";
  import TrackList from "#lib/components/TrackList.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatSongCount, formatTotalDuration } from "#lib/format.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { playlistMenuById } from "#lib/menu/menus.ts";
  import { keepPlaylist, playlistPages } from "#lib/page-cache.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { playlists } from "#lib/playlists.svelte.ts";
  import { syncState } from "#lib/sync.svelte.ts";

  let id = $derived(page.params.id ?? "");
  /** `undefined` while loading the first time, `null` if it was deleted. */
  let data = $derived(playlistPages[id]);
  let error = $state<ErrorMessage | null>(null);
  /** The tracklist as shown (filtered and sorted): what Play and Shuffle play. */
  let shownIds = $state<string[]>([]);

  // Before the server answers, the sidebar already knows the name.
  let name = $derived(data?.playlist.name ?? playlists.items.find((p) => p.id === id)?.name ?? m.playlist_page_title());
  let duration = $derived(
    data ? (data.playlist.duration ?? data.songs.reduce((sum, s) => sum + (s.duration ?? 0), 0)) : 0,
  );

  // On open (the last copy shows meanwhile) and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    const playlistId = id;
    error = null;
    getPlaylistPage(playlistId)
      .then((result) => keepPlaylist(playlistId, result))
      .catch((e) => (error = describeError(e)));
  });

  function showError(e: unknown) {
    error = describeError(e);
  }
</script>

{#if data === null}
  <div class="page">
    <h1 class="page-title">{name}</h1>
    <p class="message">{m.playlist_not_found()}</p>
  </div>
{:else if data}
  {@const playlist = data.playlist}
  <CollectionHeader coverArt={playlist.coverArt} title={playlist.name}>
    {#snippet meta()}
      <p class="meta">
        <span class="kind">{m.playlist_kind()}</span>
        {#if playlist.owner}<span>• {m.collection_owner({ owner: playlist.owner })}</span>{/if}
        <span>• {formatSongCount(playlist.songCount ?? data.songs.length)}, {formatTotalDuration(duration)}</span>
      </p>
      {#if data.offline}
        <p class="meta secondary">{m.playlist_offline()}</p>
      {/if}
    {/snippet}

    <!-- No heart: the API has no favorite playlists. -->
    <CollectionActions
      disabled={shownIds.length === 0}
      onplay={() => playContext(shownIds, 0, playlist.name).catch(showError)}
      onshuffle={() => playShuffled(shownIds, playlist.name).catch(showError)}
      onmore={(event) => openMenu(event, () => playlistMenuById(playlist))}
    />
  </CollectionHeader>

  <div class="page tracks">
    {#if error}
      <p class="error">{error.message}</p>
    {/if}
    <TrackList
      songs={data.songs}
      kind="playlist"
      contextLabel={playlist.name}
      {playlist}
      bind:shownIds
      onerror={showError}
    />
  </div>
{:else}
  <div class="page">
    <h1 class="page-title">{name}</h1>
    {#if error}
      <p class="error">{error.message}</p>
    {/if}
  </div>
{/if}

<style>
  .tracks {
    padding-top: var(--space-3);
  }

  .message {
    margin-top: var(--space-4);
    color: var(--color-text-muted);
  }

  .error {
    margin-bottom: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
