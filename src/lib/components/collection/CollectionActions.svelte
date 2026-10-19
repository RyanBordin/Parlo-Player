<script lang="ts">
  // Action bar of album and playlist pages (spec 6.5): Play (bigger circle),
  // Shuffle, Favorite (albums) and "..." (menus come in step 8).
  import Ellipsis from "@lucide/svelte/icons/ellipsis";
  import Heart from "@lucide/svelte/icons/heart";
  import Play from "@lucide/svelte/icons/play";
  import Shuffle from "@lucide/svelte/icons/shuffle";
  import { m } from "#lib/paraglide/messages.js";

  let {
    disabled = false,
    starred = null,
    onplay,
    onshuffle,
    onstar,
  }: {
    /** No songs to play. */
    disabled?: boolean;
    /** `null`: no heart (playlists cannot be starred). */
    starred?: boolean | null;
    onplay: () => void;
    onshuffle: () => void;
    onstar?: () => void;
  } = $props();
</script>

<div class="actions">
  <button type="button" class="play" onclick={onplay} {disabled} aria-label={m.collection_play()} title={m.collection_play()}>
    <Play size={24} fill="currentColor" />
  </button>
  <button type="button" class="icon" onclick={onshuffle} {disabled} aria-label={m.collection_shuffle()} title={m.collection_shuffle()}>
    <Shuffle size={22} />
  </button>
  {#if starred !== null}
    <button
      type="button"
      class="icon"
      class:active={starred}
      onclick={onstar}
      aria-label={starred ? m.player_unfavorite() : m.player_favorite()}
      aria-pressed={starred}
      title={starred ? m.player_unfavorite() : m.player_favorite()}
    >
      <Heart size={22} fill={starred ? "currentColor" : "none"} />
    </button>
  {/if}
  <button type="button" class="icon" aria-label={m.player_more()} title={m.player_more()}>
    <Ellipsis size={22} />
  </button>
</div>

<style>
  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-5);
  }

  .play {
    width: 56px;
    height: 56px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-text);
    color: var(--color-text-inverse);
    cursor: pointer;
    transition: transform var(--transition-fast);
  }

  .play:not(:disabled):hover {
    transform: scale(1.05);
  }

  .icon {
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .icon:not(:disabled):hover,
  .icon.active {
    color: var(--color-text);
  }

  button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
