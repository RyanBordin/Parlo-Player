<script lang="ts">
  // Settings > Playback and audio (spec 6.10): ReplayGain, output device,
  // exclusive mode (Windows), mix size and scrobble. Changes reach the song
  // playing at once; the next song is loaded again with them.
  import { listen } from "@tauri-apps/api/event";
  import { AUDIO_DEVICES_EVENT, getAudioDevices, type AppSettings, type AudioDevice } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { appSettings, updateSettings } from "#lib/settings/app-settings.svelte.ts";
  import ChoiceSetting from "#lib/settings/ChoiceSetting.svelte";
  import { librarySettings, updateLibrarySettings } from "#lib/settings/library-settings.svelte.ts";
  import { IS_WINDOWS } from "#lib/settings/registry.ts";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";
  import SliderSetting from "#lib/settings/SliderSetting.svelte";
  import SwitchSetting from "#lib/settings/SwitchSetting.svelte";

  const MIX_SIZES = [50, 100, 200, 500];

  const modes: { value: AppSettings["replayGain"]; label: string }[] = [
    { value: "off", label: m.settings_off() },
    { value: "track", label: m.settings_replay_gain_track() },
    { value: "album", label: m.settings_replay_gain_album() },
    { value: "auto", label: m.settings_replay_gain_auto() },
  ];
  const mixSizes = MIX_SIZES.map((value) => ({ value, label: String(value) }));

  let devices = $state<AudioDevice[]>([]);
  let gainOff = $derived(appSettings.replayGain === "off");

  // The outputs now, and again when one is plugged in or removed.
  $effect(() => {
    getAudioDevices()
      .then((list) => (devices = list))
      .catch((e) => console.error("settings: could not list the audio devices", e));
    const unlisten = listen<AudioDevice[]>(AUDIO_DEVICES_EVENT, (event) => (devices = event.payload));
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  /** The system's default first; the chosen one stays listed while it is unplugged. */
  let deviceChoices = $derived.by(() => {
    const choices = devices.map((device) => ({
      value: device.name,
      label: device.name === "auto" ? m.settings_audio_device_default() : device.description,
    }));
    if (!choices.some((c) => c.value === "auto")) choices.unshift({ value: "auto", label: m.settings_audio_device_default() });
    if (!choices.some((c) => c.value === appSettings.audioDevice)) {
      choices.push({ value: appSettings.audioDevice, label: m.settings_audio_device_missing() });
    }
    return choices;
  });

  function decibels(value: number): string {
    return `${value > 0 ? "+" : ""}${value} dB`;
  }
</script>

<SettingsGroup title={m.settings_group_replay_gain()} options={["replay-gain", "preamp", "avoid-clipping"]}>
  <SettingRow option="replay-gain">
    <ChoiceSetting value={appSettings.replayGain} choices={modes} onchange={(replayGain) => updateSettings({ replayGain })} />
  </SettingRow>
  <SettingRow option="preamp">
    <SliderSetting
      value={appSettings.replayGainPreamp}
      min={-15}
      max={15}
      format={decibels}
      disabled={gainOff}
      onchange={(replayGainPreamp) => updateSettings({ replayGainPreamp })}
    />
  </SettingRow>
  <SettingRow option="avoid-clipping">
    <SwitchSetting
      checked={appSettings.avoidClipping}
      disabled={gainOff}
      onchange={(avoidClipping) => updateSettings({ avoidClipping })}
    />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title={m.settings_group_output()} options={["audio-device", "exclusive-audio"]}>
  <SettingRow option="audio-device">
    <ChoiceSetting
      value={appSettings.audioDevice}
      choices={deviceChoices}
      onchange={(audioDevice) => updateSettings({ audioDevice })}
    />
  </SettingRow>
  {#if IS_WINDOWS}
    <SettingRow option="exclusive-audio">
      <SwitchSetting checked={appSettings.exclusiveAudio} onchange={(exclusiveAudio) => updateSettings({ exclusiveAudio })} />
    </SettingRow>
  {/if}
</SettingsGroup>

<SettingsGroup title={m.settings_group_playback()} options={["mix-size", "scrobble"]}>
  <SettingRow option="mix-size">
    <ChoiceSetting value={appSettings.mixSize} choices={mixSizes} onchange={(mixSize) => updateSettings({ mixSize })} />
  </SettingRow>
  <SettingRow option="scrobble">
    <SwitchSetting checked={librarySettings.scrobble} onchange={(scrobble) => updateLibrarySettings({ scrobble })} />
  </SettingRow>
</SettingsGroup>
