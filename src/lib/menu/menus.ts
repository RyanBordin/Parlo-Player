// The items of each "..." menu (spec 8.1), in the spec's groups. Items that
// do not apply are left out (never disabled), and so are the items of later
// versions (Download, v0.2).
import AudioLines from "@lucide/svelte/icons/audio-lines";
import Copy from "@lucide/svelte/icons/copy";
import Dices from "@lucide/svelte/icons/dices";
import Disc3 from "@lucide/svelte/icons/disc-3";
import ExternalLink from "@lucide/svelte/icons/external-link";
import Globe from "@lucide/svelte/icons/globe";
import Heart from "@lucide/svelte/icons/heart";
import HeartOff from "@lucide/svelte/icons/heart-off";
import Info from "@lucide/svelte/icons/info";
import ListEnd from "@lucide/svelte/icons/list-end";
import ListMinus from "@lucide/svelte/icons/list-minus";
import ListMusic from "@lucide/svelte/icons/list-music";
import ListPlus from "@lucide/svelte/icons/list-plus";
import ListStart from "@lucide/svelte/icons/list-start";
import ListX from "@lucide/svelte/icons/list-x";
import Lock from "@lucide/svelte/icons/lock";
import MicVocal from "@lucide/svelte/icons/mic-vocal";
import Pencil from "@lucide/svelte/icons/pencil";
import Play from "@lucide/svelte/icons/play";
import Plus from "@lucide/svelte/icons/plus";
import Radio from "@lucide/svelte/icons/radio";
import Save from "@lucide/svelte/icons/save";
import Shuffle from "@lucide/svelte/icons/shuffle";
import Trash from "@lucide/svelte/icons/trash";
import Users from "@lucide/svelte/icons/users";
import { goto } from "$app/navigation";
import {
  getAlbumInfo,
  getArtistInfo,
  getSongInfo,
  openInNavidrome,
  playArtistRadio,
  playItem,
  playSimilarSongs,
  removeFromQueue,
  type ItemTarget,
  type SaveTarget,
  type PlaylistEntry,
  type PlaylistSummary,
} from "#lib/api.ts";
import { formatNumber } from "#lib/format.ts";
import {
  addSongsToPlaylist,
  copySongText,
  goToSimilarArtists,
  queueSongs,
  removeSongsFromPlaylist,
  setPlaylistPublic,
  starAlbum,
  starArtist,
  starSongs,
} from "#lib/menu/actions.svelte.ts";
import { showDialog } from "#lib/menu/dialogs.svelte.ts";
import { openMenu, type MenuAction, type MenuGroup, type MenuSpec, type MenuSubmenu } from "#lib/menu/menu.svelte.ts";
import { m } from "#lib/paraglide/messages.js";
import { playlists } from "#lib/playlists.svelte.ts";
import { saveItem } from "#lib/saves.svelte.ts";
import { server } from "#lib/server-info.svelte.ts";
import { app } from "#lib/session.svelte.ts";
import { appSettings } from "#lib/settings/app-settings.svelte.ts";

/** With more playlists, "Add to playlist ›" gets a search field (spec 8.2). */
const PLAYLIST_SEARCH_FROM = 11;

/** A song as the lists show it. Its heart follows a change made from the menu. */
export type MenuSong = {
  id: string;
  title: string;
  artist: string | null;
  artistId: string | null;
  albumId: string | null;
  starred: boolean;
};

/** Where the songs of a menu are. */
export type SongPlace = {
  /** "Playing from" when nothing is queued yet. */
  label: string;
  /** A playlist of the user's, with the songs' entries: "Remove from this playlist". */
  playlist?: { id: string; name: string; entries: PlaylistEntry[] };
  /** Positions in Up Next: "Remove from queue". */
  queue?: number[];
};

function action(label: string, icon: MenuAction["icon"], run: () => unknown): MenuAction {
  return { kind: "action", label, icon, run };
}

function go(path: string): Promise<void> {
  return goto(path);
}

function songsTarget(songs: { id: string }[]): ItemTarget {
  return { kind: "songs", ids: songs.map((song) => song.id) };
}

/** The signed-in user owns it, so it can be edited (Navidrome allows nothing else). */
export function isOwnPlaylist(playlist: { owner: string | null }): boolean {
  const user = app.view.name === "connected" ? app.view.session.username : null;
  return playlist.owner === null || playlist.owner === user;
}

/** "Add to playlist ›": "+ New playlist" first, then the user's playlists, last changed first. */
export function playlistSubmenu(target: ItemTarget): MenuSubmenu {
  const mine = playlists.items
    .filter(isOwnPlaylist)
    .sort((a, b) => (b.changed ?? "").localeCompare(a.changed ?? ""));
  const newPlaylist: MenuAction = {
    ...action(m.menu_new_playlist(), Plus, () => showDialog({ kind: "newPlaylist", target })),
    pinned: true,
  };
  return {
    kind: "submenu",
    label: m.menu_add_to_playlist(),
    icon: ListPlus,
    groups: [[newPlaylist], mine.map((playlist) => action(playlist.name, ListMusic, () => addSongsToPlaylist(playlist, target)))],
    search:
      mine.length >= PLAYLIST_SEARCH_FROM
        ? { placeholder: m.menu_search_playlists(), empty: m.menu_no_playlists() }
        : undefined,
  };
}

