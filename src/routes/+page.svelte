<script lang="ts">
  // Home (spec 6.1): six sections, in this order. Song sections have "Play
  // all"; album and artist rows have "See more".
  import { playContext, type SongRow } from "#lib/api.ts";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import PlayAllButton from "#lib/components/cards/PlayAllButton.svelte";
  import TrackCard from "#lib/components/cards/TrackCard.svelte";
  import TrackShelf from "#lib/components/cards/TrackShelf.svelte";
  import { describeError } from "#lib/errors.ts";
  import { home, refreshHome } from "#lib/home.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  /** Cover width of the standard and the large ("Listen again") cards. */
  const CARD = 160;
  const LARGE_CARD = 200;

  // The last data shows at once; then each opening and each sync refresh it.
  $effect(() => {
    void syncState.finishedCount;
    refreshHome();
  });

  function showError(e: unknown) {
    home.error = describeError(e);
  }

  /** A song section plays its whole list, from `index` ("Playing from" its title). */
  function playRow(row: SongRow | undefined, index: number, label: string) {
    if (row) playContext(row.allIds, index, label).catch(showError);
  }
</script>

{#snippet seeMore(href: string)}
  <a class="pill" {href}>{m.home_see_more()}</a>
{/snippet}

<div class="page">
  <h1 class="page-title">{m.nav_home()}</h1>
  {#if home.activity?.offline}
    <p class="page-subtitle">{m.home_offline()}</p>
  {/if}
  {#if home.error}
    <p class="error">{home.error.message}</p>
  {/if}

  <div class="rows">
    <Carousel
      title={m.home_listen_again()}
      count={home.library?.listenAgain.songs.length ?? 0}
      loaded={home.library !== null}
      empty={m.home_listen_again_empty()}
    >
      {#snippet actions()}
        <PlayAllButton onplay={() => playRow(home.library?.listenAgain, 0, m.home_listen_again())} />
      {/snippet}
      {#each home.library?.listenAgain.songs ?? [] as song, index (song.id)}
        <TrackCard
          {song}
          size={LARGE_CARD}
          large
          onplay={() => playRow(home.library?.listenAgain, index, m.home_listen_again())}
        />
      {/each}
    </Carousel>

    <TrackShelf
      title={m.home_starred_songs()}
      row={home.library?.starredSongs ?? null}
      loaded={home.library !== null}
      empty={m.home_starred_songs_empty()}
      onerror={showError}
    />

    <Carousel
      title={m.home_most_played_albums()}
      count={home.activity?.mostPlayedAlbums.length ?? 0}
      loaded={home.activity !== null}
      empty={m.home_most_played_albums_empty()}
    >
      {#snippet actions()}{@render seeMore("/home/most-played-albums")}{/snippet}
      {#each home.activity?.mostPlayedAlbums ?? [] as album (album.id)}
        <AlbumCard {album} size={CARD} onerror={showError} />
      {/each}
    </Carousel>

    <Carousel
      title={m.home_starred_albums()}
      count={home.library?.starredAlbums.length ?? 0}
      loaded={home.library !== null}
      empty={m.home_starred_albums_empty()}
    >
      {#snippet actions()}{@render seeMore("/home/favorite-albums")}{/snippet}
      {#each home.library?.starredAlbums ?? [] as album (album.id)}
        <AlbumCard {album} size={CARD} onerror={showError} />
      {/each}
    </Carousel>

    <TrackShelf
      title={m.home_forgotten_starred()}
      row={home.library?.forgottenStarred ?? null}
      loaded={home.library !== null}
      empty={m.home_forgotten_starred_empty()}
      onerror={showError}
    />

    <Carousel
      title={m.home_starred_artists()}
      count={home.library?.starredArtists.length ?? 0}
      loaded={home.library !== null}
      empty={m.home_starred_artists_empty()}
    >
      {#snippet actions()}{@render seeMore("/home/favorite-artists")}{/snippet}
      {#each home.library?.starredArtists ?? [] as artist (artist.id)}
        <ArtistCard {artist} size={CARD} />
      {/each}
    </Carousel>
  </div>
</div>

<style>
  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    margin-top: var(--space-6);
  }

  .error {
    margin-top: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
