<script lang="ts">
  // Artist page (spec 6.7), after YouTube Music: the banner with the photo,
  // name and biography; Top songs (the first 5 of the artist's songs, "See
  // all" has the whole list); albums, singles and EPs, "Appears on" and
  // similar artists. Sections without content are not shown. Songs and
  // albums come from the local cache; the biography and similar artists from
  // the server, kept 7 days.
  import { page } from "$app/state";
  import { playContext, playItem, playMix, type AlbumCard as AlbumCardData } from "#lib/api.ts";
  import { artistSongs, loadArtistAbout, loadArtistPage, toggleArtistStar } from "#lib/artist-page.svelte.ts";
  import ArtistHeader from "#lib/components/ArtistHeader.svelte";
  import TrackTable from "#lib/components/TrackTable.svelte";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import CollectionActions from "#lib/components/collection/CollectionActions.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { artistSection } from "#lib/menu/actions.svelte.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { artistMenu, openSongsMenu } from "#lib/menu/menus.ts";
  import { artistAbouts, artistPages, keepArtistAbout } from "#lib/page-cache.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { scrollBehavior } from "#lib/settings/app-settings.svelte.ts";
  import { syncState } from "#lib/sync.svelte.ts";
  import type { ColumnId, TableRow } from "#lib/track-columns.ts";

  /** Songs in Top songs; "See all" has the rest. */
  const TOP_SIZE = 5;
  /** Cover width asked for: a grid column is about this wide. */
  const CARD = 180;
  const COLUMNS: ColumnId[] = ["number", "title", "artist", "album", "duration", "favorite"];

  let id = $derived(page.params.id ?? "");
  /** `undefined` while loading the first time, `null` if not in the library. */
  let data = $derived(artistPages[id]);
  /** `undefined` while the server part loads. */
  let about = $derived(artistAbouts[id]);
  let error = $state<ErrorMessage | null>(null);

  /** The artist's whole list: what Play, the Awesome Mix and a click play. */
  let songs = $derived(data ? artistSongs(data, about) : []);
  let songIds = $derived(songs.map((s) => s.id));
  let topRows = $derived(songs.slice(0, TOP_SIZE).map((_, index): TableRow => ({ kind: "song", index })));
  let similar = $derived(about?.similar ?? []);
  let similarSection = $state<HTMLElement>();

  // On open (the last copy shows meanwhile) and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    const artistId = id;
    error = null;
    loadArtistPage(artistId).catch(showError);
  });

  // The server part, once per opening. Without it the page still works:
  // no biography, no similar artists.
  $effect(() => {
    const artistId = id;
    loadArtistAbout(artistId).catch((e) => {
      console.error("artist page: could not read the artist info", e);
      if (artistAbouts[artistId] === undefined) keepArtistAbout(artistId, null);
    });
  });

  // "Go to similar artists" (spec 8.2): scrolls there once the section shows.
  $effect(() => {
    if (artistSection.similarOf !== id || about === undefined) return;
    const target = similarSection;
    if (similar.length > 0 && !target) return;
    artistSection.similarOf = null;
    if (target) requestAnimationFrame(() => target.scrollIntoView({ behavior: scrollBehavior(), block: "start" }));
  });

  function showError(e: unknown) {
    error = describeError(e);
  }

  /** "Playing from: [Artist]" (spec 6.7). */
  function play(index: number) {
    if (!data) return;
    error = null;
    playContext(songIds, index, data.artist.name).catch(showError);
  }

  /** "Random order" (spec 6.7, 9): the artist's albums drawn, each in order. */
  function shuffle() {
    if (!data) return;
    error = null;
    playItem({ kind: "artist", id: data.artist.id }, "shuffled", data.artist.name).catch(showError);
  }

  function mix() {
    if (!data) return;
    error = null;
    playMix(songIds, m.mix_context({ origin: data.artist.name })).catch(showError);
  }

  function star() {
    if (data) toggleArtistStar(data).catch(showError);
  }
</script>

{#snippet albumSection(title: string, albums: AlbumCardData[])}
  {#if albums.length > 0}
    <section aria-label={title}>
      <h2 class="section-title">{title}</h2>
      <div class="grid">
        {#each albums as album (album.id)}
          <AlbumCard {album} size={CARD} fluid onerror={showError} />
        {/each}
      </div>
    </section>
  {/if}
{/snippet}

{#if data === null}
  <div class="page">
    <h1 class="page-title">{m.artist_page_title()}</h1>
    <p class="message">{m.artist_not_found()}</p>
  </div>
{:else if data}
  {@const artist = data.artist}
  <!-- A new artist starts with the biography closed. -->
  {#key id}
    <ArtistHeader {artist} {about}>
      <CollectionActions
        disabled={songIds.length === 0}
        starred={artist.starred}
        onplay={() => play(0)}
        onshuffle={shuffle}
        onmix={mix}
        onstar={star}
        onmore={(event) => openMenu(event, () => artistMenu(artist))}
      />
    </ArtistHeader>
  {/key}

  <div class="page sections">
    {#if error}
      <p class="error">{error.message}</p>
    {/if}

    {#if songs.length > 0}
      <section aria-label={m.artist_top_songs()}>
        <header class="songs-header">
          <h2 class="section-title">{m.artist_top_songs()}</h2>
          <a class="pill" href={`/artists/${encodeURIComponent(id)}/songs`}>{m.artist_see_all()}</a>
        </header>
        <TrackTable
          rows={topRows}
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
          onplay={play}
          onmenu={(event, indexes) =>
            openSongsMenu(
              event,
              indexes.map((index) => songs[index]),
              { label: artist.name },
            )}
          onerror={showError}
        />
      </section>
    {/if}

    {@render albumSection(m.artist_albums(), data.albums)}
    {@render albumSection(m.artist_singles_eps(), data.singlesAndEps)}
    {@render albumSection(m.artist_appears_on(), data.appearsOn)}

    {#if similar.length > 0}
      <div class="similar" bind:this={similarSection}>
        <Carousel title={m.artist_similar()} count={similar.length} empty="">
          {#each similar as similarArtist (similarArtist.id)}
            <ArtistCard artist={similarArtist} size={160} />
          {/each}
        </Carousel>
      </div>
    {/if}
  </div>
{:else if error}
  <div class="page">
    <p class="error">{error.message}</p>
  </div>
{/if}

<style>
  /* Below the Topbar when "Go to similar artists" scrolls here. */
  .similar {
    scroll-margin-top: var(--space-5);
  }

  .sections {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding-top: var(--space-3);
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

  /* "See all": the same outlined pill as on the Favorites page. */
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

  /* As many columns as fit; the cards fill them. */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--space-6) var(--space-5);
  }

  .message {
    margin-top: var(--space-4);
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
