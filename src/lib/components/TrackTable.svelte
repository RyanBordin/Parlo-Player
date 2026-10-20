<script lang="ts">
  // The table of a tracklist (album, playlist and Songs pages): virtualized,
  // with configurable columns, sortable headers (spec 7), the current song
  // indicator (spec 5.5) and the heart. A click plays (spec 5.1); Ctrl /
  // Shift + click select without playing (spec 5.3). Where the songs come
  // from, and how they are searched and sorted, is up to the page: a line
  // whose song is not loaded yet shows a placeholder.
  import ChevronDown from "@lucide/svelte/icons/chevron-down";
  import ChevronUp from "@lucide/svelte/icons/chevron-up";
  import Clock3 from "@lucide/svelte/icons/clock-3";
  import Heart from "@lucide/svelte/icons/heart";
  import Pause from "@lucide/svelte/icons/pause";
  import Play from "@lucide/svelte/icons/play";
  import { SvelteSet } from "svelte/reactivity";
  import { setSongStarred, togglePause, type TrackListSong } from "#lib/api.ts";
  import Cover from "#lib/components/Cover.svelte";
  import VirtualList from "#lib/components/VirtualList.svelte";
  import { formatDuration, formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { player } from "#lib/player.svelte.ts";
  import { COLUMNS, gridColumns, type ColumnId, type TableRow, type TableSort } from "#lib/track-columns.ts";

  const SONG_HEIGHT = 56;
  const DISC_HEIGHT = 48;

  let {
    rows,
    songAt,
    columns,
    sortKeys,
    sort,
    onsort,
    numberTitle,
    numberOf,
    showCover,
    artistBelow,
    selectKey,
    onplay,
    onerror,
  }: {
    rows: TableRow[];
    /** The song at a position of the list shown; `undefined` while it loads. */
    songAt: (index: number) => TrackListSong | undefined;
    columns: ColumnId[];
    /** The key each sortable column sorts by; `null` is the page's own order. */
    sortKeys: Partial<Record<ColumnId, string | null>>;
    sort: TableSort;
    onsort: (key: string | null) => void;
    /** Tooltip of the "#" header. */
    numberTitle: string;
    /** The number shown in the "#" column. */
    numberOf: (index: number, song: TrackListSong) => number;
    /** A small cover before the title. */
    showCover: boolean;
    /** The song's artist under the title (playlists, compilations). */
    artistBelow: (song: TrackListSong) => boolean;
    /** What a line is selected by; kept while the list is sorted again. */
    selectKey: (index: number) => string | number;
    /** A click on a song that is not the current one. */
    onplay: (index: number) => void;
    onerror?: (error: unknown) => void;
  } = $props();

  const selected = new SvelteSet<string | number>();
  /** Where the last plain or Ctrl click was, for Shift + click ranges. */
  let anchor: number | null = null;

  let songCount = $derived(rows.reduce((count, row) => count + (row.kind === "song" ? 1 : 0), 0));
  let template = $derived(gridColumns(columns));

  function isCurrent(song: TrackListSong) {
    return player.track?.id === song.id;
  }

  /** The Player Bar may have changed the current song's heart. */
  function isStarred(song: TrackListSong) {
    return isCurrent(song) && player.track ? player.track.starred : song.starred;
  }

  function onRowClick(event: MouseEvent | KeyboardEvent, song: TrackListSong, index: number) {
    const key = selectKey(index);
    if (event.ctrlKey || event.metaKey) {
      if (selected.has(key)) selected.delete(key);
      else selected.add(key);
      anchor = index;
      return;
    }
    if (event.shiftKey) {
      const from = Math.min(anchor ?? index, index);
      const to = Math.max(anchor ?? index, index);
      selected.clear();
      for (let i = from; i <= to; i++) selected.add(selectKey(i));
      anchor ??= index;
      return;
    }
    selected.clear();
    anchor = index;
    // The song already playing pauses / resumes instead of starting over.
    if (isCurrent(song)) {
      togglePause().catch((e) => onerror?.(e));
      return;
    }
    onplay(index);
  }

  function onRowKeydown(event: KeyboardEvent, song: TrackListSong, index: number) {
    if (event.key === "Enter") {
      event.preventDefault();
      onRowClick(event, song, index);
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

  function sortable(column: ColumnId): boolean {
    return column in sortKeys;
  }

  function isSorted(column: ColumnId): boolean {
    const key = sortKeys[column];
    return key === null ? sort === null : sort?.key === key;
  }

  function ariaSort(column: ColumnId): "ascending" | "descending" | "none" {
    if (!isSorted(column)) return "none";
    return sort?.direction === "desc" ? "descending" : "ascending";
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && selected.size > 0) selected.clear();
  }

  function genreOf(song: TrackListSong) {
    return song.genres.length ? song.genres.join(", ") : (song.genre ?? "");
  }

  const stop = (event: Event) => event.stopPropagation();
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#snippet sortIcon(column: ColumnId)}
  {#if isSorted(column) && sort}
    {#if sort.direction === "asc"}<ChevronUp size={14} />{:else}<ChevronDown size={14} />{/if}
  {/if}
{/snippet}

{#snippet header(column: ColumnId)}
  {@const info = COLUMNS[column]}
  {#if column === "favorite"}
    <div role="columnheader"><span class="visually-hidden">{m.player_favorite()}</span></div>
  {:else if column === "number"}
    <div role="columnheader" aria-sort={ariaSort(column)}>
      {#if sortable(column)}
        <button type="button" class="sort center" onclick={() => onsort(sortKeys.number ?? null)} title={numberTitle}>
          #{#if sort !== null}{@render sortIcon(column)}{/if}
        </button>
      {:else}
        <span class="sort center">#</span>
      {/if}
    </div>
  {:else}
    {@const label = column === "duration" ? m.tracklist_duration() : (info.label?.() ?? "")}
    <div role="columnheader" aria-sort={ariaSort(column)}>
      <button
        type="button"
        class="sort"
        class:end={info.numeric}
        disabled={!sortable(column)}
        onclick={() => onsort(sortKeys[column] ?? null)}
        aria-label={m.tracklist_sort_by({ column: label })}
        title={m.tracklist_sort_by({ column: label })}
      >
        {#if info.numeric}{@render sortIcon(column)}{/if}
        {#if column === "duration"}<Clock3 size={16} />{:else}{label}{/if}
        {#if !info.numeric}{@render sortIcon(column)}{/if}
      </button>
    </div>
  {/if}
{/snippet}

{#snippet cell(column: ColumnId, song: TrackListSong, index: number)}
  {#if column === "number"}
    <div class="number" role="cell">
      {#if isCurrent(song)}
        {#if player.status === "playing"}
          <span role="img" aria-label={m.tracklist_playing()}><Play size={16} fill="currentColor" /></span>
        {:else}
          <span role="img" aria-label={m.tracklist_paused()}><Pause size={16} fill="currentColor" /></span>
        {/if}
      {:else}
        <span class="digits">{numberOf(index, song)}</span>
        <span class="hover-play" aria-hidden="true"><Play size={16} fill="currentColor" /></span>
      {/if}
    </div>
  {:else if column === "title"}
    <div class="title-cell" role="cell">
      {#if showCover}
        <span class="cover"><Cover id={song.coverArt} size={40} radius="var(--radius-sm)" /></span>
      {/if}
      <span class="text">
        <span class="title" title={song.title}>{song.title}</span>
        {#if song.artist && artistBelow(song)}
          <span class="subtitle" title={song.artist}>
            {#if song.artistId}
              <a href={`/artists/${encodeURIComponent(song.artistId)}`} onclick={stop}>{song.artist}</a>
            {:else}
              {song.artist}
            {/if}
          </span>
        {/if}
      </span>
    </div>
  {:else if column === "artist"}
    <div class="muted" role="cell">
      {#if song.artist && song.artistId}
        <a href={`/artists/${encodeURIComponent(song.artistId)}`} title={song.artist} onclick={stop}>{song.artist}</a>
      {:else if song.artist}
        <span title={song.artist}>{song.artist}</span>
      {/if}
    </div>
  {:else if column === "album"}
    <div class="muted" role="cell">
      {#if song.album && song.albumId}
        <a href={`/albums/${encodeURIComponent(song.albumId)}`} title={song.album} onclick={stop}>{song.album}</a>
      {:else if song.album}
        <span title={song.album}>{song.album}</span>
      {/if}
    </div>
  {:else if column === "year"}
    <div class="muted numeric" role="cell">{song.year ?? ""}</div>
  {:else if column === "genre"}
    {@const genre = genreOf(song)}
    <div class="muted" role="cell"><span title={genre}>{genre}</span></div>
  {:else if column === "bitRate"}
    <div class="muted numeric" role="cell">{song.bitRate ? m.tracklist_kbps({ value: song.bitRate }) : ""}</div>
  {:else if column === "playCount"}
    <div class="muted numeric" role="cell">{song.playCount ? formatNumber(song.playCount) : ""}</div>
  {:else if column === "duration"}
    <div class="muted numeric" role="cell">{formatDuration(song.duration)}</div>
  {:else if column === "favorite"}
    {@const starred = isStarred(song)}
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
  {/if}
{/snippet}

<div class="table" role="table" aria-rowcount={songCount} style:--columns={template}>
  <div class="head grid" role="row">
    {#each columns as column (column)}
      {@render header(column)}
    {/each}
  </div>

  <VirtualList
    items={rows}
    height={(row) => (row.kind === "disc" ? DISC_HEIGHT : SONG_HEIGHT)}
    key={(row) => (row.kind === "disc" ? `disc-${row.disc}` : `song-${selectKey(row.index)}`)}
  >
    {#snippet row(row: TableRow)}
      {#if row.kind === "disc"}
        <div class="disc" role="row">{m.tracklist_disc({ number: row.disc })}</div>
      {:else}
        {@const index = row.index}
        {@const song = songAt(index)}
        {#if song}
          {@const key = selectKey(index)}
          <div
            class="song grid"
            class:current={isCurrent(song)}
            class:selected={selected.has(key)}
            role="row"
            tabindex="0"
            aria-rowindex={index + 1}
            aria-selected={selected.has(key)}
            onclick={(event) => onRowClick(event, song, index)}
            onkeydown={(event) => onRowKeydown(event, song, index)}
          >
            {#each columns as column (column)}
              {@render cell(column, song, index)}
            {/each}
          </div>
        {:else}
          <div class="song grid loading" role="row" aria-rowindex={index + 1} aria-busy="true">
            <div class="number" role="cell"></div>
            <div class="title-cell" role="cell">
              {#if showCover}<span class="cover placeholder"></span>{/if}
              <span class="bar"></span>
            </div>
          </div>
        {/if}
      {/if}
    {/snippet}
  </VirtualList>
</div>

<style>
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
    min-width: 0;
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
    white-space: nowrap;
    cursor: pointer;
  }

  .sort:hover:not(:disabled) {
    color: var(--color-text);
  }

  .sort:disabled {
    cursor: default;
  }

  .sort.center {
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

  .song.loading {
    cursor: default;
  }

  .song.loading:hover {
    background: transparent;
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

  .cover.placeholder {
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .bar {
    width: 40%;
    height: 12px;
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }

  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title,
  .subtitle,
  .muted > * {
    display: block;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .title {
    font-size: 14px;
    font-weight: var(--font-weight-medium);
  }

  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  .muted {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-muted);
    font-size: 14px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .muted a,
  .subtitle a {
    color: inherit;
    text-decoration: none;
  }

  .muted a:hover,
  .subtitle a:hover {
    color: var(--color-text);
    text-decoration: underline;
  }

  .numeric {
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

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
