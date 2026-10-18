<script lang="ts">
  // Temporary main screen (steps 3–5A). Replaced by the app shell in step 6.
  import {
    getLibraryStatus,
    listSongs,
    logout,
    playSong,
    type LibraryStatus,
    type SavedSession,
    type SongSummary,
  } from "#lib/api.ts";
  import PlayerBar from "#lib/components/PlayerBar.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDateTime, formatDuration, formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { listenToPlayer, player } from "#lib/player.svelte.ts";
  import { phaseLabel, progressLabel, startSync, syncState } from "#lib/sync.svelte.ts";

  const SONG_LIST_SIZE = 20;

  let { session, onLogout }: { session: SavedSession; onLogout: () => void } = $props();

  let status = $state<LibraryStatus | null>(null);
  let songs = $state<SongSummary[]>([]);
  let busy = $state(false);
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

  $effect(() => {
    listenToPlayer();
  });

  async function play(song: SongSummary) {
    error = null;
    try {
      await playSong(song.id);
    } catch (e) {
      error = describeError(e);
    }
  }

  async function signOut() {
    busy = true;
    error = null;
    try {
      await logout();
      onLogout();
    } catch (e) {
      error = describeError(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="screen">
  <p>{m.home_connected_as({ username: session.username, server: session.serverUrl })}</p>

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
  </div>

  <div class="actions">
    <button class="button button-primary" onclick={() => startSync(true)} disabled={syncState.running}>
      {syncState.running ? m.home_syncing() : m.home_sync_now()}
    </button>
    <button class="button button-secondary" onclick={signOut} disabled={busy}>
      {m.home_sign_out()}
    </button>
  </div>

  {#if error}
    <p class="error">{error.message}</p>
  {/if}

  <section class="songs">
    <h2>{m.songs_recent_title()}</h2>
    {#if songs.length === 0}
      <p class="muted">{m.songs_empty()}</p>
    {:else}
      <ol>
        {#each songs as song (song.id)}
          <li>
            <button class:current={player.track?.id === song.id} onclick={() => play(song)}>
              <span class="song-title">{song.title}</span>
              <span class="muted">{song.artist ?? ""}</span>
              <span class="muted duration">{formatDuration(song.duration)}</span>
            </button>
          </li>
        {/each}
      </ol>
    {/if}
  </section>
</div>

<PlayerBar />

<style>
  .screen {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-5);
    /* Room at the bottom for the fixed player bar. */
    padding: var(--space-6) var(--space-4) 140px;
    text-align: center;
  }

  .songs {
    width: 100%;
    max-width: 720px;
    text-align: left;
  }

  h2 {
    margin-bottom: var(--space-3);
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li button {
    width: 100%;
    display: grid;
    grid-template-columns: 2fr 1.5fr auto;
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

  /* Current song: accent color, like the track indicator of spec 5.5. */
  li button.current .song-title {
    color: var(--color-accent-text);
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }

  .stats {
    display: flex;
    gap: var(--space-4);
    margin: 0;
  }

  .stats div {
    min-width: 120px;
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
    gap: var(--space-2);
    font-size: var(--font-size-sm);
  }

  .actions {
    display: flex;
    gap: var(--space-3);
  }

  .muted {
    color: var(--color-text-muted);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
