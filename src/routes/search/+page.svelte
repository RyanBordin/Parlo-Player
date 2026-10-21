<script lang="ts">
  // Search results (spec 6.9): the "Top", "Songs", "Albums" and "Artists"
  // pills over the results of the search in the address, typed in the
  // Topbar. An empty search lists the recent searches.
  import History from "@lucide/svelte/icons/history";
  import X from "@lucide/svelte/icons/x";
  import { page } from "$app/state";
  import { playContext } from "#lib/api.ts";
  import TrackTable from "#lib/components/TrackTable.svelte";
  import AlbumGrid from "#lib/components/cards/AlbumGrid.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import CardGrid from "#lib/components/cards/CardGrid.svelte";
  import SearchTop from "#lib/components/search/SearchTop.svelte";
  import type { ColumnId, TableRow } from "#lib/track-columns.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import {
    forgetSearch,
    foundSongAt,
    loadRecentSearches,
    parseTab,
    rememberSearch,
    runSearch,
    SEARCH_TABS,
    searchHref,
    searchState,
    type SearchTab,
  } from "#lib/search.svelte.ts";
  import { syncState } from "#lib/sync.svelte.ts";

  const TAB_LABELS: Record<SearchTab, () => string> = {
    top: m.search_tab_top,
    songs: m.nav_songs,
    albums: m.nav_albums,
    artists: m.nav_artists,
  };
  /** The Songs page's columns; the order is the relevance. */
  const SONG_COLUMNS: ColumnId[] = ["number", "title", "artist", "album", "duration", "favorite"];
  /** Under an artist's cover: the gap and the name line (as on the Artists page). */
  const ARTIST_TEXT_HEIGHT = 32;

  let query = $derived(page.url.searchParams.get("q") ?? "");
  let tab = $derived(parseTab(page.url.searchParams.get("tab")));
  let blank = $derived(query.trim() === "");
  /** The last results stay while the new ones are found. */
  let results = $derived(blank ? null : searchState.results);
  let nothing = $derived(
    results !== null &&
      searchState.query === query &&
      results.songIds.length + results.albums.length + results.artists.length === 0,
  );
  let songRows = $derived((results?.songIds ?? []).map((_, index): TableRow => ({ kind: "song", index })));
  let shownQuery = $derived(query.trim());

  // On each search, and again after every sync.
  let lastSync = syncState.finishedCount;
  $effect(() => {
    const finished = syncState.finishedCount;
    const fresh = finished !== lastSync;
    lastSync = finished;
    if (!blank) runSearch(query, fresh);
  });

  $effect(() => {
    if (blank) loadRecentSearches();
  });

  let error = $state<ErrorMessage | null>(null);

  function showError(e: unknown) {
    error = describeError(e);
  }

  /** A song plays every song found, from it ("Playing from: Search"). */
  function play(index: number) {
    if (!results) return;
    error = null;
    playContext(results.songIds, index, m.search_context()).catch(showError);
  }

  /** Opening or playing a result makes the search a recent one. */
  function onResultsClick(event: MouseEvent) {
    if (event.target instanceof Element && event.target.closest("a, button")) rememberSearch(query);
  }
</script>

<div class="page">
  <h1 class="visually-hidden">{m.search_label()}</h1>

  {#if blank}
    <section class="recent" aria-labelledby="recent-title">
      <h2 id="recent-title" class="section-title">{m.search_recent()}</h2>
      {#if searchState.recent?.length}
        <ul>
          {#each searchState.recent as recent (recent)}
            <li>
              <a href={searchHref(recent)} onclick={() => rememberSearch(recent)}>
                <History size={16} />
                <span class="text">{recent}</span>
              </a>
              <button
                type="button"
                class="remove"
                onclick={() => forgetSearch(recent)}
                aria-label={m.search_recent_remove({ query: recent })}
                title={m.search_recent_remove({ query: recent })}
              >
                <X size={16} />
              </button>
            </li>
          {/each}
        </ul>
      {:else if searchState.recent}
        <p class="empty">{m.search_recent_empty()}</p>
      {/if}
    </section>
  {:else}
    <nav class="tabs" aria-label={m.search_tabs_label()}>
      {#each SEARCH_TABS as each (each)}
        <a class="tab" href={searchHref(query, each)} aria-current={each === tab ? "page" : undefined}>
          {TAB_LABELS[each]()}
        </a>
      {/each}
    </nav>

    {#if error ?? searchState.error}
      <p class="error">{(error ?? searchState.error)!.message}</p>
    {/if}

    {#if nothing}
      <p class="empty">{m.search_no_results({ query: shownQuery })}</p>
    {:else if results}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (only listens to the links and buttons inside) -->
      <div onclickcapture={onResultsClick}>
        {#if tab === "top"}
          <SearchTop {query} {results} onplay={play} onerror={showError} />
        {:else if tab === "songs"}
          {#if results.songIds.length === 0}
            <p class="empty">{m.tracklist_no_results({ query: shownQuery })}</p>
          {:else}
            <TrackTable
              rows={songRows}
              songAt={foundSongAt}
              columns={SONG_COLUMNS}
              sortKeys={{}}
              sort={null}
              onsort={() => {}}
              numberTitle={m.search_relevance()}
              numberOf={(index) => index + 1}
              showCover
              artistBelow={() => false}
              selectKey={(index) => results?.songIds[index] ?? index}
              onplay={play}
              onerror={showError}
            />
          {/if}
        {:else if tab === "albums"}
          {#if results.albums.length === 0}
            <p class="empty">{m.albums_no_results({ query: shownQuery })}</p>
          {:else}
            <AlbumGrid albums={results.albums} onerror={showError} />
          {/if}
        {:else if results.artists.length === 0}
          <p class="empty">{m.artists_no_results({ query: shownQuery })}</p>
        {:else}
          <CardGrid items={results.artists} key={(artist) => artist.id} textHeight={ARTIST_TEXT_HEIGHT}>
            {#snippet card(artist, size)}
              <ArtistCard {artist} {size} fluid />
            {/snippet}
          </CardGrid>
        {/if}
      </div>
    {/if}
  {/if}
</div>

<style>
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  /* Swing Music pills: the tab shown in the text color, the others on the surface. */
  .tab {
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text);
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    text-decoration: none;
  }

  .tab:hover {
    background: var(--color-surface-hover);
  }

  .tab[aria-current="page"] {
    background: var(--color-text);
    color: var(--color-text-inverse);
  }

  .recent ul {
    max-width: 560px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .recent li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    border-radius: var(--radius-md);
  }

  .recent li:hover {
    background: var(--color-surface);
  }

  .recent a {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    color: var(--color-text-muted);
    text-decoration: none;
  }

  .recent .text {
    overflow: hidden;
    color: var(--color-text);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .remove {
    display: grid;
    place-items: center;
    margin-right: var(--space-2);
    padding: var(--space-1);
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .remove:hover {
    color: var(--color-text);
  }

  .empty {
    color: var(--color-text-muted);
  }

  .error {
    margin-bottom: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
