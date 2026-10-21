// Settings page (spec 6.10): the sectors shown in this version, every option
// with its name and description (what the search looks at), and the
// preferences each sector's "Restore defaults" puts back. Sectors of later
// versions are not listed until they exist (no "Coming soon").
import AudioLines from "@lucide/svelte/icons/audio-lines";
import Info from "@lucide/svelte/icons/info";
import LibraryBig from "@lucide/svelte/icons/library-big";
import Palette from "@lucide/svelte/icons/palette";
import Radio from "@lucide/svelte/icons/radio";
import RefreshCw from "@lucide/svelte/icons/refresh-cw";
import Server from "@lucide/svelte/icons/server";
import SlidersHorizontal from "@lucide/svelte/icons/sliders-horizontal";
import { getContext, setContext } from "svelte";
import type { AppSettings, LibrarySettings } from "#lib/api.ts";
import { m } from "#lib/paraglide/messages.js";

export type SectorId =
  | "general"
  | "appearance"
  | "playback"
  | "streaming"
  | "server"
  | "sync"
  | "library"
  | "about";

export type Sector = {
  id: SectorId;
  label: () => string;
  icon: typeof Palette;
  /** App preferences put back by "Restore defaults". */
  defaults: (keyof AppSettings)[];
  /** Server preferences put back by "Restore defaults". No defaults at all: no button. */
  libraryDefaults?: (keyof LibrarySettings)[];
};

/** In the order of spec 6.10. */
export const SECTORS: Sector[] = [
  { id: "general", label: m.settings_sector_general, icon: SlidersHorizontal, defaults: ["language"] },
  {
    id: "appearance",
    label: m.settings_sector_appearance,
    icon: Palette,
    defaults: ["homeSections", "sidebarItems", "density", "scale", "reduceMotion", "nowPlayingBackdrop"],
  },
  {
    id: "playback",
    label: m.settings_sector_playback,
    icon: AudioLines,
    defaults: ["replayGain", "replayGainPreamp", "avoidClipping", "audioDevice", "exclusiveAudio", "mixSize"],
    libraryDefaults: ["scrobble"],
  },
  { id: "streaming", label: m.settings_sector_streaming, icon: Radio, defaults: ["stream"] },
  { id: "server", label: m.settings_sector_server, icon: Server, defaults: [] },
  {
    id: "sync",
    label: m.settings_sector_sync,
    icon: RefreshCw,
    defaults: [],
    libraryDefaults: ["syncIntervalMinutes", "continueOtherDevice"],
  },
  {
    id: "library",
    label: m.settings_sector_library,
    icon: LibraryBig,
    defaults: ["coverCacheMb"],
    libraryDefaults: ["hiddenFolders"],
  },
  { id: "about", label: m.settings_sector_about, icon: Info, defaults: [] },
];

/** Exclusive mode exists only on Windows (spec 6.10). */
export const IS_WINDOWS = typeof navigator !== "undefined" && navigator.userAgent.includes("Windows");

export function findSector(id: string | undefined): Sector | undefined {
  return SECTORS.find((sector) => sector.id === id);
}

/** The last sector opened is remembered (spec 6.10), on this computer. */
const SECTOR_KEY = "parlo.settings.sector";

export function rememberSector(id: SectorId): void {
  try {
    localStorage.setItem(SECTOR_KEY, id);
  } catch {
    // Storage unavailable: the page opens on the first sector.
  }
}

export function lastSector(): SectorId {
  try {
    return findSector(localStorage.getItem(SECTOR_KEY) ?? undefined)?.id ?? SECTORS[0].id;
  } catch {
    return SECTORS[0].id;
  }
}

export type SettingOption = {
  id: string;
  sector: SectorId;
  label: () => string;
  /** Only when the name is not obvious (spec 6.10). */
  description?: () => string;
};

