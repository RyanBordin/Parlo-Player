<script lang="ts">
  // Banner of the artist page (spec 6.7), after YouTube Music: the artist's
  // photo across the page, darker at the bottom; without a photo, the latest
  // album's cover enlarged and blurred (as on album pages). Over it, the big
  // name, the biography cut at 3 lines with "More", and the action bar.
  import type { Snippet } from "svelte";
  import type { ArtistAbout, ArtistHeader } from "#lib/api.ts";
  import { coverUrl } from "#lib/covers.ts";
  import { m } from "#lib/paraglide/messages.js";

  /** Photo width asked for: the banner spans the page. */
  const PHOTO = 1024;

  let {
    artist,
    about,
    children,
  }: {
    artist: ArtistHeader;
    /** `undefined` while it loads: the banner waits, so it does not flash the wrong image. */
    about: ArtistAbout | null | undefined;
    /** The action bar. */
    children: Snippet;
  } = $props();

  let photo = $derived.by(() => {
    if (about === undefined || !artist.coverArt || about?.hasPhoto === false) return null;
    // Unknown (no connection on the first visit): the cover protocol swaps in
    // the album cover if the server only has its generic image.
    const fallback = about?.hasPhoto ? null : artist.fallbackCoverArt;
    return coverUrl(artist.coverArt, PHOTO, fallback);
  });
  /** The photo URL that failed to load (offline and not cached). */
  let failedPhoto = $state<string | null>(null);
  let showPhoto = $derived(photo !== null && photo !== failedPhoto);
  // A small image is enough: it is blurred anyway.
  let backdrop = $derived(
    !showPhoto && about !== undefined && artist.fallbackCoverArt ? coverUrl(artist.fallbackCoverArt, 128) : null,
  );

  let biography = $derived(about?.biography ?? null);
  let expanded = $state(false);
  /** The biography is longer than 3 lines: "More" shows. */
  let overflowing = $state(false);
  let bio = $state<HTMLElement>();

  $effect(() => {
    void biography;
    const element = bio;
    if (!element) return;
    const check = () => {
      if (!expanded) overflowing = element.scrollHeight > element.clientHeight + 1;
    };
    check();
    const observer = new ResizeObserver(check);
    observer.observe(element);
    return () => observer.disconnect();
  });
</script>

<header class="banner" class:with-photo={showPhoto}>
  {#if showPhoto}
    <img class="photo" src={photo} alt="" aria-hidden="true" onerror={() => (failedPhoto = photo)} />
  {:else if backdrop}
    <img class="backdrop" src={backdrop} alt="" aria-hidden="true" />
  {/if}
  <div class="shade"></div>

  <div class="content">
    <h1 class="name">{artist.name}</h1>
    {#if biography}
      <!-- Plain text from the backend: shown as text, never as HTML. -->
      <p class="bio" class:expanded bind:this={bio}>{biography}</p>
      {#if overflowing || expanded}
        <button type="button" class="more" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
          {expanded ? m.artist_bio_less() : m.artist_bio_more()}
        </button>
      {/if}
    {/if}
    {@render children()}
  </div>
</header>

<style>
  .banner {
    position: relative;
    min-height: clamp(320px, 48vh, 520px);
    display: flex;
    align-items: flex-end;
    overflow: hidden;
    padding: var(--space-6) var(--page-gutter) var(--space-5);
  }

  .photo {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    /* Faces are usually in the upper part of artist photos. */
    object-position: center 25%;
    animation: fade-in 300ms ease-out;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    filter: blur(64px) saturate(1.3);
    opacity: 0.6;
    transform: scale(1.4);
    animation: fade-in 300ms ease-out;
  }

  /* Dark gradient down to the page's black, as on album pages. */
  .shade {
    position: absolute;
    inset: 0;
    background: linear-gradient(to bottom, rgb(0 0 0 / 15%), rgb(0 0 0 / 55%) 60%, var(--color-bg));
  }

  /* Over a photo: darker at the bottom and on the left, where the text is. */
  .with-photo .shade {
    background:
      linear-gradient(to bottom, rgb(0 0 0 / 0%) 30%, rgb(0 0 0 / 70%) 75%, var(--color-bg)),
      linear-gradient(to right, rgb(0 0 0 / 45%), rgb(0 0 0 / 0%) 65%);
  }

  .content {
    position: relative;
    max-width: 760px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .name {
    display: -webkit-box;
    overflow: hidden;
    font-size: clamp(40px, 6vw, 72px);
    font-weight: var(--font-weight-bold);
    letter-spacing: -0.02em;
    line-height: 1.05;
    text-shadow: 0 2px 24px rgb(0 0 0 / 35%);
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }

  .bio {
    display: -webkit-box;
    overflow: hidden;
    color: rgb(255 255 255 / 80%);
    font-size: 14px;
    line-height: 1.5;
    /* One line per paragraph, as the backend sends it. */
    white-space: pre-line;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }

  .bio.expanded {
    display: block;
  }

  .more {
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .more:hover {
    text-decoration: underline;
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }
</style>
