<script lang="ts">
  // "See all" of the artist's songs (spec 6.7): the whole list, in the
  // artist page's order, in the virtualized tracklist with the list tools
  // (spec 7). A click plays the list as shown ("Playing from: [Artist]").
  import Dices from "@lucide/svelte/icons/dices";
  import Play from "@lucide/svelte/icons/play";
  import { page } from "$app/state";
  import { playContext, playMix } from "#lib/api.ts";
  import { artistSongs, loadArtistAbout, loadArtistPage } from "#lib/artist-page.svelte.ts";
  import TrackList from "#lib/components/TrackList.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatSongCount } from "#lib/format.ts";
  import { artistAbouts, artistPages } from "#lib/page-cache.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  let id = $derived(page.params.id ?? "");
  let data = $derived(artistPages[id]);
  let songs = $derived(data ? artistSongs(data, artistAbouts[id]) : []);
  let error = $state<ErrorMessage | null>(null);
  /** The list as shown (searched and sorted). */
  let shownIds = $state<string[]>([]);

  // On open (the artist page's copy shows meanwhile) and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    const artistId = id;
    error = null;
    loadArtistPage(artistId).catch(showError);
  });

  // Opened directly (not from the artist page): the order may come from the server.
  $effect(() => {
    const artistId = id;
    if (artistAbouts[artistId] === undefined) {
      loadArtistAbout(artistId).catch((e) => console.error("artist songs: could not read the artist info", e));
    }
  });

  function showError(e: unknown) {
    error = describeError(e);
  }

  function playAll() {
    if (!data) return;
    error = null;
    playContext(shownIds, 0, data.artist.name).catch(showError);
  }

  function shuffle() {
    if (!data) return;
    error = null;
    playMix(shownIds, data.artist.name).catch(showError);
  }
</script>

<div class="page">
  {#if data === null}
    <h1 class="page-title">{m.artist_page_title()}</h1>
    <p class="message">{m.artist_not_found()}</p>
  {:else if data}
    <header class="header">
      <div>
        <h1 class="page-title">
          <a href={`/artists/${encodeURIComponent(id)}`}>{data.artist.name}</a>
        </h1>
        <p class="page-subtitle">{formatSongCount(songs.length)}</p>
      </div>
      <div class="buttons">
        <button type="button" class="button button-primary" onclick={playAll} disabled={shownIds.length === 0}>
          <Play size={16} fill="currentColor" />
          {m.albums_play_all()}
        </button>
        <button type="button" class="button button-secondary" onclick={shuffle} disabled={shownIds.length === 0}>
          <Dices size={16} />
          {m.songs_shuffle()}
        </button>
      </div>
    </header>

    {#if error}
      <p class="error">{error.message}</p>
    {/if}

    <div class="list">
      <TrackList {songs} kind="artist" contextLabel={data.artist.name} bind:shownIds onerror={showError} />
    </div>
  {:else if error}
    <p class="error">{error.message}</p>
  {/if}
</div>

<style>
  .header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .page-title a {
    color: inherit;
    text-decoration: none;
  }

  .page-title a:hover {
    text-decoration: underline;
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .buttons .button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    font-size: 14px;
  }

  .list {
    margin-top: var(--space-5);
  }

  .message {
    margin-top: var(--space-4);
    color: var(--color-text-muted);
  }

  .error {
    margin-top: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