export const OPTIONS: SettingOption[] = [
  { id: "language", sector: "general", label: m.settings_language, description: m.settings_language_description },
  { id: "shortcuts", sector: "general", label: m.settings_shortcuts, description: m.settings_shortcuts_description },
  { id: "home-layout", sector: "appearance", label: m.settings_home_layout, description: m.settings_home_layout_description },
  {
    id: "sidebar-layout",
    sector: "appearance",
    label: m.settings_sidebar_layout,
    description: m.settings_sidebar_layout_description,
  },
  { id: "density", sector: "appearance", label: m.settings_density, description: m.settings_density_description },
  { id: "scale", sector: "appearance", label: m.settings_scale, description: m.settings_scale_description },
  {
    id: "reduce-motion",
    sector: "appearance",
    label: m.settings_reduce_motion,
    description: m.settings_reduce_motion_description,
  },
  {
    id: "now-playing-backdrop",
    sector: "appearance",
    label: m.settings_now_playing_backdrop,
    description: m.settings_now_playing_backdrop_description,
  },
  { id: "replay-gain", sector: "playback", label: m.settings_replay_gain, description: m.settings_replay_gain_description },
  { id: "preamp", sector: "playback", label: m.settings_preamp, description: m.settings_preamp_description },
  { id: "avoid-clipping", sector: "playback", label: m.settings_avoid_clipping, description: m.settings_avoid_clipping_description },
  { id: "audio-device", sector: "playback", label: m.settings_audio_device, description: m.settings_audio_device_description },
  ...(IS_WINDOWS
    ? [{ id: "exclusive-audio", sector: "playback" as const, label: m.settings_exclusive, description: m.settings_exclusive_description }]
    : []),
  { id: "mix-size", sector: "playback", label: m.settings_mix_size, description: m.settings_mix_size_description },
  { id: "scrobble", sector: "playback", label: m.settings_scrobble, description: m.settings_scrobble_description },
  { id: "stream-quality", sector: "streaming", label: m.settings_stream_quality, description: m.settings_stream_quality_description },
  { id: "stream-format", sector: "streaming", label: m.settings_stream_format },
  { id: "stream-bit-rate", sector: "streaming", label: m.settings_stream_bit_rate },
  { id: "server-address", sector: "server", label: m.settings_server_address },
  { id: "server-status", sector: "server", label: m.settings_server_status, description: m.settings_server_status_description },
  {
    id: "server-extensions",
    sector: "server",
    label: m.settings_server_extensions,
    description: m.settings_server_extensions_description,
  },
  { id: "account", sector: "server", label: m.settings_account, description: m.settings_account_description },
  { id: "sync-interval", sector: "sync", label: m.settings_sync_interval, description: m.settings_sync_interval_description },
  { id: "sync-now", sector: "sync", label: m.home_sync_now },
  { id: "full-sync", sector: "sync", label: m.settings_full_sync, description: m.settings_full_sync_description },
  {
    id: "continue-queue",
    sector: "sync",
    label: m.settings_continue_queue,
    description: m.settings_continue_queue_description,
  },
  { id: "pending-scrobbles", sector: "sync", label: m.settings_pending, description: m.settings_pending_description },
  { id: "library-summary", sector: "library", label: m.settings_library_summary },
  { id: "music-folders", sector: "library", label: m.settings_music_folders, description: m.settings_music_folders_description },
  { id: "cover-cache-limit", sector: "library", label: m.settings_cover_cache_limit, description: m.settings_cover_cache_limit_description },
  { id: "cover-cache-clear", sector: "library", label: m.settings_cover_cache_clear, description: m.settings_cover_cache_clear_description },
  { id: "version", sector: "about", label: m.settings_version },
  { id: "repository", sector: "about", label: m.settings_repository, description: m.settings_repository_description },
  { id: "contact", sector: "about", label: m.settings_contact },
  { id: "logs", sector: "about", label: m.settings_logs, description: m.settings_logs_description },
  { id: "licenses", sector: "about", label: m.settings_licenses, description: m.settings_licenses_description },
];

export function findOption(id: string): SettingOption {
  const option = OPTIONS.find((o) => o.id === id);
  if (!option) throw new Error(`Unknown setting: ${id}`);
  return option;
}

/** Lower case and without accents: "animações" matches "animacoes". */
function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/**
 * The options whose name or description has every word of `query`, in the
 * current language. `null` when the query is empty (everything shown).
 */
export function searchOptions(query: string): Set<string> | null {
  const words = fold(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return null;
  const found = OPTIONS.filter((option) => {
    const text = fold(`${option.label()} ${option.description?.() ?? ""}`);
    return words.every((word) => text.includes(word));
  });
  return new Set(found.map((option) => option.id));
}

// --- Which options are shown: every one, or the search results. Set by the
// settings page, read by the groups and rows.

type Filter = { shows: (id: string) => boolean };
const FILTER = Symbol("settings-filter");

export function setSettingsFilter(filter: Filter): void {
  setContext(FILTER, filter);
}

export function getSettingsFilter(): Filter {
  return getContext<Filter | undefined>(FILTER) ?? { shows: () => true };
}

// --- The control of a row is named by the row's label (screen readers).

const ROW_LABEL = Symbol("settings-row-label");

export function setRowLabel(id: string): void {
  setContext(ROW_LABEL, id);
}

/** ID of the label of the row around a control, if any. */
export function getRowLabel(): string | undefined {
  return getContext<string | undefined>(ROW_LABEL);
}
