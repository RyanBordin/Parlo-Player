<script lang="ts">
  // "See more" of a Home section (spec 6.1).
  import { page } from "$app/state";
  import SectionGrid from "#lib/components/cards/SectionGrid.svelte";
</script>

<SectionGrid section={page.params.section ?? ""} />
