<script lang="ts">
  // Playlist page (spec 6.5), opened from the sidebar.
  import { page } from "$app/state";
  import UnderConstruction from "#lib/components/UnderConstruction.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { playlists } from "#lib/playlists.svelte.ts";

  let name = $derived(playlists.items.find((p) => p.id === page.params.id)?.name);
</script>

<UnderConstruction title={name ?? m.playlist_page_title()} />
