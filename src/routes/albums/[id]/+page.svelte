<script lang="ts">
  // Album page (spec 6.5), opened from the Player Bar for now.
  import { page } from "$app/state";
  import UnderConstruction from "#lib/components/UnderConstruction.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  // The name is known when it is the current song's album.
  let name = $derived(player.track?.albumId === page.params.id ? player.track?.album : null);
</script>

<UnderConstruction title={name ?? m.album_page_title()} />
