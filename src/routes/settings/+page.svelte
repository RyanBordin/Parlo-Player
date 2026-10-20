<script lang="ts">
  // Settings opens on the sector seen last (spec 6.10).
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { lastSector } from "#lib/settings/registry.ts";

  onMount(() => {
    goto(`/settings/${lastSector()}`, { replaceState: true });
  });
</script>
