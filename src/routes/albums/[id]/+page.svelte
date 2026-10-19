<script lang="ts">
  // Album page (spec 6.5), from the local cache.
  import { page } from "$app/state";
  import { getAlbumPage, playContext, playShuffled, setAlbumStarred } from "#lib/api.ts";
  import CollectionActions from "#lib/components/collection/CollectionActions.svelte";
  import CollectionHeader from "#lib/components/collection/CollectionHeader.svelte";
  import TrackList from "#lib/components/TrackList.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDate, formatSongCount, formatTotalDuration } from "#lib/format.ts";
  import { albumPages, keepAlbum } from "#lib/page-cache.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  let id = $derived(page.params.id ?? "");
  /** `undefined` while loading the first time, `null` if not in the library. */
  let data = $derived(albumPages[id]);
  let error = $state<ErrorMessage | null>(null);
  /** The tracklist as shown (filtered and sorted): what Play and Shuffle play. */
  let shownIds = $state<string[]>([]);

  let duration = $derived(data ? (data.album.duration ?? data.songs.reduce((sum, s) => sum + (s.duration ?? 0), 0)) : 0);
  let genres = $derived(data ? (data.album.genres.length ? data.album.genres : data.album.genre ? [data.album.genre] : []) : []);

  // On open (the last copy shows meanwhile) and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    const albumId = id;
    error = null;
    getAlbumPage(albumId)
      .then((result) => keepAlbum(albumId, result))
      .catch((e) => (error = describeError(e)));
  });

  function showError(e: unknown) {
    error = describeError(e);
  }

  // The heart changes at once and goes back if the server refuses.
  async function toggleStar() {
    const album = data?.album;
    if (!album) return;
    const starred = !album.starred;
    album.starred = starred;
    try {
      await setAlbumStarred(album.id, starred);
    } catch (e) {
      album.starred = !starred;
      showError(e);
    }
  }
</script>

{#if data === null}
  <div class="page">
    <h1 class="page-title">{m.album_page_title()}</h1>
    <p class="message">{m.album_not_found()}</p>
  </div>
{:else if data}
  {@const album = data.album}
  <CollectionHeader coverArt={album.coverArt} title={album.name}>
    {#snippet meta()}
      <p class="meta">
        <span class="kind">{m.album_kind()}</span>
        {#if album.artist}
          <span>
            {m.collection_by()}
            {#if album.artistId}
              <a href={`/artists/${encodeURIComponent(album.artistId)}`}>{album.artist}</a>
            {:else}
              {album.artist}
            {/if}
          </span>
        {/if}
        <span>• {formatSongCount(album.songCount ?? data.songs.length)}, {formatTotalDuration(duration)}</span>
        {#if album.year}<span>• {album.year}</span>{/if}
      </p>
      <p class="meta secondary">
        {#if album.created && formatDate(album.created)}
          <span>{m.album_added({ date: formatDate(album.created) })}</span>
        {/if}
        {#if album.artist && album.artistId}
          <a class="pill" href={`/artists/${encodeURIComponent(album.artistId)}`}>{album.artist}</a>
        {/if}
        <!-- No genre page in v0.1: the pill has no action (spec 6.5). -->
        {#each genres as genre (genre)}
          <span class="pill">{genre}</span>
        {/each}
      </p>
    {/snippet}

    <CollectionActions
      disabled={shownIds.length === 0}
      starred={album.starred}
      onplay={() => playContext(shownIds, 0, album.name).catch(showError)}
      onshuffle={() => playShuffled(shownIds, album.name).catch(showError)}
      onstar={toggleStar}
    />
  </CollectionHeader>

  <div class="page tracks">
    {#if error}
      <p class="error">{error.message}</p>
    {/if}
    <TrackList
      songs={data.songs}
      kind="album"
      contextLabel={album.name}
      albumArtist={album.artist}
      bind:shownIds
      onerror={showError}
    />
  </div>
{:else if error}
  <div class="page">
    <p class="error">{error.message}</p>
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
