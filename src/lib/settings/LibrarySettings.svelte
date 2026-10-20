<script lang="ts">
  // Settings > Library (spec 6.10): counts and space of the local database;
  // which music folders show (only with more than one: the sync then
  // downloads just the visible ones); the cover cache, its limit and "Clear".
  import { clearCoverCache, getLibrarySummary, getMusicFolders, type LibrarySummary, type MusicFolder } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { formatBytes, formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import ActionSetting from "#lib/settings/ActionSetting.svelte";
  import { appSettings, updateSettings } from "#lib/settings/app-settings.svelte.ts";
  import ChoiceSetting from "#lib/settings/ChoiceSetting.svelte";
  import { librarySettings, updateLibrarySettings } from "#lib/settings/library-settings.svelte.ts";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";
  import SwitchSetting from "#lib/settings/SwitchSetting.svelte";
  import { startSync, syncState } from "#lib/sync.svelte.ts";
  import { showToast } from "#lib/toasts.svelte.ts";

  const CACHE_SIZES = [250, 500, 1024, 2048];

  let summary = $state<LibrarySummary | null>(null);
  let folders = $state<MusicFolder[]>([]);

  const cacheSizes = CACHE_SIZES.map((mb) => ({ value: mb, label: formatBytes(mb * 1024 * 1024) }));

  // On open and after every sync (the counts change).
  $effect(() => {
    void syncState.finishedCount;
    refresh();
  });

  // Asked once: without connection, the folders are not shown.
  $effect(() => {
    getMusicFolders()
      .then((list) => (folders = list))
      .catch((e) => console.error("settings: could not read the music folders", e));
  });

  function refresh() {
    getLibrarySummary()
      .then((s) => (summary = s))
      .catch((e) => console.error("settings: could not read the library summary", e));
  }

  let visibleCount = $derived(folders.filter((f) => !librarySettings.hiddenFolders.includes(f.id)).length);

  /** The library changes: the full sync downloads the folders shown now. */
  async function setFolderVisible(id: string, visible: boolean) {
    const hidden = librarySettings.hiddenFolders.filter((folder) => folder !== id);
    if (!visible) hidden.push(id);
    await updateLibrarySettings({ hiddenFolders: hidden });
    startSync(true);
  }

  async function clearCovers() {
    try {
      await clearCoverCache();
      showToast(m.settings_cover_cache_cleared());
    } catch (e) {
      showToast(describeError(e).message);
    }
    refresh();
  }
</script>

<SettingsGroup title={m.settings_group_summary()} options={["library-summary"]}>
  <SettingRow option="library-summary" stacked>
    <dl class="stats">
      <div>
        <dt>{m.home_stat_songs()}</dt>
        <dd>{summary ? formatNumber(summary.songs) : "–"}</dd>
      </div>
      <div>
        <dt>{m.home_stat_albums()}</dt>
        <dd>{summary ? formatNumber(summary.albums) : "–"}</dd>
      </div>
      <div>
        <dt>{m.home_stat_artists()}</dt>
        <dd>{summary ? formatNumber(summary.artists) : "–"}</dd>
      </div>
      <div>
        <dt>{m.settings_database_size()}</dt>
        <dd>{summary ? formatBytes(summary.databaseBytes) : "–"}</dd>
      </div>
    </dl>
  </SettingRow>
</SettingsGroup>

{#if folders.length > 1}
  <SettingsGroup title={m.settings_group_folders()} options={["music-folders"]}>
    <SettingRow option="music-folders" stacked>
      <ul class="folders">
        {#each folders as folder (folder.id)}
          {@const visible = !librarySettings.hiddenFolders.includes(folder.id)}
          {@const locked = visible && visibleCount <= 1}
          <li>
            <span class:hidden-folder={!visible}>{folder.name}</span>
            <SwitchSetting
              checked={visible}
              disabled={locked || syncState.running}
              title={locked ? m.settings_music_folders_min() : undefined}
              label={m.settings_show_item({ name: folder.name })}
              onchange={(show) => setFolderVisible(folder.id, show)}
            />
          </li>
        {/each}
      </ul>
    </SettingRow>
  </SettingsGroup>
{/if}

<SettingsGroup title={m.settings_group_covers()} options={["cover-cache-limit", "cover-cache-clear"]}>
  <SettingRow option="cover-cache-limit">
    <ChoiceSetting value={appSettings.coverCacheMb} choices={cacheSizes} onchange={(coverCacheMb) => updateSettings({ coverCacheMb })} />
  </SettingRow>
  <SettingRow option="cover-cache-clear">
    <div class="line">
      <span class="muted">{summary ? m.settings_cover_cache_used({ size: formatBytes(summary.coverCacheBytes) }) : ""}</span>
      <ActionSetting
        label={m.settings_cover_cache_clear_button()}
        confirm={{ title: m.settings_cover_cache_confirm_title(), message: m.settings_cover_cache_confirm() }}
        onclick={clearCovers}
      />
    </div>
  </SettingRow>
</SettingsGroup>

<style>
  .stats {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: var(--space-3);
    margin: 0;
  }

  .stats div {
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  dt {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  dd {
    margin: var(--space-1) 0 0;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .folders {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    list-style: none;
  }

  .folders li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-height: 48px;
    padding: 0 var(--space-4);
    font-size: 14px;
  }

  .folders li + li {
    border-top: 1px solid var(--color-border);
  }

  .hidden-folder {
    color: var(--color-text-muted);
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }
</style>
