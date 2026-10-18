<script lang="ts">
  // Home (spec 6.1): six rows of cards, in this order.
  import {
    getHomeActivity,
    getHomeLibrary,
    playContext,
    type HomeActivity,
    type HomeLibrary,
    type SongRow,
  } from "#lib/api.ts";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import ArtistCard from "#lib/components/cards/ArtistCard.svelte";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import TrackCard from "#lib/components/cards/TrackCard.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  /** Cover width of the standard and the large ("Listen again") cards. */
  const CARD = 160;
  const LARGE_CARD = 200;

  let library = $state<HomeLibrary | null>(null);
  let activity = $state<HomeActivity | null>(null);
  let error = $state<ErrorMessage | null>(null);

  // The songs of "Listen again", in the row's order: a click plays them.
  let listenAgainSongs = $derived(
    (activity?.listenAgain ?? []).flatMap((item) => (item.kind === "song" ? [item.id] : [])),
  );

  // Each time the page opens and after every sync (spec 6.1). The local rows
  // come at once; the server ones when it answers.
  $effect(() => {
    void syncState.finishedCount;
    error = null;
    getHomeLibrary()
      .then((rows) => (library = rows))
      .catch((e) => (error = describeError(e)));
    getHomeActivity()
      .then((rows) => (activity = rows))
      .catch((e) => (error = describeError(e)));
  });

  function play(ids: string[], index: number, label: string) {
    error = null;
    playContext(ids, index, label).catch((e) => (error = describeError(e)));
  }

  /** A click on a song of a row plays the whole list it stands for. */
  function playRow(row: SongRow, index: number, label: string) {
    play(row.allIds, index, label);
  }

  function showError(e: unknown) {
    error = describeError(e);
  }
</script>

<div class="page">
  <h1 class="page-title">{m.nav_home()}</h1>
  {#if activity && !activity.fromServer}
    <p class="page-subtitle">{m.home_offline()}</p>
  {/if}
  {#if error}
    <p class="error">{error.message}</p>
  {/if}

  <div class="rows">
    <Carousel
      title={m.home_listen_again()}
      count={activity?.listenAgain.length ?? 0}
      loaded={activity !== null}
      empty={m.home_listen_again_empty()}
    >
      {#each activity?.listenAgain ?? [] as item (`${item.kind}-${item.id}`)}
        {#if item.kind === "album"}
          <AlbumCard album={item} size={LARGE_CARD} large label={m.card_type_album()} onerror={showError} />
        {:else}
          <TrackCard
            song={item}
            size={LARGE_CARD}
            large
            label={m.card_type_song()}
            onplay={() => play(listenAgainSongs, listenAgainSongs.indexOf(item.id), m.home_listen_again())}
          />
        {/if}
      {/each}
    </Carousel>

    <Carousel
      title={m.home_most_played_albums()}
      count={activity?.mostPlayedAlbums.length ?? 0}
      loaded={activity !== null}
      empty={m.home_most_played_albums_empty()}
    >
      {#each activity?.mostPlayedAlbums ?? [] as album (album.id)}
        <AlbumCard {album} size={CARD} onerror={showError} />
      {/each}
    </Carousel>

    <Carousel
      title={m.home_starred_songs()}
      count={library?.starredSongs.songs.length ?? 0}
      loaded={library !== null}
      empty={m.home_starred_songs_empty()}
    >
      {#each library?.starredSongs.songs ?? [] as song, index (song.id)}
        <TrackCard {song} size={CARD} onplay={() => playRow(library!.starredSongs, index, m.home_starred_songs())} />
      {/each}
    </Carousel>

    <Carousel
      title={m.home_starred_albums()}
      count={library?.starredAlbums.length ?? 0}
      loaded={library !== null}
      empty={m.home_starred_albums_empty()}
    >
      {#each library?.starredAlbums ?? [] as album (album.id)}
        <AlbumCard {album} size={CARD} onerror={showError} />
      {/each}
    </Carousel>

    <Carousel
      title={m.home_forgotten_starred()}
      count={library?.forgottenStarred.songs.length ?? 0}
      loaded={library !== null}
      empty={m.home_forgotten_starred_empty()}
    >
      {#each library?.forgottenStarred.songs ?? [] as song, index (song.id)}
        <TrackCard {song} size={CARD} onplay={() => playRow(library!.forgottenStarred, index, m.home_forgotten_starred())} />
      {/each}
    </Carousel>

    <Carousel
      title={m.home_starred_artists()}
      count={library?.starredArtists.length ?? 0}
      loaded={library !== null}
      empty={m.home_starred_artists_empty()}
    >
      {#each library?.starredArtists ?? [] as artist (artist.id)}
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
