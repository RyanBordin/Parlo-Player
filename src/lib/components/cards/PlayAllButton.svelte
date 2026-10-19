<script lang="ts">
  // "Play all" of a song section (spec 6.1): a pill next to the row's arrows.
  import Play from "@lucide/svelte/icons/play";
  import { m } from "#lib/paraglide/messages.js";

  let { onplay }: { onplay: () => void } = $props();
</script>

<button type="button" class="pill" onclick={onplay}>
  <Play size={14} fill="currentColor" />
  {m.home_play_all()}
</button>
