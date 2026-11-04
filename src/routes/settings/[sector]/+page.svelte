<script lang="ts">
  // Settings (spec 6.10), one sector.
  import { page } from "$app/state";
  import SettingsPage from "#lib/settings/SettingsPage.svelte";
</script>

<SettingsPage sectorId={page.params.sector} />
