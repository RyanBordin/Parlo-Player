<script lang="ts">
  // Favorites (spec 6.2), after the Swing Music page: counter, "Play all"
  // and "Shuffle"; then Recent, Albums, Songs and Artists, last starred
  // first. A song plays the whole list of favorite songs ("Playing from:
  // Favorites"), not only the ones in view.
  import Play from "@lucide/svelte/icons/play";
  import { onDestroy } from "svelte";
  import { playContext, playMix } from "#lib/api.ts";
  import TrackTable from "#lib/components/TrackTable.svelte";
  import MixIcon from "#lib/components/icons/MixIcon.svelte";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import TrackCard from "#lib/components/cards/TrackCard.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { dropUnstarred, favorites, favoritesCount, isStarredNow, refreshFavorites } from "#lib/favorites.svelte.ts";
  import { openSongsMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { startLightSync, syncState } from "#lib/sync.svelte.ts";
  import type { ColumnId, TableRow } from "#lib/track-columns.ts";

  /** Cover width of the cards. */
  const CARD = 160;
  /** Songs in the table; "See all" has the rest. */
  const TABLE_SIZE = 10;
  const COLUMNS: ColumnId[] = ["number", "title", "artist", "album", "duration", "favorite"];

  let error = $state<ErrorMessage | null>(null);

  let data = $derived(favorites.data);
  let songs = $derived(data?.songs ?? []);
  /** Every song shown, in order: the queue of a click. */
  let songIds = $derived(songs.map((s) => s.id));
  /** "Play all" and the Awesome Mix leave out the songs just unstarred. */
  let starredIds = $derived(songs.filter(isStarredNow).map((s) => s.id));
  let tableRows = $derived(songs.slice(0, TABLE_SIZE).map((_, index): TableRow => ({ kind: "song", index })));
  let count = $derived(data ? favoritesCount(data.albums.length, starredIds.length, data.artists.length) : "");
  let nothing = $derived(data !== null && data.albums.length + data.artists.length + data.songs.length === 0);

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

  function playSong(index: number) {
    error = null;
    playContext(songIds, index, m.nav_favorites()).catch(showError);
  }

  function playSongId(id: string) {
    const index = songIds.indexOf(id);
    if (index >= 0) playSong(index);
  }

  function playAll() {
    error = null;
    playContext(starredIds, 0, m.nav_favorites()).catch(showError);
  }

  function shuffle() {
    error = null;
    playMix(starredIds, m.mix_context({ origin: m.nav_favorites() })).catch(showError);
  }
</script>

{#snippet seeMore(href: string, label: string)}
  <a class="pill" {href}>{label}</a>
{/snippet}

<div class="page">
  <header class="header">
    <div>
      <h1 class="page-title">{m.nav_favorites()}</h1>
      {#if count}
        <p class="page-subtitle">{count}</p>
      {/if}
    </div>
    {#if data && !nothing}
      <div class="buttons">
        <button type="button" class="button button-primary" onclick={playAll} disabled={starredIds.length === 0}>
          <Play size={16} fill="currentColor" />
          {m.albums_play_all()}
        </button>
        <button type="button" class="button button-secondary" onclick={shuffle} disabled={starredIds.length === 0}>
          <MixIcon size={16} />
          {m.awesome_mix()}
        </button>
      </div>
    {/if}
  </header>

  {#if error ?? favorites.error}
    <p class="error">{(error ?? favorites.error)!.message}</p>
  {/if}

  {#if nothing}
    <p class="empty">{m.favorites_empty()}</p>
  {:else}
    <div class="rows">
      <Carousel title={m.favorites_recent()} count={data?.recent.length ?? 0} loaded={data !== null} empty={m.favorites_empty()}>
        {#each data?.recent ?? [] as item (`${item.kind}-${item.id}`)}
          {#if item.kind === "album"}
            <AlbumCard album={item} size={CARD} label={m.card_type_album()} onerror={showError} />
          {:else if item.kind === "artist"}
            <ArtistCard artist={item} size={CARD} label={m.card_type_artist()} />
          {:else}
            <TrackCard
              song={item}
              size={CARD}
              label={m.card_type_song()}
              contextLabel={m.nav_favorites()}
              onplay={() => playSongId(item.id)}
            />
          {/if}
        {/each}
      </Carousel>

      <Carousel title={m.nav_albums()} count={data?.albums.length ?? 0} loaded={data !== null} empty={m.home_starred_albums_empty()}>
        {#snippet actions()}{@render seeMore("/favorites/albums", m.home_see_more())}{/snippet}
        {#each data?.albums ?? [] as album (album.id)}
          <AlbumCard {album} size={CARD} onerror={showError} />
        {/each}
      </Carousel>

      <section class="songs" aria-label={m.nav_songs()}>
        <header class="songs-header">
          <h2 class="section-title">{m.nav_songs()}</h2>
          {#if songs.length > 0}
            {@render seeMore("/favorites/songs", m.favorites_see_all())}
          {/if}
        </header>
        {#if songs.length > 0}
          <TrackTable
            rows={tableRows}
            songAt={(index) => songs[index]}
            columns={COLUMNS}
            sortKeys={{}}
            sort={null}
            onsort={() => {}}
            numberTitle={m.tracklist_default_order()}
            numberOf={(index) => index + 1}
            showCover
            artistBelow={() => false}
            selectKey={(index) => songs[index]?.id ?? index}
            onplay={playSong}
            onmenu={(event, indexes) =>
              openSongsMenu(
                event,
                indexes.map((index) => songs[index]),
                { label: m.nav_favorites() },
              )}
            onerror={showError}
          />
        {:else if data}
          <p class="section-empty">{m.home_starred_songs_empty()}</p>
        {/if}
      </section>

      <Carousel title={m.nav_artists()} count={data?.artists.length ?? 0} loaded={data !== null} empty={m.home_starred_artists_empty()}>
        {#snippet actions()}{@render seeMore("/favorites/artists", m.home_see_more())}{/snippet}
        {#each data?.artists ?? [] as artist (artist.id)}
          <ArtistCard {artist} size={CARD} />
        {/each}
      </Carousel>
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

  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    margin-top: var(--space-6);
  }

  .songs-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-3);
  }

  .songs-header .section-title {
    margin: 0;
  }

  /* "See all": the same outlined pill as "See more" in the rows. */
  .songs-header .pill {
    display: flex;
    align-items: center;
    height: 32px;
    padding: 0 var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    color: var(--color-text);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    text-decoration: none;
  }

  .songs-header .pill:hover {
    background: var(--color-surface-hover);
  }

  .empty {
    margin-top: var(--space-6);
    color: var(--color-text-muted);
  }

  .section-empty {
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .error {
    margin-top: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