/** The Player Bar's quick "Add to playlist" (spec 4.3): the submenu on its own. */
export function addToPlaylistMenu(target: ItemTarget): MenuSpec {
  const { groups, search } = playlistSubmenu(target);
  return { groups, search };
}

/** "Save to computer..." (spec 8.2): the original files, in a place the user chooses. */
function saveAction(target: SaveTarget): MenuAction {
  return action(m.menu_save_to_computer(), Save, () => saveItem(target));
}

/** "Open in Navidrome" (spec 8.2): only when the server is a Navidrome. */
function navidromeActions(kind: "song" | "album" | "artist", id: string): MenuAction[] {
  return server.navidrome ? [action(m.menu_open_in_navidrome(), ExternalLink, () => openInNavidrome(kind, id))] : [];
}

/** "Remove from this playlist" and "Remove from queue", where they apply. */
function placeGroup(place: SongPlace): MenuGroup {
  const group: MenuGroup = [];
  const playlist = place.playlist;
  if (playlist) {
    group.push(action(m.menu_remove_from_playlist(), ListX, () => removeSongsFromPlaylist(playlist, playlist.entries)));
  }
  const positions = place.queue;
  if (positions) group.push(action(m.menu_remove_from_queue(), ListMinus, () => removeFromQueue(positions)));
  return group;
}

/** A song (spec 8.1). Reads the cache for its heart, album and every artist. */
export async function songMenu(song: MenuSong, place: SongPlace): Promise<MenuSpec> {
  const info = await getSongInfo(song.id);
  const target = songsTarget([song]);
  const starred = info?.starred ?? song.starred;
  const albumId = info?.albumId ?? song.albumId;
  const artists = info?.artists.length
    ? info.artists
    : song.artistId && song.artist
      ? [{ id: song.artistId, name: song.artist }]
      : [];

  const navigate: MenuGroup = [];
  if (albumId) navigate.push(action(m.menu_go_to_album(), Disc3, () => go(`/albums/${encodeURIComponent(albumId)}`)));
  if (artists.length === 1) {
    const artist = artists[0];
    navigate.push(action(m.menu_go_to_artist(), MicVocal, () => go(`/artists/${encodeURIComponent(artist.id)}`)));
  } else if (artists.length > 1) {
    navigate.push({
      kind: "submenu",
      label: m.menu_go_to_artist(),
      icon: MicVocal,
      groups: [artists.map((artist) => action(artist.name, MicVocal, () => go(`/artists/${encodeURIComponent(artist.id)}`)))],
    });
  }

  return {
    groups: [
      [
        action(m.menu_play_next(), ListStart, () => queueSongs(target, "next", place.label)),
        action(m.menu_add_to_queue(), ListEnd, () => queueSongs(target, "end", place.label)),
        action(m.menu_play_similar(), AudioLines, () =>
          playSimilarSongs(song.id, m.menu_similar_context({ title: song.title })),
        ),
      ],
      [
        starred
          ? action(m.menu_unfavorite(), HeartOff, () => starSongs([song], false))
          : action(m.menu_favorite(), Heart, () => starSongs([song], true)),
        playlistSubmenu(target),
      ],
      navigate,
      [
        saveAction({ kind: "songs", ids: [song.id] }),
        action(m.menu_copy_title(), Copy, () => copySongText(song.title, song.artist)),
        action(m.menu_file_info(), Info, () => showDialog({ kind: "info", target: { kind: "song", id: song.id } })),
        ...navidromeActions("song", song.id),
      ],
      placeGroup(place),
    ],
  };
}

/** Right click on a multiple selection (spec 5.3, 8.1): for every song selected. */
export function selectionMenu(songs: MenuSong[], place: SongPlace): MenuSpec {
  const target = songsTarget(songs);
  const count = songs.length;
  return {
    title: m.menu_selected({ count, countText: formatNumber(count) }),
    groups: [
      [
        action(m.menu_play_next(), ListStart, () => queueSongs(target, "next", place.label)),
        action(m.menu_add_to_queue(), ListEnd, () => queueSongs(target, "end", place.label)),
      ],
      [action(m.menu_favorite_all(), Heart, () => starSongs(songs, true)), playlistSubmenu(target)],
      [saveAction({ kind: "songs", ids: songs.map((song) => song.id) })],
      placeGroup(place),
    ],
  };
}

/** A song of a list that is not loaded yet (a long selection): its ID is enough. */
export function songById(id: string): MenuSong {
  return { id, title: "", artist: null, artistId: null, albumId: null, starred: false };
}

/** One song: its menu; several (a selection): the selection's. */
export function openSongsMenu(event: MouseEvent, songs: MenuSong[], place: SongPlace): void {
  if (songs.length === 0) return;
  openMenu(event, () => (songs.length === 1 ? songMenu(songs[0], place) : selectionMenu(songs, place)));
}

