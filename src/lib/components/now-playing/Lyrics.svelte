<script lang="ts">
  // "Lyrics" tab (spec 4.4). Synced lyrics: the current line stands out and
  // scrolls into the middle; a click on a line jumps there; scrolling by hand
  // pauses the automatic scrolling for a few seconds. Plain lyrics: the text.
  import { getLyrics, seek } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";
  import { keepLyrics, lyricsBySong } from "#lib/song-cache.svelte.ts";

  /** After scrolling by hand, the lyrics stay put this long. */
  const MANUAL_SCROLL_PAUSE_MS = 4000;

  let songId = $derived(player.track?.id ?? null);
  let lyrics = $derived(songId ? lyricsBySong[songId] : undefined);
  let error = $state<ErrorMessage | null>(null);
  let scroller = $state<HTMLElement>();
  const lineElements: HTMLElement[] = [];
  let pausedUntil = 0;
  let resumeTimer: ReturnType<typeof setTimeout> | undefined;

  // Asked when the tab opens and when the song changes; kept per song.
  $effect(() => {
    const id = songId;
    if (!id || lyricsBySong[id]) return;
    error = null;
    getLyrics(id)
      .then((result) => keepLyrics(id, result))
      .catch((e) => (error = describeError(e)));
  });

  // The current line: the last one started. A positive offset shows the
  // lines sooner (OpenSubsonic).
  let current = $derived.by(() => {
    if (lyrics?.kind !== "synced") return -1;
    const now = player.position * 1000 + lyrics.offsetMs;
    let low = 0;
    let high = lyrics.lines.length;
    while (low < high) {
      const mid = (low + high) >> 1;
      if (lyrics.lines[mid].startMs <= now) low = mid + 1;
      else high = mid;
    }
    return low - 1;
  });

  function scrollToCurrent(smooth = true) {
    const line = lineElements[current];
    if (!scroller || !line) return;
    const top = line.offsetTop - scroller.clientHeight / 2 + line.clientHeight / 2;
    scroller.scrollTo({ top: Math.max(0, top), behavior: smooth ? "smooth" : "instant" });
  }

  $effect(() => {
    void current;
    if (Date.now() >= pausedUntil) scrollToCurrent();
  });

  // Scrolling by hand (wheel, touch, scrollbar, keys) pauses the automatic one.
  function scrolledByHand() {
    pausedUntil = Date.now() + MANUAL_SCROLL_PAUSE_MS;
    clearTimeout(resumeTimer);
    resumeTimer = setTimeout(() => scrollToCurrent(), MANUAL_SCROLL_PAUSE_MS);
  }

  $effect(() => () => clearTimeout(resumeTimer));

  function jumpTo(startMs: number, offsetMs: number) {
    pausedUntil = 0;
    clearTimeout(resumeTimer);
    seek(Math.max(0, (startMs - offsetMs) / 1000)).catch((e) => (error = describeError(e)));
  }
</script>

<!-- The listeners only notice scrolling by hand; they do nothing themselves. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="lyrics"
  bind:this={scroller}
  onwheel={scrolledByHand}
  ontouchmove={scrolledByHand}
  onpointerdown={scrolledByHand}
  onkeydown={scrolledByHand}
  role="region"
  aria-label={m.now_playing_lyrics()}
>
  {#if !songId}
    <p class="message">{m.player_nothing_playing()}</p>
  {:else if error && !lyrics}
    <p class="message">{m.lyrics_error()}</p>
  {:else if !lyrics}
    <p class="message">{m.lyrics_loading()}</p>
  {:else if lyrics.kind === "none"}
    <p class="message">{m.lyrics_none()}</p>
  {:else if lyrics.kind === "plain"}
    <p class="plain">{lyrics.text}</p>
  {:else}
    {@const offset = lyrics.offsetMs}
    <ol class="synced">
      {#each lyrics.lines as line, index (index)}
        <li bind:this={lineElements[index]}>
          <button
            type="button"
            class:current={index === current}
            class:past={index < current}
            onclick={() => jumpTo(line.startMs, offset)}
            title={m.lyrics_jump()}
          >
            {line.text || "♪"}
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</div>

<style>
  .lyrics {
    position: relative;
    height: 100%;
    overflow-y: auto;
    padding: var(--space-5) var(--space-2) 40vh;
  }

  .message {
    color: var(--color-text-muted);
  }

  .plain {
    font-size: var(--font-size-lg);
    line-height: 1.7;
    white-space: pre-line;
  }

  .synced {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .synced button {
    width: 100%;
    padding: var(--space-2) 0;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-size: 22px;
    font-weight: var(--font-weight-semibold);
    line-height: 1.35;
    text-align: left;
    cursor: pointer;
    transition:
      color var(--transition-fast),
      opacity var(--transition-fast);
  }

  .synced button.past {
    opacity: 0.55;
  }

  .synced button:hover {
    color: var(--color-text);
    opacity: 1;
  }

  /* The current line (spec 4.4). */
  .synced button.current {
    color: var(--color-text);
    opacity: 1;
  }
</style>
