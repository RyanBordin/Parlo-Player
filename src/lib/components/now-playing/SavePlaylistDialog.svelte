<script lang="ts">
  // "Save" of Up Next (spec 4.4, 9): the queue as a new playlist on the
  // server, with a suggested name ("Albums, 10/18/2026") that can be edited.
  import { untrack } from "svelte";
  import { createPlaylist } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatSongCount } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { loadPlaylists } from "#lib/playlists.svelte.ts";
  import { startSync } from "#lib/sync.svelte.ts";
  import { showToast } from "#lib/toasts.svelte.ts";

  let {
    songIds,
    suggestedName,
    onclose,
  }: {
    songIds: string[];
    suggestedName: string;
    onclose: () => void;
  } = $props();

  let dialog = $state<HTMLDialogElement>();
  let input = $state<HTMLInputElement>();
  // Edited from here on; the suggestion is only where it starts.
  let name = $state(untrack(() => suggestedName.trim()));
  let saving = $state(false);
  let error = $state<ErrorMessage | null>(null);

  $effect(() => {
    dialog?.showModal();
    input?.select();
  });

  async function save(event: SubmitEvent) {
    event.preventDefault();
    const trimmed = name.trim();
    if (!trimmed || saving) return;
    saving = true;
    error = null;
    try {
      const shownNow = await createPlaylist(trimmed, songIds);
      if (shownNow) {
        await loadPlaylists();
        showToast(m.playlist_created({ name: trimmed }));
      } else {
        // The server did not send the playlist back: a light sync brings it.
        startSync(false);
        showToast(m.playlist_created_later({ name: trimmed }));
      }
      dialog?.close();
    } catch (e) {
      error = describeError(e);
      saving = false;
    }
  }
</script>

<!-- Esc and the backdrop close it without saving ("close" event). -->
<dialog bind:this={dialog} onclose={onclose} aria-labelledby="save-playlist-title">
  <form method="dialog" onsubmit={save}>
    <h2 id="save-playlist-title">{m.save_playlist_title()}</h2>
    <p class="muted">{formatSongCount(songIds.length)}</p>

    <label>
      <span>{m.save_playlist_name()}</span>
      <input class="input" bind:this={input} bind:value={name} maxlength="200" required />
    </label>

    {#if error}
      <p class="error">{error.message}</p>
    {/if}

    <div class="buttons">
      <button type="button" class="button button-secondary" onclick={() => dialog?.close()}>
        {m.save_playlist_cancel()}
      </button>
      <button type="submit" class="button button-primary" disabled={saving || !name.trim()}>
        {saving ? m.save_playlist_saving() : m.save_playlist_save()}
      </button>
    </div>
  </form>
</dialog>

<style>
  dialog {
    width: min(440px, calc(100vw - 48px));
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: 0 24px 64px rgb(0 0 0 / 60%);
  }

  dialog::backdrop {
    background: rgb(0 0 0 / 60%);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  h2 {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
  }

  label .input {
    background: var(--color-bg);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
</style>
