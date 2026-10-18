<script lang="ts">
  // Home (spec 6.1). The carousels come in step 7; for now it keeps the
  // temporary list of recently added songs, the queue and the library status.
  import { getLibraryStatus, listSongs, playContext, type LibraryStatus, type SongSummary } from "#lib/api.ts";
  import QueueList from "#lib/components/QueueList.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDateTime, formatDuration, formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";
  import { phaseLabel, progressLabel, startSync, syncState } from "#lib/sync.svelte.ts";

  const SONG_LIST_SIZE = 20;

  let status = $state<LibraryStatus | null>(null);
  let songs = $state<SongSummary[]>([]);
  let error = $state<ErrorMessage | null>(null);

  let syncError = $derived(syncState.error ? describeError(syncState.error) : null);
  let lastSync = $derived.by(() => {
    const dates = [status?.lastFullSyncAt, status?.lastLightSyncAt].filter((d) => d != null);
    return dates.length ? Math.max(...dates) : null;
  });

  // Reload the counts and songs on open and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    getLibraryStatus()
      .then((s) => (status = s))
      .catch((e) => (error = describeError(e)));
    listSongs(SONG_LIST_SIZE)
      .then((list) => (songs = list))
      .catch((e) => (error = describeError(e)));
  });

  // Spec 5.1: the queue becomes the whole list, starting at the clicked song.
  async function play(index: number) {
    error = null;
    try {
      await playContext(
        songs.map((s) => s.id),
        index,
        m.songs_recent_title(),
      );
    } catch (e) {
      error = describeError(e);
    }
  }
</script>

<div class="page">
  <h1 class="page-title">{m.nav_home()}</h1>

  {#if error}
    <p class="error">{error.message}</p>
  {/if}

  <section class="songs">
    <h2 class="section-title">{m.songs_recent_title()}</h2>
    {#if songs.length === 0}
      <p class="muted">{m.songs_empty()}</p>
    {:else}
      <ol>
        {#each songs as song, index (song.id)}
          <li>
            <button class:current={player.track?.id === song.id} onclick={() => play(index)}>
              <span class="number">{index + 1}</span>
              <span class="song-title">{song.title}</span>
              <span class="muted">{song.artist ?? ""}</span>
              <span class="muted duration">{formatDuration(song.duration)}</span>
            </button>
          </li>
        {/each}
      </ol>
    {/if}
  </section>

  <QueueList />

  <section class="library">
    <h2 class="section-title">{m.home_library_title()}</h2>
    {#if status}
      <dl class="stats">
        <div>
          <dt>{m.home_stat_songs()}</dt>
          <dd>{formatNumber(status.songs)}</dd>
        </div>
        <div>
          <dt>{m.home_stat_albums()}</dt>
          <dd>{formatNumber(status.albums)}</dd>
        </div>
        <div>
          <dt>{m.home_stat_artists()}</dt>
          <dd>{formatNumber(status.artists)}</dd>
        </div>
      </dl>
    {/if}

    <div class="sync" aria-live="polite">
      {#if syncState.running}
        <p class="muted">
          {m.home_syncing()} · {phaseLabel(syncState.progress)}
          {#if progressLabel(syncState.progress)}({progressLabel(syncState.progress)}){/if}
        </p>
      {:else if lastSync !== null}
        <p class="muted">{m.home_last_sync({ date: formatDateTime(lastSync) })}</p>
      {:else}
        <p class="muted">{m.home_never_synced()}</p>
      {/if}
      {#if syncError}
        <p class="error">{syncError.message}</p>
      {/if}
      <button class="button button-secondary" onclick={() => startSync(true)} disabled={syncState.running}>
        {syncState.running ? m.home_syncing() : m.home_sync_now()}
      </button>
    </div>
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li button {
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

  li button:hover {
    background: var(--color-surface);
  }

  li button span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .number,
  .duration {
    color: var(--color-text-muted);
    font-variant-numeric: tabular-nums;
  }

  /* Current song: accent color, like the track indicator of spec 5.5. */
  li button.current .song-title,
  li button.current .number {
    color: var(--color-accent-text);
  }

  .stats {
    display: flex;
    gap: var(--space-4);
    margin: 0 0 var(--space-4);
  }

  .stats div {
    min-width: 140px;
    padding: var(--space-4) var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
  }

  dt {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  dd {
    margin: var(--space-1) 0 0;
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  .sync {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    font-size: var(--font-size-sm);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
