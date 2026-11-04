<script lang="ts">
  // Settings opens on the sector seen last (spec 6.10). On the phone (spec
  // 4.5) it opens on the list of sectors.
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { lastSector } from "#lib/settings/registry.ts";
  import SettingsPage from "#lib/settings/SettingsPage.svelte";
  import { layout } from "#lib/shell/phone.svelte.ts";

  onMount(() => {
    if (!layout.phone) goto(`/settings/${lastSector()}`, { replaceState: true });
  });
</script>

{#if layout.phone}
  <SettingsPage />
{/if}
