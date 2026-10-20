<script lang="ts">
  // Settings > About (spec 6.10): logo, version and description; GitHub and
  // contact; the logs folder, for reporting problems (the logs never carry
  // passwords, tokens or URLs with credentials, spec 12.5); and the
  // third-party licenses (`npm run licenses` writes the list).
  import { getVersion } from "@tauri-apps/api/app";
  import { openLink, openLogsFolder, type ExternalLink } from "#lib/api.ts";
  import wordmark from "#lib/assets/logo-wordmark.png";
  import { describeError } from "#lib/errors.ts";
  import { formatNumber } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";
  import ActionSetting from "#lib/settings/ActionSetting.svelte";
  import InfoSetting from "#lib/settings/InfoSetting.svelte";
  import { getSettingsFilter } from "#lib/settings/registry.ts";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";
  import licenses from "#lib/settings/third-party-licenses.json";
  import { showToast } from "#lib/toasts.svelte.ts";

  const filter = getSettingsFilter();
  let version = $state("");

  $effect(() => {
    getVersion()
      .then((v) => (version = v))
      .catch(() => (version = ""));
  });

  function open(action: () => Promise<void>) {
    action().catch((e) => showToast(describeError(e).message));
  }

  const link = (target: ExternalLink) => () => open(() => openLink(target));

  const LISTS = [
    { title: m.settings_licenses_native, items: licenses.native },
    { title: m.settings_licenses_web, items: licenses.web },
    { title: m.settings_licenses_rust, items: licenses.rust },
  ];
</script>

{#if filter.shows("version")}
  <div class="identity">
    <img src={wordmark} alt={m.app_name()} height="40" />
    <p class="description">{m.settings_about_description()}</p>
  </div>
{/if}

<SettingsGroup title={m.settings_group_app()} options={["version", "repository", "contact"]}>
  <SettingRow option="version">
    <InfoSetting value={version} />
  </SettingRow>
  <SettingRow option="repository">
    <ActionSetting label={m.settings_repository_button()} onclick={link("repository")} />
  </SettingRow>
  <SettingRow option="contact">
    <ActionSetting label="ryanbordin@proton.me" onclick={link("email")} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title={m.settings_group_support()} options={["logs", "licenses"]}>
  <SettingRow option="logs">
    <ActionSetting label={m.settings_logs_button()} onclick={() => open(openLogsFolder)} />
  </SettingRow>
  <SettingRow option="licenses" stacked>
    <div class="licenses">
      {#each LISTS as list (list.title)}
        <details>
          <summary>{list.title()} ({formatNumber(list.items.length)})</summary>
          <ul>
            {#each list.items as item (`${item.name}@${item.version}`)}
              <li>
                <span class="name">{item.name}</span>
                <span class="version">{item.version}</span>
                <span class="license">{item.license}</span>
                {#if "note" in item && item.note}<span class="note">{item.note}</span>{/if}
              </li>
            {/each}
          </ul>
        </details>
      {/each}
    </div>
  </SettingRow>
</SettingsGroup>

<style>
  .identity {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    margin-bottom: var(--space-6);
  }

  .identity img {
    width: auto;
  }

  .description {
    max-width: 560px;
    color: var(--color-text-muted);
    font-size: 14px;
    line-height: 1.5;
  }

  .licenses {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  details {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  summary {
    padding: var(--space-3) var(--space-4);
    font-size: 14px;
    font-weight: var(--font-weight-medium);
    cursor: pointer;
  }

  ul {
    max-height: 320px;
    overflow-y: auto;
    margin: 0;
    padding: 0 var(--space-4) var(--space-3);
    list-style: none;
  }

  li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-3);
    padding: var(--space-1) 0;
    font-size: var(--font-size-sm);
  }

  .version,
  .note {
    color: var(--color-text-muted);
  }

  .license {
    margin-left: auto;
    color: var(--color-accent-text);
  }

  .note {
    flex-basis: 100%;
    font-size: var(--font-size-xs);
  }
</style>
