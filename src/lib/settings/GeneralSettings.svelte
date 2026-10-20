<script lang="ts">
  // Settings > General (spec 6.10): language and the keyboard shortcuts.
  // Starting with the system, closing to the tray and track notifications
  // come with the tray (system integration step).
  import { m } from "#lib/paraglide/messages.js";
  import { locales } from "#lib/paraglide/runtime.js";
  import { appSettings, setLanguage } from "#lib/settings/app-settings.svelte.ts";
  import ChoiceSetting from "#lib/settings/ChoiceSetting.svelte";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";
  import { SHORTCUTS } from "#lib/shortcuts.ts";

  const languages = [
    { value: "system" as const, label: m.settings_follow_system() },
    // Each language in its own language, e.g. "English".
    ...locales.map((locale) => ({ value: locale, label: m.language_name({}, { locale }) })),
  ];
</script>

<SettingsGroup title={m.settings_group_language()} options={["language"]}>
  <SettingRow option="language">
    <ChoiceSetting value={appSettings.language} choices={languages} onchange={setLanguage} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title={m.settings_group_keyboard()} options={["shortcuts"]}>
  <SettingRow option="shortcuts" stacked>
    <dl class="shortcuts">
      {#each SHORTCUTS as shortcut (shortcut.label())}
        <div class="shortcut">
          <dt>
            {#each shortcut.keys() as key, index (index)}
              {#if index > 0}<span class="plus">+</span>{/if}<kbd>{key}</kbd>
            {/each}
          </dt>
          <dd>{shortcut.label()}</dd>
        </div>
      {/each}
    </dl>
  </SettingRow>
</SettingsGroup>

<style>
  .shortcuts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--space-2) var(--space-5);
    margin: 0;
  }

  .shortcut {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  dt {
    min-width: 96px;
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  dd {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }

  kbd {
    min-width: 24px;
    padding: 2px var(--space-2);
    border: 1px solid var(--color-border-focus);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text);
    font: inherit;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    text-align: center;
  }

  .plus {
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
  }
</style>
