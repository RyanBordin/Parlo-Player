<script lang="ts">
  // Tracklist of album and playlist pages (spec 6.5), and later of Songs and
  // "See all". Virtualized; quick search and column sorting (spec 7); current
  // song indicator (spec 5.5); a click plays the list as shown (spec 5.1);
  // Ctrl / Shift + click select without playing (spec 5.3).
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronUp from "@lucide/svelte/icons/chevron-up";
  import Clock3 from "@lucide/svelte/icons/clock-3";
  import Heart from "@lucide/svelte/icons/heart";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import { SvelteSet } from "svelte/reactivity";
  import { playContext, setSongStarred, togglePause, type TrackListSong } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import ListSearch from "#lib/components/ListSearch.svelte";
  import VirtualList from "#lib/components/VirtualList.svelte";
  import { formatDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";
  import { filterEntries, nextSort, sortEntries, type Entry, type Sort, type SortKey } from "#lib/tracklist.ts";

  const SONG_HEIGHT = 56;
  const DISC_HEIGHT = 48;

  let {
    songs,
    kind,
    contextLabel,
    albumArtist = null,
    shownIds = $bindable([]),
    onerror,
  }: {
    songs: TrackListSong[];
    /** Album: #, title, duration, heart. Playlist: also cover and album. */
    kind: "album" | "playlist";
    /** "Playing from" when a song is clicked. */
    contextLabel: string;
    /** Album pages: a song by someone else (compilations) shows its artist. */
    albumArtist?: string | null;
    /** The list as shown (filtered and sorted), for Play / Shuffle. */
    shownIds?: string[];
    onerror?: (error: unknown) => void;
  } = $props();

  type Row = { kind: "disc"; disc: number } | { kind: "song"; entry: Entry; index: number };

  let query = $state("");
  let sort = $state<Sort>(null);
  /** Selected lines, by position in the page's own order. */
  const selected = new SvelteSet<number>();
  /** Where the last plain or Ctrl click was, for Shift + click ranges. */
  let anchor: number | null = null;

  let entries = $derived(songs.map((song, position) => ({ song, position })));
  let shown = $derived(sortEntries(filterEntries(entries, query), sort));
  let manyDiscs = $derived(kind === "album" && new Set(songs.map((s) => s.discNumber ?? 1)).size > 1);

  // "Disc N" separators: only in the album's own order (spec 6.5).
  let rows = $derived.by(() => {
    const list: Row[] = [];
    let disc: number | null = null;
    shown.forEach((entry, index) => {
      const songDisc = entry.song.discNumber ?? 1;
      if (manyDiscs && !sort && songDisc !== disc) {
        disc = songDisc;
        list.push({ kind: "disc", disc });
      }
      list.push({ kind: "song", entry, index });
    });
    return list;
  });

  $effect(() => {
    shownIds = shown.map((e) => e.song.id);
  });

  function isCurrent(song: TrackListSong) {
    return player.track?.id === song.id;
  }

  /** The Player Bar may have changed the current song's heart. */
  function isStarred(song: TrackListSong) {
    return isCurrent(song) && player.track ? player.track.starred : song.starred;
  }

  function number(entry: Entry) {
    return kind === "album" ? (entry.song.track ?? entry.position + 1) : entry.position + 1;
  }

  function onRowClick(event: MouseEvent | KeyboardEvent, entry: Entry, index: number) {
    if (event.ctrlKey || event.metaKey) {
      if (selected.has(entry.position)) selected.delete(entry.position);
      else selected.add(entry.position);
      anchor = index;
      return;
    }
    if (event.shiftKey) {
      const from = Math.min(anchor ?? index, index);
      const to = Math.max(anchor ?? index, index);
      selected.clear();
      for (let i = from; i <= to; i++) selected.add(shown[i].position);
      anchor ??= index;
      return;
    }
    selected.clear();
    anchor = index;
    // The song already playing pauses / resumes instead of starting over.
    if (isCurrent(entry.song)) {
      togglePause().catch((e) => onerror?.(e));
      return;
    }
    playContext(
      shown.map((e) => e.song.id),
      index,
      contextLabel,
    ).catch((e) => onerror?.(e));
  }

  function onRowKeydown(event: KeyboardEvent, entry: Entry, index: number) {
    if (event.key === "Enter") {
      event.preventDefault();
      onRowClick(event, entry, index);
    }
  }

  // The heart changes at once and goes back if the server refuses.
  async function toggleStar(event: MouseEvent, song: TrackListSong) {
    event.stopPropagation();
    const starred = !isStarred(song);
    const current = isCurrent(song) ? player.track : null;
    song.starred = starred;
    if (current) current.starred = starred;
    try {
      await setSongStarred(song.id, starred);
    } catch (e) {
      song.starred = !starred;
      if (current) current.starred = !starred;
      onerror?.(e);
    }
  }

  function sortBy(key: SortKey | null) {
    sort = key ? nextSort(sort, key) : null;
  }

  function ariaSort(key: SortKey | null): "ascending" | "descending" | "none" {
    if (key === null) return sort === null ? "ascending" : "none";
    return sort?.key === key ? (sort.direction === "asc" ? "ascending" : "descending") : "none";
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && selected.size > 0) selected.clear();
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#snippet sortIcon(key: SortKey)}
  {#if sort?.key === key}
    {#if sort.direction === "asc"}<ChevronUp size={14} />{:else}<ChevronDown size={14} />{/if}
  {/if}
{/snippet}

<div class="tracklist {kind}">
  <div class="tools">
    <ListSearch bind:value={query} placeholder={m.tracklist_search()} />
    {#if query}
      <span class="count">{m.tracklist_count({ shown: shown.length, total: songs.length })}</span>
    {/if}
  </div>

  {#if songs.length === 0}
    <p class="empty">{kind === "album" ? m.tracklist_empty_album() : m.tracklist_empty_playlist()}</p>
  {:else}
    <div class="table" role="table" aria-rowcount={shown.length}>
      <div class="head grid" role="row">
        <div role="columnheader" aria-sort={ariaSort(null)}>
          <button type="button" class="sort number" onclick={() => sortBy(null)} title={m.tracklist_default_order()}>#</button>
        </div>
        <div role="columnheader" aria-sort={ariaSort("title")}>
          <button type="button" class="sort" onclick={() => sortBy("title")} title={m.tracklist_sort_by({ column: m.tracklist_title() })}>
            {m.tracklist_title()}{@render sortIcon("title")}
          </button>
        </div>
        {#if kind === "playlist"}
          <div role="columnheader" aria-sort={ariaSort("album")}>
            <button type="button" class="sort" onclick={() => sortBy("album")} title={m.tracklist_sort_by({ column: m.tracklist_album() })}>
              {m.tracklist_album()}{@render sortIcon("album")}
            </button>
          </div>
        {/if}
        <div role="columnheader" aria-sort={ariaSort("duration")}>
          <button
            type="button"
            class="sort end"
            onclick={() => sortBy("duration")}
            aria-label={m.tracklist_sort_by({ column: m.tracklist_duration() })}
            title={m.tracklist_sort_by({ column: m.tracklist_duration() })}
          >
            {@render sortIcon("duration")}<Clock3 size={16} />
          </button>
        </div>
        <div role="columnheader"><span class="visually-hidden">{m.player_favorite()}</span></div>
      </div>

      {#if shown.length === 0}
        <p class="empty">{m.tracklist_no_results({ query })}</p>
      {:else}
        <VirtualList
          items={rows}
          height={(row) => (row.kind === "disc" ? DISC_HEIGHT : SONG_HEIGHT)}
          key={(row) => (row.kind === "disc" ? `disc-${row.disc}` : `song-${row.entry.position}`)}
        >
          {#snippet row(row: Row)}
            {#if row.kind === "disc"}
              <div class="disc" role="row">{m.tracklist_disc({ number: row.disc })}</div>
            {:else}
              {@const { entry, index } = row}
              {@const song = entry.song}
              {@const current = isCurrent(song)}
              {@const starred = isStarred(song)}
              {@const artistShown = kind === "playlist" || (song.artist && song.artist !== albumArtist)}
              <div
                class="song grid"
                class:current
                class:selected={selected.has(entry.position)}
                role="row"
                tabindex="0"
                aria-rowindex={index + 1}
                aria-selected={selected.has(entry.position)}
                onclick={(event) => onRowClick(event, entry, index)}
                onkeydown={(event) => onRowKeydown(event, entry, index)}
              >
                <div class="number" role="cell">
                  {#if current}
                    {#if player.status === "playing"}
                      <span role="img" aria-label={m.tracklist_playing()}><Play size={16} fill="currentColor" /></span>
                    {:else}
                      <span role="img" aria-label={m.tracklist_paused()}><Pause size={16} fill="currentColor" /></span>
                    {/if}
                  {:else}
                    <span class="digits">{number(entry)}</span>
                    <span class="hover-play" aria-hidden="true"><Play size={16} fill="currentColor" /></span>
                  {/if}
                </div>
                <div class="title-cell" role="cell">
                  {#if kind === "playlist"}
                    <span class="cover"><Cover id={song.coverArt} size={40} radius="var(--radius-sm)" /></span>
                  {/if}
                  <span class="text">
                    <span class="title" title={song.title}>{song.title}</span>
                    {#if artistShown && song.artist}
                      <span class="artist" title={song.artist}>{song.artist}</span>
                    {/if}
                  </span>
                </div>
                {#if kind === "playlist"}
                  <div class="album" role="cell">
                    {#if song.album && song.albumId}
                      <a
                        href={`/albums/${encodeURIComponent(song.albumId)}`}
                        title={song.album}
                        onclick={(event) => event.stopPropagation()}>{song.album}</a
                      >
                    {:else if song.album}
                      <span title={song.album}>{song.album}</span>
                    {/if}
                  </div>
                {/if}
                <div class="duration" role="cell">{formatDuration(song.duration)}</div>
                <div class="favorite" role="cell">
                  <button
                    type="button"
                    class:active={starred}
                    onclick={(event) => toggleStar(event, song)}
                    aria-label={starred ? m.player_unfavorite() : m.player_favorite()}
                    aria-pressed={starred}
                    title={starred ? m.player_unfavorite() : m.player_favorite()}
                  >
                    <Heart size={16} fill={starred ? "currentColor" : "none"} />
                  </button>
                </div>
              </div>
            {/if}
          {/snippet}
        </VirtualList>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tracklist {
    --columns: 48px minmax(0, 1fr) 72px 48px;
  }

  .tracklist.playlist {
    --columns: 48px minmax(0, 3fr) minmax(0, 2fr) 72px 48px;
  }

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

  .grid {
    display: grid;
    grid-template-columns: var(--columns);
    align-items: center;
    gap: var(--space-4);
    padding: 0 var(--space-3);
  }

  /* Stays under the Topbar while the list scrolls. */
  .head {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 40px;
    margin-bottom: var(--space-1);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg);
  }

  .sort {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text-muted);
    font: inherit;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    cursor: pointer;
  }

  .sort:hover {
    color: var(--color-text);
  }

  .sort.number {
    width: 100%;
    justify-content: center;
  }

  .sort.end {
    width: 100%;
    justify-content: flex-end;
  }

  .disc {
    height: 100%;
    display: flex;
    align-items: flex-end;
    padding: 0 var(--space-3) var(--space-2);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
  }

  .song {
    height: 100%;
    border-radius: var(--radius-md);
    cursor: pointer;
    user-select: none;
  }

  .song:hover {
    background: var(--color-surface);
  }

  .song.selected {
    background: var(--color-surface-active);
  }

  .number {
    display: grid;
    place-items: center;
    color: var(--color-text-muted);
    font-size: 14px;
    font-variant-numeric: tabular-nums;
  }

  .hover-play {
    display: none;
    color: var(--color-text);
  }

  /* On hover the number becomes a Play icon (spec 5.5). */
  .song:hover .digits {
    display: none;
  }

  .song:hover .hover-play {
    display: grid;
  }

  /* Current song: accent color (spec 5.5). */
  .song.current .number,
  .song.current .title {
    color: var(--color-accent-text);
  }

  .title-cell {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .cover {
    flex: none;
    width: 40px;
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title,
  .artist,
  .album > * {
    display: block;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: var(--font-weight-medium);
  }

  .artist {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .album {
    min-width: 0;
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .album a {
    color: inherit;
    text-decoration: none;
  }

  .album a:hover {
    color: var(--color-text);
    text-decoration: underline;
  }

  .duration {
    color: var(--color-text-muted);
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .favorite {
    display: grid;
    place-items: center;
  }

  .favorite button {
    display: grid;
    place-items: center;
    padding: var(--space-1);
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    opacity: 0;
    cursor: pointer;
  }

  /* The heart shows on hover, and always when the song is a favorite. */
  .song:hover .favorite button,
  .favorite button:focus-visible,
  .favorite button.active {
    opacity: 1;
  }

  .favorite button.active,
  .favorite button:hover {
    color: var(--color-text);
  }

  .empty {
    padding: var(--space-5) var(--space-3);
    color: var(--color-text-muted);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
