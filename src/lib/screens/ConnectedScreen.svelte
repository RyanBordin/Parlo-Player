<script lang="ts">
  // Temporary main screen (step 3). Replaced by the app shell in step 6.
  import { logout, type SavedSession } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";

  let { session, onLogout }: { session: SavedSession; onLogout: () => void } = $props();

  let busy = $state(false);
  let error = $state<ErrorMessage | null>(null);

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
  <button class="button button-secondary" onclick={signOut} disabled={busy}>
    {m.home_sign_out()}
  </button>
  {#if error}
    <p class="error">{error.message}</p>
  {/if}
</div>

<style>
  .screen {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    padding: var(--space-4);
    text-align: center;
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
