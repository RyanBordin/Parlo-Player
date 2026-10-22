<script module lang="ts">
  import type { AlbumResult, ArtistResult, TrackListSong } from "#lib/api.ts";

  export type BestItem =
    | { kind: "song"; song: TrackListSong }
    | { kind: "album"; album: AlbumResult }
    | { kind: "artist"; artist: ArtistResult };
</script>

<script lang="ts">
  // "Best result" of the search (spec 6.9), after Swing Music's "Top Result":
  // large card with the image, the type, the name and a count. A click opens
  // the album or artist, or plays the song; Play shows on hover. "..." opens
  // the item's menu, as does a right click (spec 8.1).
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import { getArtistPage, playAlbum, playContext } from "#lib/api.ts";
  import { artistSongs } from "#lib/artist-page.svelte.ts";
  import Cover from "#lib/components/Cover.svelte";
  import PlayButton from "#lib/components/cards/PlayButton.svelte";
  import { formatDuration, formatNumber, formatSongCount } from "#lib/format.ts";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { albumMenu, artistMenu, openSongsMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  /** Cover width in CSS pixels. */
  const COVER = 120;

  let {
    item,
    onplaysong,
    onerror,
  }: {
    item: BestItem;
    /** Plays the songs found, from this one. */
    onplaysong: () => void;
    onerror?: (error: unknown) => void;
  } = $props();

  let view = $derived.by(() => {
    switch (item.kind) {
      case "song": {
        const { song } = item;
        return {
          label: m.card_type_song(),
          name: song.title,
          cover: song.coverArt,
          fallback: null,
          round: false,
          details: [song.artist, formatDuration(song.duration)],
          href: null,
        };
      }
      case "album": {
        const { album } = item;
        return {
          label: m.card_type_album(),
          name: album.name,
          cover: album.coverArt,
          fallback: null,
          round: false,
          details: [album.artist, album.songCount === null ? null : formatSongCount(album.songCount)],
          href: `/albums/${encodeURIComponent(album.id)}`,
        };
      }
      case "artist": {
        const { artist } = item;
        const count = artist.albumCount;
        return {
          label: m.card_type_artist(),
          name: artist.name,
          cover: artist.coverArt,
          fallback: artist.fallbackCoverArt,
          // Artists are round everywhere; albums and songs square.
          round: true,
          details: [count === 0 ? m.albums_count_zero() : m.albums_count({ count, countText: formatNumber(count) })],
          href: `/artists/${encodeURIComponent(artist.id)}`,
        };
      }
    }
  });
  let details = $derived(view.details.filter(Boolean).join(" • "));
  let current = $derived(item.kind === "song" && player.track?.id === item.song.id);

  /** The artist plays as on their page: most played songs first (spec 6.7). */
  async function playArtist(artist: ArtistResult) {
    const page = await getArtistPage(artist.id);
    const ids = page ? artistSongs(page, undefined).map((s) => s.id) : [];
    if (ids.length) await playContext(ids, 0, artist.name);
  }

  function onmenu(event: MouseEvent) {
    if (item.kind === "song") openSongsMenu(event, [item.song], { label: m.search_context() });
    else if (item.kind === "album") {
      const { album } = item;
      openMenu(event, () => albumMenu(album));
    } else {
      const { artist } = item;
      openMenu(event, () => artistMenu(artist));
    }
  }

  function play() {
    if (item.kind === "song") onplaysong();
    else if (item.kind === "album") playAlbum(item.album.id).catch((e) => onerror?.(e));
    else playArtist(item.artist).catch((e) => onerror?.(e));
  }
</script>

<article class="card" class:current oncontextmenu={onmenu}>
  <div class="cover" style:width={`${COVER}px`}>
    <Cover id={view.cover} fallback={view.fallback} size={COVER} radius={view.round ? "50%" : undefined} />
  </div>
  <span class="label">{view.label}</span>
  <!-- Its ::after covers the whole card, so a click anywhere opens or plays. -->
  {#if view.href}
    <a class="name" href={view.href} title={view.name}>{view.name}</a>
  {:else}
    <button type="button" class="name" onclick={play} aria-label={m.card_play({ name: view.name })} title={view.name}>
      {view.name}
    </button>
  {/if}
  {#if details}
    <span class="details" title={details}>{details}</span>
  {/if}
  <button type="button" class="more" onclick={onmenu} aria-label={m.player_more()} title={m.player_more()} aria-haspopup="menu">
    <Ellipsis size={20} />
  </button>
  <PlayButton label={m.card_play({ name: view.name })} onplay={play} />
</article>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    min-width: 0;
    padding: var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
    transition: background var(--transition-fast);
  }

  .card:hover {
    background: var(--color-surface-hover);
  }

  .cover {
    margin-bottom: var(--space-2);
  }

  /* Small colored pill with the type, as in Swing Music. */
  .label {
    padding: 2px var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--color-accent);
    color: var(--color-text);
    font-size: 11px;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .name {
    max-width: 100%;
    overflow: hidden;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 26px;
    font-weight: var(--font-weight-bold);
    line-height: 1.2;
    text-align: left;
    text-decoration: none;
    white-space: nowrap;
    text-overflow: ellipsis;
    cursor: pointer;
  }

  .name::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: var(--radius-lg);
  }

  /* The focus ring goes around the whole card, not only the name. */
  .name:focus-visible {
    outline: none;
  }

  .name:focus-visible::after {
    outline: 2px solid var(--color-accent-text);
    outline-offset: 2px;
  }

  .current .name {
    color: var(--color-accent-text);
  }

  .details {
    max-width: 100%;
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 14px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  /* Above the name's ::after, so it does not open or play. */
  .more {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    z-index: 1;
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .more:hover {
    color: var(--color-text);
  }

  .card :global(.play) {
    z-index: 1;
    right: var(--space-5);
    bottom: var(--space-5);
  }
</style>
