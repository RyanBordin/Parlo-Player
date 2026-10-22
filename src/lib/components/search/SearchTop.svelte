<script lang="ts">
  // "Top" tab of the search (spec 6.9): the best result with the first songs
  // next to it, then rows of albums and of artists (the Home cards). Each
  // section has "See all" (its tab); empty sections are left out.
  import type { SearchResults } from "#lib/api.ts";
  import TrackRow from "#lib/components/TrackRow.svelte";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import BestResult, { type BestItem } from "#lib/components/search/BestResult.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { searchHref } from "#lib/search.svelte.ts";

  /** Cover width of the cards, and how many each row shows (as on Home). */
  const CARD = 160;
  const ROW_SIZE = 20;

  let {
    query,
    results,
    onplay,
    onerror,
  }: {
    query: string;
    results: SearchResults;
    /** Plays the songs found, from `index`. */
    onplay: (index: number) => void;
    onerror: (error: unknown) => void;
  } = $props();

  let best = $derived.by((): BestItem | null => {
    if (results.best === "song" && results.topSongs[0]) return { kind: "song", song: results.topSongs[0] };
    if (results.best === "album" && results.albums[0]) return { kind: "album", album: results.albums[0] };
    if (results.best === "artist" && results.artists[0]) return { kind: "artist", artist: results.artists[0] };
    return null;
  });
  let albums = $derived(results.albums.slice(0, ROW_SIZE));
  let artists = $derived(results.artists.slice(0, ROW_SIZE));
</script>

{#snippet seeAll(href: string)}
  <a class="pill" {href}>{m.search_see_all()}</a>
{/snippet}

<div class="sections">
  {#if best}
    <div class="top">
      <section class="best" aria-labelledby="best-title">
        <header>
          <h2 id="best-title" class="section-title">{m.search_best_result()}</h2>
        </header>
        <BestResult item={best} onplaysong={() => onplay(0)} {onerror} />
      </section>

      {#if results.topSongs.length > 0}
        <section class="songs" aria-labelledby="songs-title">
          <header>
            <h2 id="songs-title" class="section-title">{m.nav_songs()}</h2>
            {@render seeAll(searchHref(query, "songs"))}
          </header>
          <div class="rows">
            {#each results.topSongs as song, index (song.id)}
              <TrackRow {song} contextLabel={m.search_context()} onplay={() => onplay(index)} />
            {/each}
          </div>
        </section>
      {/if}
    </div>
  {/if}

  {#if albums.length > 0}
    <Carousel title={m.nav_albums()} count={albums.length} empty="">
      {#snippet actions()}{@render seeAll(searchHref(query, "albums"))}{/snippet}
      {#each albums as album (album.id)}
        <AlbumCard {album} size={CARD} {onerror} />
      {/each}
    </Carousel>
  {/if}

  {#if artists.length > 0}
    <Carousel title={m.nav_artists()} count={artists.length} empty="">
      {#snippet actions()}{@render seeAll(searchHref(query, "artists"))}{/snippet}
      {#each artists as artist (artist.id)}
        <ArtistCard {artist} size={CARD} />
      {/each}
    </Carousel>
  {/if}
</div>

<style>
  .sections {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  /* Best result on the left, the songs on the right (Swing Music). */
  .top {
    display: grid;
    grid-template-columns: minmax(280px, 380px) minmax(0, 1fr);
    gap: var(--space-6);
    align-items: start;
  }

  /* Both titles on one line, as tall as the "See all" pill. */
  .top header {
    min-height: 32px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-3);
  }

  .top .section-title {
    margin: 0;
  }

  /* "See all": the same outlined pill as "See more" in the rows. */
  .songs .pill {
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

  .songs .pill:hover {
    background: var(--color-surface-hover);
  }

  .rows {
    display: flex;
    flex-direction: column;
  }
</style>
