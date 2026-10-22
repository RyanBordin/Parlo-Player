<script lang="ts">
  // Tracklist of album and playlist pages (spec 6.5) and of the favorite
  // songs (spec 6.2): the page has all its songs, so the quick search and the
  // column sorting (spec 7) run here, in memory. A click plays the list as
  // shown (spec 5.1). "..." and a right click open the menus (spec 8.1).
  import { playContext, type TrackListSong } from "#lib/api.ts";
  import ListSearch from "#lib/components/ListSearch.svelte";
  import TrackTable from "#lib/components/TrackTable.svelte";
  import { isOwnPlaylist, openSongsMenu, playlistEntries, type SongPlace } from "#lib/menu/menus.ts";
  import { m } from "#lib/paraglide/messages.js";
  import type { ColumnId, TableRow } from "#lib/track-columns.ts";
  import { filterEntries, nextSort, sortEntries, type Sort, type SortKey } from "#lib/tracklist.ts";

  let {
    songs,
    kind,
    contextLabel,
    albumArtist = null,
    playlist = null,
    shownIds = $bindable([]),
    onerror,
  }: {
    songs: TrackListSong[];
    /** Album: #, title, duration, heart. Playlist: also cover and album. Favorites and artist: also artist. */
    kind: "album" | "playlist" | "favorites" | "artist";
    /** "Playing from" when a song is clicked. */
    contextLabel: string;
    /** Album pages: a song by someone else (compilations) shows its artist. */
    albumArtist?: string | null;
    /** Playlist pages: the playlist, for "Remove from this playlist" when it is the user's. */
    playlist?: { id: string; name: string; owner: string | null } | null;
    /** The list as shown (filtered and sorted), for Play / Shuffle. */
    shownIds?: string[];
    onerror?: (error: unknown) => void;
  } = $props();

  const COLUMNS: Record<typeof kind, ColumnId[]> = {
    album: ["number", "title", "duration", "favorite"],
    playlist: ["number", "title", "album", "duration", "favorite"],
    favorites: ["number", "title", "artist", "album", "duration", "favorite"],
    artist: ["number", "title", "artist", "album", "duration", "favorite"],
  };
  const SORT_KEYS: Partial<Record<ColumnId, SortKey | null>> = {
    number: null,
    title: "title",
    artist: "artist",
    album: "album",
    duration: "duration",
  };

  let query = $state("");
  let sort = $state<Sort>(null);

  let entries = $derived(songs.map((song, position) => ({ song, position })));
  let shown = $derived(sortEntries(filterEntries(entries, query), sort));
  let manyDiscs = $derived(kind === "album" && new Set(songs.map((s) => s.discNumber ?? 1)).size > 1);

  // "Disc N" separators: only in the album's own order (spec 6.5).
  let rows = $derived.by(() => {
    const list: TableRow[] = [];
    let disc: number | null = null;
    shown.forEach((entry, index) => {
      const songDisc = entry.song.discNumber ?? 1;
      if (manyDiscs && !sort && songDisc !== disc) {
        disc = songDisc;
        list.push({ kind: "disc", disc });
      }
      list.push({ kind: "song", index });
    });
    return list;
  });

  $effect(() => {
    shownIds = shown.map((e) => e.song.id);
  });

  function numberOf(index: number, song: TrackListSong) {
    const position = shown[index].position;
    return kind === "album" ? (song.track ?? position + 1) : position + 1;
  }

  function artistBelow(song: TrackListSong) {
    if (kind === "playlist") return true;
    if (kind === "favorites" || kind === "artist") return false; // the artist has its own column
    return song.artist !== albumArtist;
  }

  const EMPTY: Record<typeof kind, () => string> = {
    album: m.tracklist_empty_album,
    playlist: m.tracklist_empty_playlist,
    favorites: m.home_starred_songs_empty,
    artist: m.artist_songs_empty,
  };

  function onsort(key: string | null) {
    sort = key ? nextSort(sort, key as SortKey) : null;
  }

  function onmenu(event: MouseEvent, indexes: number[]) {
    const picked = indexes.map((index) => shown[index]);
    const place: SongPlace = { label: contextLabel };
    if (playlist && isOwnPlaylist(playlist)) {
      const entries = playlistEntries(
        songs,
        picked.map((entry) => entry.position),
      );
      place.playlist = { id: playlist.id, name: playlist.name, entries };
    }
    openSongsMenu(
      event,
      picked.map((entry) => entry.song),
      place,
    );
  }

  function onplay(index: number) {
    playContext(
      shown.map((e) => e.song.id),
      index,
      contextLabel,
    ).catch((e) => onerror?.(e));
  }
</script>

<div class="tracklist">
  <div class="tools">
    <ListSearch bind:value={query} placeholder={m.tracklist_search()} />
    {#if query}
      <span class="count">{m.tracklist_count({ shown: shown.length, total: songs.length })}</span>
    {/if}
  </div>

  {#if songs.length === 0}
    <p class="empty">{EMPTY[kind]()}</p>
  {:else if shown.length === 0}
    <p class="empty">{m.tracklist_no_results({ query })}</p>
  {:else}
    <TrackTable
      {rows}
      songAt={(index) => shown[index]?.song}
      columns={COLUMNS[kind]}
      sortKeys={SORT_KEYS}
      {sort}
      {onsort}
      numberTitle={m.tracklist_default_order()}
      {numberOf}
      showCover={kind !== "album"}
      {artistBelow}
      selectKey={(index) => shown[index].position}
      {onplay}
      {onmenu}
      resetOn={songs}
      {onerror}
    />
  {/if}
</div>

<style>
  .tools {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
  }

  .count {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .empty {
    padding: var(--space-5) var(--space-3);
    color: var(--color-text-muted);
  }
</style>
