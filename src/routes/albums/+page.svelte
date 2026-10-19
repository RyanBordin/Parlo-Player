<script lang="ts">
  // Albums (spec 6.4): every album of the library in a grid, with quick
  // search, "Sort by" and three ways to play the albums shown.
  import Dices from "@lucide/svelte/icons/dices";
  import Play from "@lucide/svelte/icons/play";
  import RefreshCw from "@lucide/svelte/icons/refresh-cw";
  import Shuffle from "@lucide/svelte/icons/shuffle";
  import { playAlbums, type AlbumsPlayMode } from "#lib/api.ts";
  import { ALBUM_SORTS, filterAlbums, sortAlbums, type AlbumSort } from "#lib/album-list.ts";
  import {
    albumsView,
    drawRandomOrder,
    randomRank,
    refreshAlbums,
    setAlbumQuery,
    setAlbumSort,
  } from "#lib/albums.svelte.ts";
  import AlbumGrid from "#lib/components/cards/AlbumGrid.svelte";
  import ListSearch from "#lib/components/ListSearch.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  const SORT_LABELS: Record<AlbumSort, () => string> = {
    name: m.albums_sort_name,
    artist: m.albums_sort_artist,
    year: m.albums_sort_year,
    added: m.albums_sort_added,
    mostPlayed: m.albums_sort_mostPlayed,
    recentlyPlayed: m.albums_sort_recentlyPlayed,
    random: m.albums_sort_random,
  };

  let error = $state<ErrorMessage | null>(null);

  let all = $derived(albumsView.items ?? []);
  // The albums shown: what the three buttons play, in this order.
  let shown = $derived.by(() => {
    void albumsView.draw;
    return sortAlbums(filterAlbums(all, albumsView.query), albumsView.sort, randomRank);
  });
  let count = $derived.by(() => {
    if (albumsView.query) return m.albums_filtered({ shown: formatNumber(shown.length), total: formatNumber(all.length) });
    return all.length === 0 ? m.albums_count_zero() : m.albums_count({ count: all.length, countText: formatNumber(all.length) });
  });

  // The search field and the saved search stay in step.
  let query = $state(albumsView.query);
  $effect(() => {
    if (query !== albumsView.query) setAlbumQuery(query);
  });

  // On open (the last list shows meanwhile) and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    refreshAlbums();
  });

  function play(mode: AlbumsPlayMode) {
    error = null;
    playAlbums(
      shown.map((a) => a.id),
      mode,
      m.nav_albums(),
    ).catch((e) => (error = describeError(e)));
  }
</script>

<div class="page">
  <header class="header">
    <div>
      <h1 class="page-title">{m.nav_albums()}</h1>
      {#if albumsView.items}
        <p class="page-subtitle">{count}</p>
      {/if}
    </div>
    <div class="buttons">
      <button type="button" class="button button-primary" onclick={() => play("inOrder")} disabled={shown.length === 0}>
        <Play size={16} fill="currentColor" />
        {m.albums_play_all()}
      </button>
      <button type="button" class="button button-secondary" onclick={() => play("shuffledAlbums")} disabled={shown.length === 0}>
        <Shuffle size={16} />
        {m.albums_shuffled_albums()}
      </button>
      <button type="button" class="button button-secondary" onclick={() => play("mix")} disabled={shown.length === 0}>
        <Dices size={16} />
        {m.albums_mix()}
      </button>
    </div>
  </header>

  <div class="tools">
    <ListSearch bind:value={query} placeholder={m.albums_search()} />
    <label class="sort">
      <span>{m.albums_sort_label()}</span>
      <select value={albumsView.sort} onchange={(e) => setAlbumSort(e.currentTarget.value as AlbumSort)}>
        {#each ALBUM_SORTS as sort (sort)}
          <option value={sort}>{SORT_LABELS[sort]()}</option>
        {/each}
      </select>
    </label>
    {#if albumsView.sort === "random"}
      <button type="button" class="redraw" onclick={drawRandomOrder} aria-label={m.albums_redraw()} title={m.albums_redraw()}>
        <RefreshCw size={16} />
      </button>
    {/if}
  </div>

  {#if error ?? albumsView.error}
    <p class="error">{(error ?? albumsView.error)!.message}</p>
  {/if}

  {#if albumsView.items && all.length === 0}
    <p class="empty">{m.albums_empty()}</p>
  {:else if albumsView.items && shown.length === 0}
    <p class="empty">{m.albums_no_results({ query: albumsView.query })}</p>
  {:else if albumsView.items}
    <AlbumGrid albums={shown} onerror={(e) => (error = describeError(e))} />
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

  .tools {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin: var(--space-5) 0;
  }

  .sort {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .sort select {
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    cursor: pointer;
  }

  .redraw {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
  }

  .redraw:hover {
    background: var(--color-surface-hover);
  }

  .empty {
    color: var(--color-text-muted);
  }

  .error {
    margin-bottom: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
