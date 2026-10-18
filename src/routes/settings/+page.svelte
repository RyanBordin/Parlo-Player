<script lang="ts">
  // Settings (spec 6.10) come later. For now this page holds the temporary
  // library status and queue list, which have no other place yet.
  import LibraryStatus from "#lib/components/LibraryStatus.svelte";
  import QueueList from "#lib/components/QueueList.svelte";
  import UnderConstruction from "#lib/components/UnderConstruction.svelte";
  import { m } from "#lib/paraglide/messages.js";
</script>

<UnderConstruction title={m.nav_settings()} />

<div class="page temporary">
  <LibraryStatus />
  <QueueList />
</div>

<style>
  .temporary {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    padding-top: 0;
  }
</style>
