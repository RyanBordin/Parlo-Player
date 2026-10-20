<script lang="ts">
  // Settings > Appearance (spec 6.10): arrange the Home and the sidebar,
  // list density, interface scale and reduce motion. Themes come later.
  import type { AppSettings } from "#lib/api.ts";
  import { homeLayout } from "#lib/home.svelte.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { appSettings, updateSettings } from "#lib/settings/app-settings.svelte.ts";
  import ChoiceSetting from "#lib/settings/ChoiceSetting.svelte";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";
  import SortableListSetting from "#lib/settings/SortableListSetting.svelte";
  import { SETTINGS_ITEM, sidebarLayout } from "#lib/shell/nav-items.ts";

  const SCALES = [90, 100, 110, 125];

  let homeItems = $derived(homeLayout().map((row) => ({ id: row.id, label: row.label(), visible: row.visible })));
  let sidebarItems = $derived(
    sidebarLayout().map((item) => ({ id: item.id, label: item.label(), icon: item.icon, visible: item.visible })),
  );
  const fixed = [{ id: SETTINGS_ITEM.id, label: SETTINGS_ITEM.label(), icon: SETTINGS_ITEM.icon }];

  const densities: { value: AppSettings["density"]; label: string }[] = [
    { value: "comfortable", label: m.settings_density_comfortable() },
    { value: "compact", label: m.settings_density_compact() },
  ];
  const scales = SCALES.map((value) => ({ value, label: `${value}%` }));
  const motions: { value: AppSettings["reduceMotion"]; label: string }[] = [
    { value: "system", label: m.settings_follow_system() },
    { value: "on", label: m.settings_on() },
    { value: "off", label: m.settings_off() },
  ];
</script>

<SettingsGroup title={m.settings_group_layout()} options={["home-layout", "sidebar-layout"]}>
  <SettingRow option="home-layout" stacked>
    <SortableListSetting
      items={homeItems}
      minVisible={1}
      minVisibleHint={m.settings_min_visible()}
      onchange={(homeSections) => updateSettings({ homeSections })}
    />
  </SettingRow>
  <SettingRow option="sidebar-layout" stacked>
    <SortableListSetting items={sidebarItems} {fixed} onchange={(sidebarItems) => updateSettings({ sidebarItems })} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title={m.settings_group_display()} options={["density", "scale", "reduce-motion"]}>
  <SettingRow option="density">
    <ChoiceSetting value={appSettings.density} choices={densities} onchange={(density) => updateSettings({ density })} />
  </SettingRow>
  <SettingRow option="scale">
    <ChoiceSetting value={appSettings.scale} choices={scales} onchange={(scale) => updateSettings({ scale })} />
  </SettingRow>
  <SettingRow option="reduce-motion">
    <ChoiceSetting
      value={appSettings.reduceMotion}
      choices={motions}
      onchange={(reduceMotion) => updateSettings({ reduceMotion })}
    />
  </SettingRow>
</SettingsGroup>
