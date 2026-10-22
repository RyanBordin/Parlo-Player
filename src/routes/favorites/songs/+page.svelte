<script lang="ts">
  // "See all" of the favorite songs (spec 6.2): the whole list, last starred
  // first, in the virtualized tracklist with the list tools (spec 7). A click
  // plays the list as shown ("Playing from: Favorites", spec 5.1).
  import Play from "@lucide/svelte/icons/play";
  import MixIcon from "#lib/components/icons/MixIcon.svelte";
  import { onDestroy } from "svelte";
  import { playContext, playMix } from "#lib/api.ts";
  import TrackList from "#lib/components/TrackList.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { dropUnstarred, favorites, favoritesCount, isStarredNow, refreshFavorites } from "#lib/favorites.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { startLightSync, syncState } from "#lib/sync.svelte.ts";

  let error = $state<ErrorMessage | null>(null);
  /** The list as shown (searched and sorted). */
  let shownIds = $state<string[]>([]);

  let songs = $derived(favorites.data?.songs ?? []);
  let starred = $derived(new Set(songs.filter(isStarredNow).map((s) => s.id)));
  /** "Play all" and "Shuffle" leave out the songs just unstarred. */
  let playable = $derived(shownIds.filter((id) => starred.has(id)));

  // On open: the cache first, then the light sync for what was starred in
  // other apps. After each sync: read again, keeping the songs unstarred here.
  let opened = false;
  $effect(() => {
    void syncState.finishedCount;
    refreshFavorites(opened);
    if (!opened) startLightSync();
    opened = true;
  });
  onDestroy(dropUnstarred);

  function showError(e: unknown) {
    error = describeError(e);
  }

  function playAll() {
    error = null;
    playContext(playable, 0, m.nav_favorites()).catch(showError);
  }

  function shuffle() {
    error = null;
    playMix(playable, m.mix_context({ origin: m.nav_favorites() })).catch(showError);
  }
</script>

<div class="page">
  <header class="header">
    <div>
      <h1 class="page-title">{m.home_starred_songs()}</h1>
      {#if favorites.data && starred.size > 0}
        <p class="page-subtitle">{favoritesCount(0, starred.size, 0)}</p>
      {/if}
    </div>
    <div class="buttons">
      <button type="button" class="button button-primary" onclick={playAll} disabled={playable.length === 0}>
        <Play size={16} fill="currentColor" />
        {m.albums_play_all()}
      </button>
      <button type="button" class="button button-secondary" onclick={shuffle} disabled={playable.length === 0}>
        <MixIcon size={16} />
        {m.awesome_mix()}
      </button>
    </div>
  </header>

  {#if error ?? favorites.error}
    <p class="error">{(error ?? favorites.error)!.message}</p>
  {/if}

  {#if favorites.data}
    <div class="list">
      <TrackList {songs} kind="favorites" contextLabel={m.nav_favorites()} bind:shownIds onerror={showError} />
    </div>
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

  .error {
    margin-top: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
