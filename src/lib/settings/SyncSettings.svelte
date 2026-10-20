<script lang="ts">
  // Settings > Sync (spec 6.10): automatic interval; "Sync now" with the date
  // and result of the last sync; the full sync that rebuilds the library
  // keeping the play history; continuing another device's queue; and the
  // plays waiting to be sent, when there are any.
  import { getSyncStatus, resendScrobbles, type SyncStatus } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { formatDateTime, formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import ActionSetting from "#lib/settings/ActionSetting.svelte";
  import ChoiceSetting from "#lib/settings/ChoiceSetting.svelte";
  import InfoSetting from "#lib/settings/InfoSetting.svelte";
  import { librarySettings, updateLibrarySettings } from "#lib/settings/library-settings.svelte.ts";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";
  import SwitchSetting from "#lib/settings/SwitchSetting.svelte";
  import { phaseLabel, progressLabel, startRebuild, startSync, syncState } from "#lib/sync.svelte.ts";
  import { showToast } from "#lib/toasts.svelte.ts";

  const intervals = [
    { value: 15, label: m.settings_sync_minutes({ minutes: 15 }) },
    { value: 30, label: m.settings_sync_minutes({ minutes: 30 }) },
    { value: 60, label: m.settings_sync_hour() },
    { value: 0, label: m.settings_sync_manual() },
  ];

  let status = $state<SyncStatus | null>(null);
  let sending = $state(false);

  // On open and after every sync.
  $effect(() => {
    void syncState.finishedCount;
    refresh();
  });

  function refresh() {
    getSyncStatus()
      .then((s) => (status = s))
      .catch((e) => console.error("settings: could not read the sync status", e));
  }

  let lastError = $derived(status?.lastError ? describeError(status.lastError).message : null);
  let lastSync = $derived(status?.lastSyncAt ? m.home_last_sync({ date: formatDateTime(status.lastSyncAt) }) : m.home_never_synced());
  let running = $derived(syncState.running || (status?.syncing ?? false));
  let progress = $derived.by(() => {
    const count = progressLabel(syncState.progress);
    return `${phaseLabel(syncState.progress)}${count ? ` (${count})` : ""}`;
  });

  async function resend() {
    sending = true;
    try {
      const sent = await resendScrobbles();
      showToast(m.settings_pending_sent({ count: sent, countText: formatNumber(sent) }));
    } catch (e) {
      showToast(describeError(e).message);
    } finally {
      sending = false;
      refresh();
    }
  }
</script>

<SettingsGroup title={m.settings_group_automatic()} options={["sync-interval"]}>
  <SettingRow option="sync-interval">
    <ChoiceSetting
      value={librarySettings.syncIntervalMinutes}
      choices={intervals}
      onchange={(syncIntervalMinutes) => updateLibrarySettings({ syncIntervalMinutes })}
    />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title={m.settings_group_now()} options={["sync-now", "full-sync", "pending-scrobbles"]}>
  <SettingRow option="sync-now">
    <div class="line" aria-live="polite">
      {#if running}
        <InfoSetting value={m.home_syncing()}>{progress}</InfoSetting>
      {:else}
        <InfoSetting value={lastError ? m.settings_sync_failed() : m.settings_sync_done()} tone={lastError ? "error" : "ok"}>
          {lastSync}{#if lastError}<br />{lastError}{/if}
        </InfoSetting>
      {/if}
      <ActionSetting label={m.home_sync_now()} disabled={running} onclick={() => startSync(true)} />
    </div>
  </SettingRow>
  <SettingRow option="full-sync">
    <ActionSetting
      label={m.settings_full_sync_button()}
      disabled={running}
      confirm={{ title: m.settings_full_sync_confirm_title(), message: m.settings_full_sync_confirm() }}
      onclick={() => startRebuild()}
    />
  </SettingRow>
  {#if status && status.pendingScrobbles > 0}
    <SettingRow option="pending-scrobbles">
      <div class="line">
        <InfoSetting value={m.settings_pending_count({ count: status.pendingScrobbles, countText: formatNumber(status.pendingScrobbles) })} />
        <ActionSetting label={m.settings_pending_resend()} disabled={sending} onclick={resend} />
      </div>
    </SettingRow>
  {/if}
</SettingsGroup>

<SettingsGroup title={m.settings_group_queue()} options={["continue-queue"]}>
  <SettingRow option="continue-queue">
    <SwitchSetting
      checked={librarySettings.continueOtherDevice}
      onchange={(continueOtherDevice) => updateLibrarySettings({ continueOtherDevice })}
    />
  </SettingRow>
</SettingsGroup>

<style>
  .line {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
</style>
