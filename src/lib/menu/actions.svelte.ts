// What the menu items do beyond one command (spec 8.1, 8.2): ask when
// needed, confirm with a discreet toast, and keep the pages already open in
// step with the change.
import { goto } from "$app/navigation";
import { page } from "$app/state";
import {
  addToPlaylist,
  deletePlaylist,
  enqueue,
  getPlaylistPage,
  removeFromPlaylist,
  setAlbumStarred,
  setArtistStarred,
  setSongsStarred,
  updatePlaylist,
  type ItemTarget,
  type Placement,
  type PlaylistEntry,
  type PlaylistSummary,
} from "#lib/api.ts";
import { albumsView } from "#lib/albums.svelte.ts";
import { artistsView } from "#lib/artists.svelte.ts";
import { favorites, refreshFavorites } from "#lib/favorites.svelte.ts";
import { formatNumber } from "#lib/format.ts";
import { askDuplicates } from "#lib/menu/dialogs.svelte.ts";
import { albumPages, artistPages, keepPlaylist, playlistPages } from "#lib/page-cache.svelte.ts";
import { m } from "#lib/paraglide/messages.js";
import { player } from "#lib/player.svelte.ts";
import { loadPlaylists } from "#lib/playlists.svelte.ts";
import { showToast } from "#lib/toasts.svelte.ts";

/** "Go to similar artists": the artist page scrolls to that section once it shows. */
export const artistSection = $state<{ similarOf: string | null }>({ similarOf: null });

export async function queueSongs(target: ItemTarget, placement: Placement, label: string): Promise<void> {
  await enqueue(target, placement, label);
  showToast(placement === "next" ? m.toast_playing_next() : m.toast_added_to_queue());
}

/** A song shown somewhere: its heart follows the change. */
type Starrable = { id: string; starred: boolean };

/** Stars or unstars songs; the hearts shown change at once. */
export async function starSongs(songs: Starrable[], starred: boolean): Promise<void> {
  await setSongsStarred(
    songs.map((song) => song.id),
    starred,
  );
  for (const song of songs) song.starred = starred;
  if (player.track && songs.some((song) => song.id === player.track?.id)) player.track.starred = starred;
  if (favorites.data) refreshFavorites(true);
}

export async function starAlbum(id: string, starred: boolean): Promise<void> {
  await setAlbumStarred(id, starred);
  const cached = albumPages[id];
  if (cached) cached.album.starred = starred;
  const item = albumsView.items?.find((album) => album.id === id);
  if (item) item.starred = starred;
  if (favorites.data) refreshFavorites(true);
}

export async function starArtist(id: string, starred: boolean): Promise<void> {
  await setArtistStarred(id, starred);
  const cached = artistPages[id];
  if (cached) cached.artist.starred = starred;
  const item = artistsView.items?.find((artist) => artist.id === id);
  if (item) item.starred = starred;
  if (favorites.data) refreshFavorites(true);
}

export function goToSimilarArtists(artistId: string): Promise<void> {
  artistSection.similarOf = artistId;
  return goto(`/artists/${encodeURIComponent(artistId)}`);
}

export async function copySongText(title: string, artist: string | null): Promise<void> {
  await navigator.clipboard.writeText(artist ? `${title} – ${artist}` : title);
  showToast(m.toast_copied());
}

/** The playlist changed: the sidebar and its page, if it was opened, read it again. */
async function playlistChanged(id: string): Promise<void> {
  await loadPlaylists();
  if (playlistPages[id] !== undefined) keepPlaylist(id, await getPlaylistPage(id));
}

/**
 * "Add to playlist ›" (spec 8.2): when some songs are already there, asks
 * whether to add them anyway; then a discreet confirmation.
 */
export async function addSongsToPlaylist(playlist: PlaylistSummary, target: ItemTarget): Promise<void> {
  let outcome = await addToPlaylist(playlist.id, target, null);
  if (outcome.kind === "duplicates") {
    const choice = await askDuplicates(outcome.count, outcome.total, playlist.name);
    if (!choice) return;
    outcome = await addToPlaylist(playlist.id, target, choice);
  }
  if (outcome.kind !== "added") return;
  const count = outcome.count;
  if (count === 0) showToast(m.toast_nothing_added({ name: playlist.name }));
  else if (target.kind === "songs" && target.ids.length === 1) showToast(m.toast_added_to_playlist({ name: playlist.name }));
  else showToast(m.toast_added_many_to_playlist({ count, countText: formatNumber(count), name: playlist.name }));
  await playlistChanged(playlist.id);
}

export async function removeSongsFromPlaylist(playlist: { id: string; name: string }, entries: PlaylistEntry[]): Promise<void> {
  await removeFromPlaylist(playlist.id, entries);
  showToast(m.toast_removed_from_playlist({ name: playlist.name }));
  await playlistChanged(playlist.id);
}

export async function setPlaylistPublic(playlist: PlaylistSummary, isPublic: boolean): Promise<void> {
  await updatePlaylist(playlist.id, { public: isPublic });
  showToast(isPublic ? m.toast_playlist_public({ name: playlist.name }) : m.toast_playlist_private({ name: playlist.name }));
  await loadPlaylists();
}

export async function editPlaylist(playlist: PlaylistSummary, name: string, comment: string): Promise<void> {
  await updatePlaylist(playlist.id, { name, comment });
  showToast(m.toast_playlist_updated());
  await playlistChanged(playlist.id);
}

/** After the user confirmed. Leaves the playlist's page if it is open. */
export async function deletePlaylistNow(playlist: PlaylistSummary): Promise<void> {
  await deletePlaylist(playlist.id);
  showToast(m.toast_playlist_deleted({ name: playlist.name }));
  await loadPlaylists();
  if (page.route.id === "/playlists/[id]" && page.params.id === playlist.id) await goto("/");
  if (playlistPages[playlist.id] !== undefined) keepPlaylist(playlist.id, null);
}
