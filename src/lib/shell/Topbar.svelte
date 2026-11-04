<script lang="ts">
  // Topbar (spec 4.1): Back / Forward, global search and profile.
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import type { SavedSession } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { canGoBack, canGoForward } from "#lib/shell/history.svelte.ts";
  import ProfileMenu from "#lib/shell/ProfileMenu.svelte";
  import SearchField from "#lib/shell/SearchField.svelte";
  import { testLibrary } from "#lib/test-library.svelte.ts";

  let { session }: { session: SavedSession } = $props();
</script>

<header class="topbar">
  <div class="history">
    <button
      class="round"
      type="button"
      onclick={() => history.back()}
      disabled={!canGoBack()}
      aria-label={m.topbar_back()}
      title={m.topbar_back()}
    >
      <ChevronLeft />
    </button>
    <button
      class="round"
      type="button"
      onclick={() => history.forward()}
      disabled={!canGoForward()}
      aria-label={m.topbar_forward()}
      title={m.topbar_forward()}
    >
      <ChevronRight />
    </button>
    <span class="divider" aria-hidden="true"></span>
  </div>

  <SearchField />

  <div class="spacer"></div>

  {#if testLibrary.enabled}
    <span class="test-badge" title={m.perf_badge_title()}>{m.perf_badge()}</span>
  {/if}
  <ProfileMenu {session} />
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4) var(--page-gutter);
  }

  /* Performance test mode only: this is not the real library. */
  .test-badge {
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-pill);
    background: var(--color-accent);
    color: var(--color-text);
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .history {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .round {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    cursor: pointer;
  }

  .round:not(:disabled):hover {
    background: var(--color-surface-hover);
  }

  .round:disabled {
    color: var(--color-text-muted);
    opacity: 0.4;
    cursor: default;
  }

  .divider {
    width: 1px;
    height: 24px;
    margin-left: var(--space-1);
    background: var(--color-border);
  }

  .spacer {
    flex: 1;
  }
</style>
