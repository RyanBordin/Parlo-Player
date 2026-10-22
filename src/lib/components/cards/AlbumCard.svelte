<script lang="ts">
  // Album card (spec 5.2): click opens the album, Play on the cover plays it,
  // the artist's name opens the artist. "..." and a right click open its
  // menu (spec 8.1).
  import Heart from "@lucide/svelte/icons/heart";
  import { playAlbum, type AlbumCard } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import MoreButton from "#lib/components/cards/MoreButton.svelte";
  import PlayButton from "#lib/components/cards/PlayButton.svelte";
  import { openMenu } from "#lib/menu/menu.svelte.ts";
  import { albumMenu } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";

  let {
    album,
    size = 160,
    large = false,
    label,
    fluid = false,
    onerror,
  }: {
    /** `starred` shows the favorite badge (Albums grid). */
    album: AlbumCard & { starred?: boolean };
    /** Cover width in CSS pixels. */
    size?: number;
    /** "Listen again" style: Play in the middle. */
    large?: boolean;
    /** Small type label above the title (rows that mix kinds). */
    label?: string;
    /** Fills its grid column instead of `size` pixels. */
    fluid?: boolean;
    onerror?: (error: unknown) => void;
  } = $props();

  let href = $derived(`/albums/${encodeURIComponent(album.id)}`);

  function onmenu(event: MouseEvent) {
    openMenu(event, () => albumMenu(album));
  }
</script>

<article class="card" style:width={fluid ? "100%" : `${size}px`} oncontextmenu={onmenu}>
  <div class="cover">
    <!-- The title link is the one announced; this one is only for the mouse. -->
    <a {href} tabindex="-1" aria-hidden="true">
      <Cover id={album.coverArt} {size} />
    </a>
    {#if album.starred}
      <span class="badge" role="img" aria-label={m.card_starred()} title={m.card_starred()}>
        <Heart size={12} fill="currentColor" />
      </span>
    {/if}
    <PlayButton
      label={m.card_play({ name: album.name })}
      centered={large}
      onplay={() => playAlbum(album.id).catch((e) => onerror?.(e))}
    />
    <MoreButton {onmenu} />
  </div>
  {#if label}
    <span class="label">{label}</span>
  {/if}
  <a class="title" {href} title={album.name}>{album.name}</a>
  {#if album.artist && album.artistId}
    <a class="subtitle" href={`/artists/${encodeURIComponent(album.artistId)}`} title={album.artist}>{album.artist}</a>
  {:else if album.artist}
    <span class="subtitle" title={album.artist}>{album.artist}</span>
  {/if}
</article>

<style>
  .card {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .cover {
    position: relative;
    margin-bottom: var(--space-2);
  }

  .cover a {
    display: block;
  }

  /* Discreet favorite badge (spec 6.4). */
  .badge {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-pill);
    background: var(--color-scrim);
    color: var(--color-text);
    pointer-events: none;
  }

  .label {
    color: var(--color-accent-text);
    font-size: 11px;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .title,
  .subtitle {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    color: var(--color-text);
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    text-decoration: none;
  }

  .title:hover {
    text-decoration: underline;
  }

  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    text-decoration: none;
  }

  a.subtitle:hover {
    color: var(--color-text);
    text-decoration: underline;
  }
</style>
