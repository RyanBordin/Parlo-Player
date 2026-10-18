<script lang="ts">
  // Artist page (spec 6.7), opened from the Player Bar for now.
  import { page } from "$app/state";
  import UnderConstruction from "#lib/components/UnderConstruction.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";

  // The name is known when it is the current song's artist.
  let name = $derived(player.track?.artistId === page.params.id ? player.track?.artist : null);
</script>

<UnderConstruction title={name ?? m.artist_page_title()} />
