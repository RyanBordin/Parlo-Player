<script lang="ts">
  // The dialog a "..." menu opened, if any (see #lib/menu/dialogs.svelte.ts).
  import ConfirmDialog from "#lib/components/ConfirmDialog.svelte";
  import { describeError } from "#lib/errors.ts";
  import { deletePlaylistNow } from "#lib/menu/actions.svelte.ts";
  import { closeDialog, menuDialog } from "#lib/menu/dialogs.svelte.ts";
  import DuplicatesDialog from "#lib/menu/DuplicatesDialog.svelte";
  import InfoDialog from "#lib/menu/InfoDialog.svelte";
  import PlaylistDialog from "#lib/menu/PlaylistDialog.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { showToast } from "#lib/toasts.svelte.ts";
</script>

{#if menuDialog.current}
  {@const dialog = menuDialog.current}
  {#key dialog}
    {#if dialog.kind === "newPlaylist"}
      <PlaylistDialog target={dialog.target} onclose={closeDialog} />
    {:else if dialog.kind === "editPlaylist"}
      <PlaylistDialog playlist={dialog.playlist} onclose={closeDialog} />
    {:else if dialog.kind === "deletePlaylist"}
      <ConfirmDialog
        title={m.delete_playlist_title()}
        message={m.delete_playlist_message({ name: dialog.playlist.name })}
        confirmLabel={m.delete_playlist_confirm()}
        onconfirm={() => deletePlaylistNow(dialog.playlist).catch((e) => showToast(describeError(e).message))}
        onclose={closeDialog}
      />
    {:else if dialog.kind === "duplicates"}
      <DuplicatesDialog
        count={dialog.count}
        total={dialog.total}
        playlist={dialog.playlist}
        onchoose={dialog.resolve}
        onclose={closeDialog}
      />
    {:else if dialog.kind === "info"}
      <InfoDialog target={dialog.target} onclose={closeDialog} />
    {/if}
  {/key}
{/if}
