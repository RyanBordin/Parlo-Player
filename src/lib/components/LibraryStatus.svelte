<script lang="ts">
  // Temporary library status ("Sync now"), until the Library settings (v0.2).
  import { getLibraryStatus, type LibraryStatus } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatDateTime, formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { phaseLabel, progressLabel, startSync, syncState } from "#lib/sync.svelte.ts";

  let status = $state<LibraryStatus | null>(null);
  let error = $state<ErrorMessage | null>(null);

  let syncError = $derived(syncState.error ? describeError(syncState.error) : null);
  let lastSync = $derived.by(() => {
    const dates = [status?.lastFullSyncAt, status?.lastLightSyncAt].filter((d) => d != null);
    return dates.length ? Math.max(...dates) : null;
  });

  // On open and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    getLibraryStatus()
      .then((s) => (status = s))
      .catch((e) => (error = describeError(e)));
  });
</script>

<section>
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
    {#if syncError ?? error}
      <p class="error">{(syncError ?? error)!.message}</p>
    {/if}
    <button class="button button-secondary" onclick={() => startSync(true)} disabled={syncState.running}>
      {syncState.running ? m.home_syncing() : m.home_sync_now()}
    </button>
  </div>
</section>

<style>
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
  }
</style>