/** Entries of a playlist's songs at these positions: which occurrence of its ID each one is. */
export function playlistEntries(songs: { id: string }[], positions: number[]): PlaylistEntry[] {
  return positions.map((position) => {
    const id = songs[position].id;
    let occurrence = 0;
    for (let i = 0; i < position; i++) if (songs[i].id === id) occurrence++;
    return { id, occurrence };
  });
}

/** An album (spec 8.1). Reads the cache for its heart. */
export async function albumMenu(album: { id: string; name: string; artistId: string | null }): Promise<MenuSpec> {
  const info = await getAlbumInfo(album.id);
  const target: ItemTarget = { kind: "album", id: album.id };
  const artistId = album.artistId;
  return {
    groups: [
      [
        action(m.menu_play(), Play, () => playItem(target, "inOrder", album.name)),
        action(m.menu_play_shuffled(), Shuffle, () => playItem(target, "shuffled", album.name)),
        action(m.menu_play_next(), ListStart, () => queueSongs(target, "next", album.name)),
        action(m.menu_add_to_queue(), ListEnd, () => queueSongs(target, "end", album.name)),
      ],
      [
        info?.starred
          ? action(m.menu_unfavorite(), HeartOff, () => starAlbum(album.id, false))
          : action(m.menu_favorite(), Heart, () => starAlbum(album.id, true)),
        playlistSubmenu(target),
      ],
      artistId ? [action(m.menu_go_to_artist(), MicVocal, () => go(`/artists/${encodeURIComponent(artistId)}`))] : [],
      [
        saveAction({ kind: "album", id: album.id }),
        action(m.menu_info(), Info, () => showDialog({ kind: "info", target: { kind: "album", id: album.id } })),
        ...navidromeActions("album", album.id),
      ],
    ],
  };
}

/** An artist (spec 8.1, 8.2). Reads the cache for its heart. */
export async function artistMenu(artist: { id: string; name: string }): Promise<MenuSpec> {
  const info = await getArtistInfo(artist.id);
  const target: ItemTarget = { kind: "artist", id: artist.id };
  return {
    groups: [
      [
        action(m.menu_play(), Play, () => playItem(target, "inOrder", artist.name)),
        action(m.menu_artist_mix({ count: appSettings.mixSize }), Dices, () => playItem(target, "mix", artist.name)),
        action(m.menu_artist_radio(), Radio, () =>
          playArtistRadio(artist.id, m.menu_radio_context({ artist: artist.name })),
        ),
        action(m.menu_play_next(), ListStart, () => queueSongs(target, "next", artist.name)),
        action(m.menu_add_to_queue(), ListEnd, () => queueSongs(target, "end", artist.name)),
      ],
      [
        info?.starred
          ? action(m.menu_unfavorite(), HeartOff, () => starArtist(artist.id, false))
          : action(m.menu_favorite(), Heart, () => starArtist(artist.id, true)),
      ],
      [action(m.menu_go_to_similar_artists(), Users, () => goToSimilarArtists(artist.id))],
      [
        action(m.menu_info(), Info, () => showDialog({ kind: "info", target: { kind: "artist", id: artist.id } })),
        ...navidromeActions("artist", artist.id),
      ],
    ],
  };
}

/** A manual playlist (spec 8.1). Editing items only on the user's own. */
export function playlistMenu(playlist: PlaylistSummary): MenuSpec {
  const target: ItemTarget = { kind: "playlist", id: playlist.id };
  const own = isOwnPlaylist(playlist);
  return {
    groups: [
      [
        action(m.menu_play(), Play, () => playItem(target, "inOrder", playlist.name)),
        action(m.menu_play_shuffled(), Shuffle, () => playItem(target, "shuffled", playlist.name)),
        action(m.menu_play_next(), ListStart, () => queueSongs(target, "next", playlist.name)),
        action(m.menu_add_to_queue(), ListEnd, () => queueSongs(target, "end", playlist.name)),
      ],
      own
        ? [
            action(m.menu_rename_playlist(), Pencil, () => showDialog({ kind: "editPlaylist", playlist })),
            playlist.public
              ? action(m.menu_make_private(), Lock, () => setPlaylistPublic(playlist, false))
              : action(m.menu_make_public(), Globe, () => setPlaylistPublic(playlist, true)),
          ]
        : [],
      [saveAction({ kind: "playlist", id: playlist.id })],
      own ? [action(m.menu_delete_playlist(), Trash, () => showDialog({ kind: "deletePlaylist", playlist }))] : [],
    ],
  };
}

/** The playlist's menu by ID, with the sidebar's fresh details when known. */
export function playlistMenuById(playlist: { id: string; name: string; owner: string | null }): MenuSpec {
  const known = playlists.items.find((item) => item.id === playlist.id);
  return playlistMenu(known ?? { ...playlist, songCount: null, comment: null, public: null, changed: null });
}
