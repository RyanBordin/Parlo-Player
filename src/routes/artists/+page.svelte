<script lang="ts">
  // Artists (spec 6.6): album artists (or every artist, featured ones
  // included) in a grid of square cards, with quick search, "Sort by" and
  // "Random mix" of the artists shown.
  import Dices from "@lucide/svelte/icons/dices";
  import { playArtistsMix } from "#lib/api.ts";
  import {
    ARTIST_SORTS,
    ARTIST_TYPES,
    filterArtists,
    sortArtists,
    type ArtistSort,
    type ArtistType,
  } from "#lib/artist-list.ts";
  import { artistsView, refreshArtists, setArtistQuery, setArtistSort, setArtistType } from "#lib/artists.svelte.ts";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import CardGrid from "#lib/components/cards/CardGrid.svelte";
  import ListSearch from "#lib/components/ListSearch.svelte";
  import SortSelect from "#lib/components/SortSelect.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  /** Under the cover: the gap and the name line. */
  const TEXT_HEIGHT = 32;

  const SORT_LABELS: Record<ArtistSort, () => string> = {
    name: m.albums_sort_name,
    mostPlayed: m.albums_sort_mostPlayed,
    added: m.albums_sort_added,
    albumCount: m.artists_sort_album_count,
  };
  const TYPE_LABELS: Record<ArtistType, () => string> = {
    albumArtists: m.artists_type_album_artists,
    all: m.artists_type_all,
  };

  let error = $state<ErrorMessage | null>(null);

  /** The artists of the type chosen, before the search. */
  let ofType = $derived(filterArtists(artistsView.items ?? [], "", artistsView.type));
  /** The artists shown: what "Random mix" plays. */
  let shown = $derived(sortArtists(filterArtists(ofType, artistsView.query, "all"), artistsView.sort));
  let count = $derived.by(() => {
    const total = ofType.length;
    if (artistsView.query) return m.artists_filtered({ shown: formatNumber(shown.length), total: formatNumber(total) });
    return total === 0 ? m.artists_count_zero() : m.artists_count({ count: total, countText: formatNumber(total) });
  });

  // The search field and the saved search stay in step.
  let query = $state(artistsView.query);
  $effect(() => {
    if (query !== artistsView.query) setArtistQuery(query);
  });

  // On open (the last list shows meanwhile) and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    refreshArtists();
  });

  function mix() {
    error = null;
    playArtistsMix(
      shown.map((a) => a.id),
      m.nav_artists(),
    ).catch((e) => (error = describeError(e)));
  }
</script>

<div class="page">
  <header class="header">
    <div>
      <h1 class="page-title">{m.nav_artists()}</h1>
      {#if artistsView.items}
        <p class="page-subtitle">{count}</p>
      {/if}
    </div>
    <div class="buttons">
      <button type="button" class="button button-primary" onclick={mix} disabled={shown.length === 0}>
        <Dices size={16} />
        {m.albums_mix()}
      </button>
    </div>
  </header>

  <div class="tools">
    <ListSearch bind:value={query} placeholder={m.artists_search()} />
    <SortSelect
      value={artistsView.sort}
      options={ARTIST_SORTS.map((sort) => ({ value: sort, label: SORT_LABELS[sort]() }))}
      onchange={setArtistSort}
    />
    <SortSelect
      label={m.artists_type_label()}
      value={artistsView.type}
      options={ARTIST_TYPES.map((type) => ({ value: type, label: TYPE_LABELS[type]() }))}
      onchange={setArtistType}
    />
  </div>

  {#if error ?? artistsView.error}
    <p class="error">{(error ?? artistsView.error)!.message}</p>
  {/if}

  {#if artistsView.items && ofType.length === 0}
    <p class="empty">{m.artists_empty()}</p>
  {:else if artistsView.items && shown.length === 0}
    <p class="empty">{m.artists_no_results({ query: artistsView.query })}</p>
  {:else if artistsView.items}
    <CardGrid items={shown} key={(artist) => artist.id} textHeight={TEXT_HEIGHT}>
      {#snippet card(artist, size)}
        <ArtistCard {artist} {size} fluid />
      {/snippet}
    </CardGrid>
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

  .empty {
    color: var(--color-text-muted);
  }

  .error {
    margin-bottom: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
