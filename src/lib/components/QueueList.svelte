<script lang="ts">
  // Temporary queue list (step 5B). The real one is the "A SEGUIR" tab (spec 4.4).
  import { playQueueIndex } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { queue } from "#lib/player.svelte.ts";

  let error = $state<ErrorMessage | null>(null);

  // Spec 5.6: a queue continued from another app says where it came from.
  let origin = $derived(
    queue.continuingFrom === null
      ? m.queue_playing_from({ label: queue.label })
      : queue.continuingFrom
        ? m.queue_continuing_from({ app: queue.continuingFrom })
        : m.queue_continuing_from_other(),
  );

  // Spec 5.1: clicking a song of the queue only jumps to it.
  async function jump(index: number) {
    error = null;
    try {
      await playQueueIndex(index);
    } catch (e) {
      error = describeError(e);
    }
  }
</script>

<section class="queue">
  <h2>{m.queue_title()}</h2>
  {#if queue.tracks.length === 0}
    <p class="muted">{m.queue_empty()}</p>
  {:else}
    <p class="muted">{origin}</p>
    <ol>
      {#each queue.tracks as track, index (`${index}-${track.id}`)}
        <li>
          <button class:current={index === queue.currentIndex} onclick={() => jump(index)}>
            <span class="number">{index + 1}</span>
            <span class="title">{track.title}</span>
            <span class="muted">{track.artist ?? ""}</span>
            <span class="muted duration">{formatDuration(track.duration)}</span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}
  {#if error}
    <p class="error">{error.message}</p>
  {/if}
</section>

<style>
  .queue {
    width: 100%;
    max-width: 720px;
    text-align: left;
  }

  h2 {
    margin-bottom: var(--space-2);
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .queue > .muted {
    margin-bottom: var(--space-3);
    font-size: var(--font-size-sm);
  }

  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  button {
    width: 100%;
    display: grid;
    grid-template-columns: 2.5em 2fr 1.5fr auto;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  button:hover {
    background: var(--color-surface);
  }

  span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .number,
  .duration {
    color: var(--color-text-muted);
    font-variant-numeric: tabular-nums;
  }

  /* Current song in the accent color (spec 5.5). */
  button.current .title,
  button.current .number {
    color: var(--color-accent-text);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
