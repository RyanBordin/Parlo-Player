// Playlists from the local cache, shared by the sidebar and the playlist page.
import { listPlaylists, type PlaylistSummary } from "#lib/api.ts";
import { formatNumber } from "#lib/format.ts";
import { m } from "#lib/paraglide/messages.js";

export const playlists = $state<{ items: PlaylistSummary[]; loaded: boolean }>({
  items: [],
  loaded: false,
});

/** Reads the list again (on open and after each sync). */
export async function loadPlaylists(): Promise<void> {
  try {
    playlists.items = await listPlaylists();
  } catch {
    // Not signed in or unreadable cache: keep what is shown.
  } finally {
    playlists.loaded = true;
  }
}

/** Sidebar subtitle (spec 4.2): owner and number of songs, e.g. "ryan • 42 songs". */
export function playlistSubtitle(playlist: PlaylistSummary): string {
  const count = playlist.songCount;
  const songs =
    count == null
      ? null
      : count === 0
        ? m.playlist_song_count_zero()
        : m.playlist_song_count({ count, countText: formatNumber(count) });
  return [playlist.owner, songs].filter(Boolean).join(" • ");
}
