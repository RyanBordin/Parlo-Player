<script lang="ts">
  // "+ New playlist" (sidebar and "Add to playlist ›", spec 4.2, 8.2: created
  // with the item's songs) and "Rename / edit description" (spec 8.1).
  import { untrack } from "svelte";
  import { createPlaylist, type ItemTarget, type PlaylistSummary } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { editPlaylist } from "#lib/menu/actions.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { loadPlaylists } from "#lib/playlists.svelte.ts";
  import { startSync } from "#lib/sync.svelte.ts";
  import { showToast } from "#lib/toasts.svelte.ts";

  let {
    playlist = null,
    target = null,
    onclose,
  }: {
    /** Set: edit this playlist. Unset: a new one. */
    playlist?: PlaylistSummary | null;
    /** The songs a new playlist starts with (none from the sidebar). */
    target?: ItemTarget | null;
    onclose: () => void;
  } = $props();

  const id = $props.id();
  let dialog = $state<HTMLDialogElement>();
  let input = $state<HTMLInputElement>();
  let name = $state(untrack(() => playlist?.name ?? ""));
  let comment = $state(untrack(() => playlist?.comment ?? ""));
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
      if (playlist) {
        await editPlaylist(playlist, trimmed, comment.trim());
      } else if ((await createPlaylist(trimmed, target ?? { kind: "songs", ids: [] })) !== null) {
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
<dialog bind:this={dialog} {onclose} aria-labelledby={`${id}-title`}>
  <form method="dialog" onsubmit={save}>
    <h2 id={`${id}-title`}>{playlist ? m.playlist_dialog_edit_title() : m.playlist_dialog_new_title()}</h2>

    <label>
      <span>{m.playlist_dialog_name()}</span>
      <input class="input" bind:this={input} bind:value={name} maxlength="200" required />
    </label>
    {#if playlist}
      <label>
        <span>{m.playlist_dialog_description()}</span>
        <textarea class="input" bind:value={comment} rows="3" maxlength="1000"></textarea>
      </label>
    {/if}

    {#if error}
      <p class="error">{error.message}</p>
    {/if}

    <div class="buttons">
      <button type="button" class="button button-secondary" onclick={() => dialog?.close()}>
        {m.save_playlist_cancel()}
      </button>
      <button type="submit" class="button button-primary" disabled={saving || !name.trim()}>
        {#if saving}
          {m.save_playlist_saving()}
        {:else}
          {playlist ? m.playlist_dialog_save() : m.playlist_dialog_create()}
        {/if}
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
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--color-backdrop);
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

  textarea {
    height: auto;
    padding-block: var(--space-2);
    resize: vertical;
    font: inherit;
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
