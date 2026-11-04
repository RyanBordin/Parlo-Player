<script lang="ts">
  // Playlists section of the Library on the phone (spec 4.5): the list of the
  // sidebar's lower zone (spec 4.2), with "+ New playlist" at the top.
  import PlaylistList from "#lib/shell/PlaylistList.svelte";
</script>

<div class="page">
  <PlaylistList />
</div>

<style>
  .page {
    padding: 0 var(--page-gutter) var(--space-6);
  }
</style>
