<script lang="ts">
  // "See more" of a row of albums or artists (Home, spec 6.1; Favorites,
  // spec 6.2): the row's title and every item in a grid. Cards are drawn in
  // batches as the end comes into view.
  import type { HomeSection } from "#lib/api.ts";
  import HomeItemCard from "#lib/components/cards/HomeItemCard.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { GRID_BATCH, refreshSection, sections } from "#lib/home.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { syncState } from "#lib/sync.svelte.ts";

  /** A `HomeSection`; anything else shows an empty page. */
  let { section }: { section: string } = $props();

  /** Cover width asked for: a grid column is about this wide. */
  const CARD = 200;

  const TITLES: Record<HomeSection, () => string> = {
    "most-played-albums": m.home_most_played_albums,
    "favorite-albums": m.home_starred_albums,
    "favorite-artists": m.home_starred_artists,
  };
  const EMPTY: Record<HomeSection, () => string> = {
    "most-played-albums": m.home_most_played_albums_empty,
    "favorite-albums": m.home_starred_albums_empty,
    "favorite-artists": m.home_starred_artists_empty,
  };

  let known = $derived(section in TITLES);
  let key = $derived(section as HomeSection);
  let data = $derived(known ? sections[key] : undefined);
  let shownItems = $derived(data?.items.slice(0, data.shown) ?? []);
  let error = $state<ErrorMessage | null>(null);
  let sentinel = $state<HTMLElement>();

  // On open and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    if (!known) return;
    error = null;
    refreshSection(key).catch((e) => (error = describeError(e)));
  });

  // Continuous scrolling: more cards when the end of the grid shows. Watched
  // again after each batch: on a tall window the end may still be in view.
  $effect(() => {
    void data?.shown;
    if (!sentinel) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const current = sections[key];
        if (entries.some((e) => e.isIntersecting) && current && current.shown < current.items.length) {
          current.shown += GRID_BATCH;
        }
      },
      { rootMargin: "400px" },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  });
</script>

<div class="page">
  <h1 class="page-title">{known ? TITLES[key]() : m.nav_home()}</h1>
  {#if data?.offline}
    <p class="page-subtitle">{m.home_offline()}</p>
  {/if}
  {#if error}
    <p class="error">{error.message}</p>
  {/if}

  {#if data && data.items.length === 0}
    <p class="empty">{EMPTY[key]()}</p>
  {:else if data}
    <div class="grid">
      {#each shownItems as item (`${item.kind}-${item.id}`)}
        <HomeItemCard {item} size={CARD} fluid onerror={(e) => (error = describeError(e))} />
      {/each}
    </div>
    <div class="sentinel" bind:this={sentinel} aria-hidden="true"></div>
  {/if}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--space-6) var(--space-5);
    margin-top: var(--space-6);
  }

  .sentinel {
    height: 1px;
  }

  .empty {
    margin-top: var(--space-6);
    color: var(--color-text-muted);
  }

  .error {
    margin-top: var(--space-3);
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }
</style>
