<script lang="ts">
  // First library sync (spec 6.11). Later syncs run in the background.
  import logoWordmark from "#lib/assets/logo-wordmark.png";
  import { describeError } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { phaseLabel, progressLabel, startSync, syncState } from "#lib/sync.svelte.ts";

  let { onDone }: { onDone: () => void } = $props();

  // Bar fill (0–1) when the total is known; null shows an indeterminate bar.
  let fraction = $derived.by(() => {
    const progress = syncState.progress;
    if (!progress?.total) return null;
    return Math.min(progress.done / progress.total, 1);
  });

  let error = $derived(syncState.error ? describeError(syncState.error) : null);

  // A finished sync without error means the library is ready.
  let initialCount = syncState.finishedCount;
  $effect(() => {
    if (syncState.finishedCount > initialCount && !syncState.error) onDone();
  });
</script>

<div class="screen">
  <img class="logo" src={logoWordmark} alt={m.login_logo_alt()} width="120" height="138" />
  <h1>{m.sync_first_title()}</h1>
  <p class="muted">{m.sync_first_subtitle()}</p>

  {#if error}
    <div class="error-box" aria-live="polite">
      <p class="error">{error.message}</p>
      {#if error.detail}
        <p class="detail">{error.detail}</p>
      {/if}
      <button class="button button-primary" onclick={() => startSync(false)}>
        {m.sync_retry()}
      </button>
    </div>
  {:else}
    <div class="progress" aria-live="polite">
      <div class="bar" class:indeterminate={fraction === null}>
        <div class="fill" style:width={fraction === null ? null : `${fraction * 100}%`}></div>
      </div>
      <p class="phase">{phaseLabel(syncState.progress)}</p>
      <p class="muted count">{progressLabel(syncState.progress) ?? " "}</p>
    </div>
  {/if}
</div>

<style>
  .screen {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }

  .logo {
    margin-bottom: var(--space-4);
  }

  h1 {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  .muted {
    color: var(--color-text-muted);
    max-width: 420px;
  }

  .progress {
    width: 100%;
    max-width: 380px;
    margin-top: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .bar {
    height: 6px;
    border-radius: var(--radius-pill);
    background: var(--color-surface-hover);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: var(--radius-pill);
    background: var(--color-accent-text);
    transition: width var(--transition-fast);
  }

  .indeterminate .fill {
    width: 30%;
    animation: slide 1.2s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }

  .phase {
    font-weight: var(--font-weight-medium);
  }

  .count {
    font-size: var(--font-size-sm);
  }

  .error-box {
    margin-top: var(--space-5);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    max-width: 420px;
  }

  .error {
    color: var(--color-error);
  }

  .detail {
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    overflow-wrap: anywhere;
  }
</style>
