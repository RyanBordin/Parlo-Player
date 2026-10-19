<script lang="ts">
  // Songs (spec 6.3): every song of the library in one virtualized table.
  // Search and sorting run in SQLite (spec 7); the lines load as they come
  // into view. A click plays the list as shown ("Playing from: Songs").
  import Dices from "@lucide/svelte/icons/dices";
  import Play from "@lucide/svelte/icons/play";
  import { playContext, playMix, type SongSortKey } from "#lib/api.ts";
  import ColumnPicker from "#lib/components/ColumnPicker.svelte";
  import ListSearch from "#lib/components/ListSearch.svelte";
  import SortSelect from "#lib/components/SortSelect.svelte";
  import TrackTable from "#lib/components/TrackTable.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatNumber, formatSongCount } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import {
    drawSongOrder,
    presetOf,
    refreshSongs,
    reloadSongs,
    setSongColumns,
    setSongPreset,
    setSongQuery,
    SONG_PRESETS,
    type SongPreset,
    songAt,
    songList,
    songsView,
    sortSongsBy,
  } from "#lib/songs.svelte.ts";
  import { syncState } from "#lib/sync.svelte.ts";
  import type { ColumnId, TableRow } from "#lib/track-columns.ts";

  /** The search runs once typing pauses this long. */
  const SEARCH_DELAY_MS = 150;

  const SORT_KEYS: Partial<Record<ColumnId, SongSortKey>> = {
    number: "added",
    title: "title",
    artist: "artist",
    album: "album",
    year: "year",
    genre: "genre",
    bitRate: "bitRate",
    playCount: "playCount",
    duration: "duration",
  };

  /** When the page began to open, for the timings. */
  const openedAt = performance.now();
  let error = $state<ErrorMessage | null>(null);

  const PRESET_LABELS: Record<SongPreset, () => string> = {
    title: m.songs_sort_title,
    artist: m.albums_sort_artist,
    year: m.albums_sort_year,
    added: m.songs_sort_added,
    mostPlayed: m.songs_sort_most_played,
    recentlyPlayed: m.songs_sort_recently_played,
    random: m.albums_sort_random,
  };
  let preset = $derived(presetOf(songsView.sort));

  let ids = $derived(songList.ids ?? []);
  let rows = $derived(ids.map((_, index): TableRow => ({ kind: "song", index })));
  let columns = $derived<ColumnId[]>(["number", "title", "artist", "album", ...songsView.columns, "duration", "favorite"]);
  let count = $derived.by(() => {
    if (songsView.query) return m.songs_filtered({ shown: formatNumber(ids.length), total: formatNumber(songsView.total) });
    return formatSongCount(songsView.total);
  });

  // The search field and the saved search stay in step; the search runs
  // when typing pauses.
  let query = $state(songsView.query);
  $effect(() => {
    const typed = query;
    if (typed === songsView.query) return;
    const timer = setTimeout(() => {
      setSongQuery(typed);
      refreshSongs("search");
    }, SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  // On open (the last list shows meanwhile), and after every sync.
  let opened = false;
  $effect(() => {
    void syncState.finishedCount;
    if (opened) reloadSongs();
    else refreshSongs("open", openedAt);
    opened = true;
  });

  function showError(e: unknown) {
    error = describeError(e);
  }

  function play(index: number) {
    error = null;
    playContext(ids, index, m.nav_songs()).catch(showError);
  }

  function shuffle() {
    error = null;
    playMix(ids, m.nav_songs()).catch(showError);
  }
</script>

<div class="page">
  <header class="header">
    <div>
      <h1 class="page-title">{m.nav_songs()}</h1>
      {#if songList.ids}
        <p class="page-subtitle">{count}</p>
      {/if}
    </div>
    <div class="buttons">
      <button type="button" class="button button-primary" onclick={() => play(0)} disabled={ids.length === 0}>
        <Play size={16} fill="currentColor" />
        {m.albums_play_all()}
      </button>
      <button type="button" class="button button-secondary" onclick={shuffle} disabled={ids.length === 0}>
        <Dices size={16} />
        {m.songs_shuffle()}
      </button>
    </div>
  </header>

  <div class="tools">
    <ListSearch bind:value={query} placeholder={m.songs_search()} />
    <SortSelect
      value={preset}
      options={SONG_PRESETS.map((p) => ({ value: p, label: PRESET_LABELS[p]() }))}
      onchange={setSongPreset}
      otherLabel={m.songs_sort_by_column()}
      onredraw={preset === "random" ? drawSongOrder : undefined}
    />
    <ColumnPicker columns={songsView.columns} onchange={setSongColumns} />
  </div>

  {#if error ?? songsView.error}
    <p class="error">{(error ?? songsView.error)!.message}</p>
  {/if}

  {#if songList.ids && songsView.total === 0}
    <p class="empty">{m.songs_empty()}</p>
  {:else if songList.ids && ids.length === 0}
    <p class="empty">{m.tracklist_no_results({ query: songsView.query })}</p>
  {:else if songList.ids}
    <TrackTable
      {rows}
      {songAt}
      {columns}
      sortKeys={SORT_KEYS}
      sort={songsView.sort}
      onsort={(key) => sortSongsBy((key ?? "added") as SongSortKey)}
      numberTitle={m.songs_sort_added()}
      numberOf={(index) => index + 1}
      showCover
      subtitleOf={() => null}
      selectKey={(index) => ids[index]}
      onplay={play}
      onerror={showError}
    />
  {/if}
</div>

<style>
  .header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .buttons {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .buttons .button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    font-size: 14px;
  }

  .tools {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin: var(--space-5) 0;
  }

  .empty {
    color: var(--color-text-muted);
  }

  .error {
    margin-bottom: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
