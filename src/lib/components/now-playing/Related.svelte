<script lang="ts">
  // "Related" tab (spec 4.4): four sections, in this order; empty ones are
  // not shown. Song lists of 4 lines and album rows, sized to the column.
  import { getRelated } from "#lib/api.ts";
  import AlbumCard from "#lib/components/cards/AlbumCard.svelte";
  import Carousel from "#lib/components/cards/Carousel.svelte";
  import TrackShelf from "#lib/components/cards/TrackShelf.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";
  import { keepRelated, relatedBySong } from "#lib/song-cache.svelte.ts";

  /** A song line needs about this much width. */
  const LINE_WIDTH = 260;
  const ALBUM_CARD = 140;

  let songId = $derived(player.track?.id ?? null);
  let related = $derived(songId ? relatedBySong[songId] : undefined);
  let error = $state<ErrorMessage | null>(null);
  let width = $state(0);
  let columns = $derived(Math.min(3, Math.max(1, Math.floor(width / LINE_WIDTH))));
  let empty = $derived(
    related === null ||
      (related !== undefined &&
        related.similar.songs.length === 0 &&
        related.artistSongs.songs.length === 0 &&
        related.artistAlbums.length === 0 &&
        related.genreAlbums.length === 0),
  );

  // Asked when the tab opens and when the song changes; kept per song.
  $effect(() => {
    const id = songId;
    if (!id || id in relatedBySong) return;
    error = null;
    getRelated(id)
      .then((result) => keepRelated(id, result))
      .catch((e) => (error = describeError(e)));
  });

  function showError(e: unknown) {
    error = describeError(e);
  }
</script>

<div class="related" bind:clientWidth={width}>
  {#if !songId}
    <p class="message">{m.player_nothing_playing()}</p>
  {:else if related === undefined}
    <p class="message">{error ? m.related_error() : m.related_loading()}</p>
  {:else if empty || !related}
    <p class="message">{m.related_none()}</p>
  {:else}
    {#if error}<p class="error">{error.message}</p>{/if}

    {#if related.similar.songs.length}
      <TrackShelf title={m.related_similar()} row={related.similar} loaded empty="" {columns} playAll={false} onerror={showError} />
    {/if}

    {#if related.artist && related.artistSongs.songs.length}
      <TrackShelf
        title={m.related_more_from({ artist: related.artist })}
        row={related.artistSongs}
        loaded
        empty=""
        {columns}
        playAll={false}
        onerror={showError}
      />
    {/if}

    {#if related.artist && related.artistAlbums.length}
      <Carousel title={m.related_albums_by({ artist: related.artist })} count={related.artistAlbums.length} empty="">
        {#each related.artistAlbums as album (album.id)}
          <AlbumCard {album} size={ALBUM_CARD} onerror={showError} />
        {/each}
      </Carousel>
    {/if}

    {#if related.genre && related.genreAlbums.length}
      <Carousel title={m.related_same_genre({ genre: related.genre })} count={related.genreAlbums.length} empty="">
        {#each related.genreAlbums as album (album.id)}
          <AlbumCard {album} size={ALBUM_CARD} onerror={showError} />
        {/each}
      </Carousel>
    {/if}
  {/if}
</div>

<style>
  .related {
    height: 100%;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding: var(--space-5) var(--space-2) var(--space-6) 0;
  }

  .message {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
