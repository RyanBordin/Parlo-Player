<script lang="ts">
  // Now Playing (spec 4.4): rises over the main area; the sidebar and the
  // Player Bar stay visible. Big cover on the left, three tabs on the right.
  import { cubicOut } from "svelte/easing";
  import Cover from "#lib/components/Cover.svelte";
  import Lyrics from "#lib/components/now-playing/Lyrics.svelte";
  import Related from "#lib/components/now-playing/Related.svelte";
  import UpNext from "#lib/components/now-playing/UpNext.svelte";
  import { NOW_PLAYING_TABS, nowPlaying, setNowPlayingTab, type NowPlayingTab } from "#lib/now-playing.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";
  import { reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  /** Asked as 1024 px on any screen: the cover in high resolution. */
  const COVER_SIZE = 600;

  const LABELS: Record<NowPlayingTab, () => string> = {
    upNext: m.now_playing_up_next,
    lyrics: m.now_playing_lyrics,
    related: m.now_playing_related,
  };

  /** Rises from the bottom (and goes back down); at once with reduced motion. */
  function rise(_node: Element) {
    return {
      duration: reducedMotion() ? 0 : 280,
      easing: cubicOut,
      css: (t: number) => `transform: translateY(${(1 - t) * 100}%)`,
    };
  }
</script>

<section class="now-playing" transition:rise aria-label={m.now_playing_tabs()}>
  <div class="art">
    <div class="frame">
      <Cover id={player.track?.coverArt} size={COVER_SIZE} radius="var(--radius-lg)" alt={player.track?.title ?? ""} />
    </div>
    {#if player.track}
      <p class="title">{player.track.title}</p>
      {#if player.track.artist}<p class="artist">{player.track.artist}</p>{/if}
    {:else}
      <p class="artist">{m.player_nothing_playing()}</p>
    {/if}
  </div>

  <div class="panel">
    <div class="tabs" role="tablist" aria-label={m.now_playing_tabs()}>
      {#each NOW_PLAYING_TABS as tab (tab)}
        <button
          type="button"
          role="tab"
          id={`now-playing-tab-${tab}`}
          aria-selected={nowPlaying.tab === tab}
          aria-controls="now-playing-panel"
          class:active={nowPlaying.tab === tab}
          onclick={() => setNowPlayingTab(tab)}
        >
          {LABELS[tab]()}
        </button>
      {/each}
    </div>
    <div class="content" id="now-playing-panel" role="tabpanel" aria-labelledby={`now-playing-tab-${nowPlaying.tab}`}>
      <!-- Each tab loads only when it is opened. -->
      {#if nowPlaying.tab === "upNext"}
        <UpNext />
      {:else if nowPlaying.tab === "lyrics"}
        <Lyrics />
      {:else}
        <Related />
      {/if}
    </div>
  </div>
</section>

<style>
  .now-playing {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: grid;
    grid-template-columns: minmax(0, 52fr) minmax(0, 48fr);
    gap: var(--space-6);
    padding: var(--space-5) var(--page-gutter) 0;
    background: var(--color-bg);
  }

  .art {
    min-height: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding-bottom: var(--space-5);
    text-align: center;
  }

  /* Square, as big as the column and the height allow. */
  .frame {
    width: min(100%, calc(100vh - var(--player-bar-height) - 300px));
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-dialog);
  }

  .title {
    margin-top: var(--space-4);
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  .artist {
    color: var(--color-text-muted);
  }

  .panel {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid var(--color-border);
  }

  .tabs button {
    flex: 1;
    padding: var(--space-3) var(--space-2);
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
  }

  .tabs button:hover {
    color: var(--color-text);
  }

  .tabs button.active {
    border-bottom-color: var(--color-text);
    color: var(--color-text);
  }

  .content {
    flex: 1;
    min-height: 0;
  }

</style>
