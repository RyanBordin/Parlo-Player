<script lang="ts">
  // Discreet indicator of "Save to computer" (spec 8.2), above the Player
  // Bar: "Saving 3 of 12" with Cancel; at the end, what was saved with "Open
  // folder", the files that failed, or why the server refused.
  import X from "@lucide/svelte/icons/x";
  import { cancelSaves, openSaveFolder } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { dismissSaveResult, saves } from "#lib/saves.svelte.ts";
  import { showToast } from "#lib/toasts.svelte.ts";

  let progress = $derived(saves.progress);
  let finished = $derived(saves.finished);

  function run(action: () => Promise<unknown>) {
    action().catch((e) => showToast(describeError(e).message));
  }
</script>

{#if progress || finished}
  <section class="save" aria-label={m.save_label()} role="status" aria-live="polite">
    {#if progress}
      <div class="line">
        <p>{m.save_progress({ done: formatNumber(progress.done), total: formatNumber(progress.total) })}</p>
        <button type="button" class="link" onclick={() => run(cancelSaves)}>{m.save_cancel()}</button>
      </div>
      <div class="bar" aria-hidden="true">
        <span style:width={`${progress.total ? (progress.done / progress.total) * 100 : 0}%`}></span>
      </div>
    {:else if finished}
      <div class="line">
        <div class="summary">
          {#if finished.forbidden}
            <p class="error">{m.save_forbidden()}</p>
          {:else if finished.cancelled}
            <p>{m.save_cancelled()}</p>
          {:else}
            {#if finished.saved > 0 || finished.skipped === 0}
              <p>{m.save_saved({ count: finished.saved, countText: formatNumber(finished.saved) })}</p>
            {/if}
            {#if finished.skipped > 0}
              <p class="muted">{m.save_skipped({ count: finished.skipped, countText: formatNumber(finished.skipped) })}</p>
            {/if}
          {/if}
        </div>
        <button type="button" class="close" onclick={dismissSaveResult} aria-label={m.save_close()} title={m.save_close()}>
          <X size={16} />
        </button>
      </div>
      {#if finished.failed.length > 0}
        <p class="error">
          {m.save_failed({ count: finished.failed.length, countText: formatNumber(finished.failed.length) })}
        </p>
        <ul class="failed">
          {#each finished.failed as name, index (index)}
            <li title={name}>{name}</li>
          {/each}
        </ul>
      {/if}
      {#if !finished.forbidden && !finished.cancelled && finished.saved + finished.skipped > 0}
        <button type="button" class="button button-secondary open" onclick={() => run(openSaveFolder)}>
          {m.save_open_folder()}
        </button>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .save {
    position: fixed;
    left: calc(var(--sidebar-width) + var(--space-5));
    bottom: calc(var(--player-bar-height) + var(--space-4));
    z-index: 20;
    width: 320px;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
    font-size: 14px;
  }

  .line {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .summary {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .link {
    flex: none;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-accent-text);
    font: inherit;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
  }

  .close {
    flex: none;
    display: grid;
    place-items: center;
    padding: var(--space-1);
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .close:hover {
    color: var(--color-text);
  }

  .bar {
    height: 4px;
    overflow: hidden;
    border-radius: var(--radius-pill);
    background: var(--color-surface-active);
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--color-accent-text);
    transition: width var(--transition-fast);
  }

  .failed {
    max-height: 120px;
    margin: 0;
    padding-left: var(--space-4);
    overflow-y: auto;
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .failed li {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .open {
    align-self: flex-start;
    padding: var(--space-1) var(--space-4);
    font-size: var(--font-size-sm);
  }
</style>
