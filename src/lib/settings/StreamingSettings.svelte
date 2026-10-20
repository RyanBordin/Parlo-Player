<script lang="ts">
  // Settings > Streaming (spec 6.10): the original file or a version
  // converted by the server (format and highest bitrate). Applies from the
  // next song on.
  import type { StreamQuality } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { appSettings, updateSettings } from "#lib/settings/app-settings.svelte.ts";
  import ChoiceSetting from "#lib/settings/ChoiceSetting.svelte";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";

  const BIT_RATES = [320, 256, 192, 128];

  const modes: { value: StreamQuality["mode"]; label: string }[] = [
    { value: "original", label: m.settings_stream_original() },
    { value: "converted", label: m.settings_stream_converted() },
  ];
  const formats: { value: StreamQuality["format"]; label: string }[] = [
    { value: "opus", label: "Opus" },
    { value: "mp3", label: "MP3" },
  ];
  const bitRates = BIT_RATES.map((value) => ({ value, label: m.tracklist_kbps({ value }) }));

  let original = $derived(appSettings.stream.mode === "original");

  function change(stream: Partial<StreamQuality>) {
    updateSettings({ stream: { ...appSettings.stream, ...stream } });
  }
</script>

<SettingsGroup title={m.settings_group_quality()} options={["stream-quality", "stream-format", "stream-bit-rate"]}>
  <SettingRow option="stream-quality">
    <ChoiceSetting value={appSettings.stream.mode} choices={modes} onchange={(mode) => change({ mode })} />
  </SettingRow>
  <SettingRow option="stream-format">
    <ChoiceSetting value={appSettings.stream.format} choices={formats} disabled={original} onchange={(format) => change({ format })} />
  </SettingRow>
  <SettingRow option="stream-bit-rate">
    <ChoiceSetting
      value={appSettings.stream.maxBitRate}
      choices={bitRates}
      disabled={original}
      onchange={(maxBitRate) => change({ maxBitRate })}
    />
  </SettingRow>
</SettingsGroup>
