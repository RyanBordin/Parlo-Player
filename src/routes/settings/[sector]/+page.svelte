<script lang="ts">
  // Settings (spec 6.10), after Symfonium: the sectors on the left, the
  // options of the one chosen on the right, under subtitles. The search at
  // the top filters the options of every sector by name and description and
  // shows each result under its sector. Changes apply at once; "Restore
  // defaults" closes each sector.
  import type { Component } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import ListSearch from "#lib/components/ListSearch.svelte";
  import { m } from "#lib/paraglide/messages.js";
  import ActionSetting from "#lib/settings/ActionSetting.svelte";
  import AppearanceSettings from "#lib/settings/AppearanceSettings.svelte";
  import { restoreDefaults } from "#lib/settings/app-settings.svelte.ts";
  import GeneralSettings from "#lib/settings/GeneralSettings.svelte";
  import {
    findSector,
    OPTIONS,
    rememberSector,
    searchOptions,
    SECTORS,
    setSettingsFilter,
    type Sector,
    type SectorId,
  } from "#lib/settings/registry.ts";
  import AboutSettings from "#lib/settings/AboutSettings.svelte";
  import { librarySettings, restoreLibraryDefaults } from "#lib/settings/library-settings.svelte.ts";
  import LibrarySector from "#lib/settings/LibrarySettings.svelte";
  import PlaybackSettings from "#lib/settings/PlaybackSettings.svelte";
  import ServerSettings from "#lib/settings/ServerSettings.svelte";
  import StreamingSettings from "#lib/settings/StreamingSettings.svelte";
  import SyncSettings from "#lib/settings/SyncSettings.svelte";
  import { startSync } from "#lib/sync.svelte.ts";

  const CONTENT: Record<SectorId, Component> = {
    general: GeneralSettings,
    appearance: AppearanceSettings,
    playback: PlaybackSettings,
    streaming: StreamingSettings,
    server: ServerSettings,
    sync: SyncSettings,
    library: LibrarySector,
    about: AboutSettings,
  };

  let sector = $derived(findSector(page.params.sector));
  let query = $state("");
  /** `null`: no search, every option shows. */
  let found = $derived(searchOptions(query));
  let foundSectors = $derived(
    found ? SECTORS.filter((s) => OPTIONS.some((option) => option.sector === s.id && found.has(option.id))) : [],
  );

  setSettingsFilter({ shows: (id) => found === null || found.has(id) });

  // An unknown sector opens the first one; a known one is remembered.
  $effect(() => {
    if (sector) rememberSector(sector.id);
    else goto(`/settings/${SECTORS[0].id}`, { replaceState: true });
  });

  // Choosing a sector leaves the search.
  $effect(() => {
    void page.params.sector;
    query = "";
  });

  function hasDefaults(s: Sector): boolean {
    return s.defaults.length + (s.libraryDefaults?.length ?? 0) > 0;
  }

  async function restore(s: Sector) {
    // Folders shown again come back only with a full sync.
    const foldersBack = (s.libraryDefaults ?? []).includes("hiddenFolders") && librarySettings.hiddenFolders.length > 0;
    await restoreLibraryDefaults(s.libraryDefaults ?? []);
    await restoreDefaults(s.defaults);
    if (foldersBack) startSync(true);
  }

  function restoreConfirm(s: Sector) {
    return {
      title: m.settings_restore_title({ sector: s.label() }),
      message: m.settings_restore_message(),
    };
  }
</script>

<div class="page">
  <header class="header">
    <h1 class="page-title">{m.nav_settings()}</h1>
    <ListSearch bind:value={query} placeholder={m.settings_search()} />
  </header>

  <div class="body">
    <nav class="sectors" aria-label={m.settings_sectors_label()}>
      {#each SECTORS as s (s.id)}
        {@const active = !found && s.id === sector?.id}
        <a href={`/settings/${s.id}`} class:active aria-current={active ? "page" : undefined}>
          <s.icon size={18} />
          <span>{s.label()}</span>
        </a>
      {/each}
    </nav>

    <div class="content">
      {#if found}
        {#each foundSectors as s (s.id)}
          {@const Content = CONTENT[s.id]}
          <section class="result" aria-label={s.label()}>
            <h2 class="sector-title">{s.label()}</h2>
            <Content />
          </section>
        {:else}
          <p class="empty">{m.settings_no_results({ query: query.trim() })}</p>
        {/each}
      {:else if sector}
        {@const Content = CONTENT[sector.id]}
        <h2 class="sector-title">{sector.label()}</h2>
        <Content />
        {#if hasDefaults(sector)}
          {@const current = sector}
          <div class="restore">
            <ActionSetting label={m.settings_restore()} confirm={restoreConfirm(current)} onclick={() => restore(current)} />
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-4);
  }

  .body {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    align-items: start;
    gap: var(--space-6);
    margin-top: var(--space-6);
  }

  .sectors {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* Same pills as the sidebar (spec 4.2). */
  .sectors a {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-pill);
    color: var(--color-text-muted);
    font-size: 14px;
    font-weight: var(--font-weight-medium);
    text-decoration: none;
    transition:
      background-color var(--transition-fast),
      color var(--transition-fast);
  }

  .sectors a:hover {
    color: var(--color-text);
  }

  .sectors a.active {
    background: var(--color-surface-active);
    color: var(--color-text);
  }

  .content {
    max-width: 760px;
  }

  .sector-title {
    margin-bottom: var(--space-4);
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  .result + .result {
    margin-top: var(--space-6);
  }

  .restore {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-6);
    padding-top: var(--space-5);
    border-top: 1px solid var(--color-border);
  }

  .empty {
    color: var(--color-text-muted);
  }
</style>
