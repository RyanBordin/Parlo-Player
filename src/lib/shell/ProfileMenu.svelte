<script lang="ts">
  // Profile button of the Topbar: who is signed in, on which server, and sign out.
  import LogOut from "@lucide/svelte/icons/log-out";
  import UserRound from "@lucide/svelte/icons/user-round";
  import type { SavedSession } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { signOut } from "#lib/session.svelte.ts";

  let { session }: { session: SavedSession } = $props();

  let open = $state(false);
  let busy = $state(false);
  let error = $state<ErrorMessage | null>(null);
  let root = $state<HTMLElement>();

  async function onSignOut() {
    busy = true;
    error = null;
    try {
      await signOut();
    } catch (e) {
      error = describeError(e);
      busy = false;
    }
  }

  // Closes on a click outside the menu or on Escape.
  function onWindowClick(event: MouseEvent) {
    if (open && root && !root.contains(event.target as Node)) open = false;
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKeydown} />

<div class="profile" bind:this={root}>
  <button
    class="avatar"
    type="button"
    aria-label={m.profile_label()}
    title={m.profile_label()}
    aria-haspopup="true"
    aria-expanded={open}
    aria-controls="profile-menu"
    onclick={() => (open = !open)}
  >
    <UserRound />
  </button>

  {#if open}
    <div class="menu" id="profile-menu">
      <dl>
        <div>
          <dt>{m.profile_user()}</dt>
          <dd>{session.username}</dd>
        </div>
        <div>
          <dt>{m.profile_server()}</dt>
          <dd title={session.serverUrl}>{session.serverUrl}</dd>
        </div>
      </dl>
      <hr />
      <button class="item" type="button" onclick={onSignOut} disabled={busy}>
        <LogOut />
        <span>{m.home_sign_out()}</span>
      </button>
      {#if error}
        <p class="error">{error.message}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .profile {
    position: relative;
  }

  .avatar {
    width: 36px;
    height: 36px;
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: var(--color-surface-active);
    color: var(--color-text);
    cursor: pointer;
  }

  .avatar:hover,
  .avatar[aria-expanded="true"] {
    background: var(--color-surface-hover);
  }

  .menu {
    position: absolute;
    top: calc(100% + var(--space-2));
    right: 0;
    z-index: 10;
    width: 260px;
    padding: var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-popover);
  }

  dl {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-2) var(--space-3);
  }

  dt {
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
  }

  dd {
    margin: 0;
    overflow: hidden;
    font-size: 14px;
    font-weight: var(--font-weight-medium);
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  hr {
    margin: var(--space-2) 0;
    border: none;
    border-top: 1px solid var(--color-border);
  }

  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }

  .item:not(:disabled):hover {
    background: var(--color-surface-hover);
  }

  .item:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .error {
    padding: var(--space-2) var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
