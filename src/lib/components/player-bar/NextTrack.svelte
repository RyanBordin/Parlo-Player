<script lang="ts">
  // Zone 5 of the Player Bar (spec 4.3): the song Next would play, mirroring
  // zone 1 (text on the right, the cover at the edge), with no label. One
  // target: a click plays it, as Next does; Play shows over the cover on
  // hover. Empty at the end of the queue without repeat.
  import Play from "@lucide/svelte/icons/play";
  import { next } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { nextTrack, queue } from "#lib/player.svelte.ts";

  let { onerror }: { onerror: (error: unknown) => void } = $props();

  let upcoming = $derived(nextTrack(queue));
</script>

{#if upcoming}
  <button
    type="button"
    class="next"
    onclick={() => next().catch(onerror)}
    aria-label={m.player_next_track({ title: upcoming.title })}
    title={m.player_next_track({ title: upcoming.title })}
  >
    <span class="meta">
      <span class="title">{upcoming.title}</span>
      {#if upcoming.artist}<span class="artist">{upcoming.artist}</span>{/if}
      {#if upcoming.album}<span class="album">{upcoming.album}</span>{/if}
    </span>
    <span class="thumb">
      <Cover id={upcoming.coverArt} size={56} radius="var(--radius-sm)" />
      <span class="play" aria-hidden="true"><Play size={20} fill="currentColor" /></span>
    </span>
  </button>
{/if}

<style>
  .next {
    max-width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: right;
    cursor: pointer;
  }

  .meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }

  .meta > * {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: var(--font-weight-bold);
  }

  .artist {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .album {
    color: var(--color-text-subtle);
    font-size: var(--font-size-sm);
  }

  .thumb {
    position: relative;
    flex: none;
    width: 56px;
  }

  .play {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-sm);
    background: var(--color-scrim);
    color: var(--color-text);
    opacity: 0;
    transition: opacity var(--transition-fast);
  }

  .next:hover .play,
  .next:focus-visible .play {
    opacity: 1;
  }

  .next:focus-visible {
    border-radius: var(--radius-sm);
    outline: 2px solid var(--color-accent-text);
    outline-offset: 2px;
  }

  /* Spec 4.3, narrow window: first only the cover, then nothing. */
  @media (max-width: 1399px) {
    .meta {
      display: none;
    }
  }

  @media (max-width: 1199px) {
    .next {
      display: none;
    }
  }
</style>
