// Typed wrappers around the Tauri commands in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";

/** Mirrors `ServerInfo` in src-tauri/src/subsonic/client.rs. */
export type ServerInfo = {
  apiVersion: string;
  serverType: string | null;
  serverVersion: string | null;
  openSubsonic: boolean;
};

/** Mirrors `SavedSession` in src-tauri/src/session.rs. */
export type SavedSession = {
  serverUrl: string;
  username: string;
};

/** Mirrors `CommandError` in src-tauri/src/commands.rs. */
export type CommandError =
  | { kind: "invalidUrl" }
  | { kind: "timeout" }
  | { kind: "httpStatus"; status: number }
  | { kind: "network"; detail: string }
  | { kind: "invalidResponse" }
  | { kind: "api"; code: number; message: string }
  | { kind: "storage"; detail: string }
  | { kind: "notLoggedIn" }
  | { kind: "emptyLibrary"; reported: number }
  | { kind: "songNotFound" }
  | { kind: "playback"; detail: string };

/** Mirrors `LibraryStatus` in src-tauri/src/commands.rs. Dates are Unix seconds. */
export type LibraryStatus = {
  artists: number;
  albums: number;
  songs: number;
  lastFullSyncAt: number | null;
  lastLightSyncAt: number | null;
  syncing: boolean;
};

/** Mirrors `SyncProgress` in src-tauri/src/sync/mod.rs (`sync-progress` event). */
export type SyncProgress = {
  phase: "artists" | "albums" | "songs" | "saving" | "starred" | "playlists";
  done: number;
  total: number | null;
  full: boolean;
};

/** Payload of the `sync-finished` event. */
export type SyncFinished = {
  error: CommandError | null;
};

export const SYNC_PROGRESS_EVENT = "sync-progress";
export const SYNC_FINISHED_EVENT = "sync-finished";

export type Credentials = {
  url: string;
  user: string;
  password: string;
};

export function testConnection(credentials: Credentials): Promise<ServerInfo> {
  return invoke("test_connection", credentials);
}

export function getSession(): Promise<SavedSession | null> {
  return invoke("get_session");
}

export function login(credentials: Credentials): Promise<SavedSession> {
  return invoke("login", credentials);
}

export function logout(): Promise<void> {
  return invoke("logout");
}

export function getLibraryStatus(): Promise<LibraryStatus> {
  return invoke("get_library_status");
}

/**
 * Starts a sync in the background. Resolves to `false` if one is already
 * running. `forceFull` skips the rules and always runs a full sync.
 */
export function syncLibrary(forceFull: boolean): Promise<boolean> {
  return invoke("sync_library", { forceFull });
}

/** Mirrors `SongSummary` in src-tauri/src/db/library.rs. Duration in seconds. */
export type SongSummary = {
  id: string;
  title: string;
  artist: string | null;
  album: string | null;
  artistId: string | null;
  albumId: string | null;
  /** ID for the `cover` protocol (see #lib/covers.ts). */
  coverArt: string | null;
  duration: number | null;
  starred: boolean;
};

/** Mirrors `AlbumCard` in src-tauri/src/db/home.rs. */
export type AlbumCard = {
  id: string;
  name: string;
  artist: string | null;
  artistId: string | null;
  coverArt: string | null;
};

/** Mirrors `ArtistCard`: without a photo, the latest album's cover stands in. */
export type ArtistCard = {
  id: string;
  name: string;
  coverArt: string | null;
  fallbackCoverArt: string | null;
};

/** A card of a "See more" grid: album or artist. */
export type HomeItem = ({ kind: "album" } & AlbumCard) | ({ kind: "artist" } & ArtistCard);

/** The cards of a song row and every song of the list it stands for. */
export type SongRow = { songs: SongSummary[]; allIds: string[] };

export type HomeLibrary = {
  listenAgain: SongRow;
  starredSongs: SongRow;
  starredAlbums: AlbumCard[];
  forgottenStarred: SongRow;
  starredArtists: ArtistCard[];
};

export type HomeActivity = {
  mostPlayedAlbums: AlbumCard[];
  /** The local cache stood in for the server. */
  offline: boolean;
};

/** Home rows with a "See more" grid. Mirrors `HomeSection` (Rust). */
export type HomeSection = "most-played-albums" | "favorite-albums" | "favorite-artists";

/** Every item of a section, for its grid. */
export type SectionItems = { items: HomeItem[]; offline: boolean };

/** A tracklist line (album / playlist page). Mirrors `TrackListSong` in src-tauri/src/db/tracks.rs. */
export type TrackListSong = {
  id: string;
  title: string;
  artist: string | null;
  artistId: string | null;
  album: string | null;
  albumId: string | null;
  albumArtist: string | null;
  composer: string | null;
  coverArt: string | null;
  track: number | null;
  discNumber: number | null;
  year: number | null;
  genre: string | null;
  genres: string[];
  /** Seconds. */
  duration: number | null;
  suffix: string | null;
  /** kbps. */
  bitRate: number | null;
  path: string | null;
  starred: boolean;
};

export type AlbumHeader = {
  id: string;
  name: string;
  artist: string | null;
  artistId: string | null;
  coverArt: string | null;
  songCount: number | null;
  /** Seconds. */
  duration: number | null;
  year: number | null;
  genre: string | null;
  genres: string[];
  /** Date added to the server (ISO 8601). */
  created: string | null;
  starred: boolean;
};

export type AlbumPage = { album: AlbumHeader; songs: TrackListSong[] };

export type PlaylistHeader = {
  id: string;
  name: string;
  owner: string | null;
  coverArt: string | null;
  songCount: number | null;
  /** Seconds. */
  duration: number | null;
};

export type PlaylistPage = {
  playlist: PlaylistHeader;
  /** Saved order; a song can appear more than once. */
  songs: TrackListSong[];
  /** Read from the local cache: the server could not be reached. */
  offline: boolean;
};

/** An album of the Albums page. Mirrors `AlbumListItem` in src-tauri/src/db/albums.rs. */
export type AlbumListItem = AlbumCard & {
  sortName: string | null;
  year: number | null;
  genre: string | null;
  genres: string[];
  /** Date added to the server (ISO 8601). */
  created: string | null;
  starred: boolean;
  playCount: number | null;
  /** Last play (ISO 8601). */
  played: string | null;
};

/** The buttons of the Albums page (spec 6.4). */
export type AlbumsPlayMode = "inOrder" | "shuffledAlbums" | "mix";

/** Mirrors `PlaylistSummary` in src-tauri/src/db/library.rs. */
export type PlaylistSummary = {
  id: string;
  name: string;
  owner: string | null;
  songCount: number | null;
};

/** Mirrors `Track` in src-tauri/src/player/events.rs. */
export type Track = SongSummary;

/** Payload of `player-state`. Volume 0–100. */
export type PlayerState = {
  status: "idle" | "loading" | "playing" | "paused";
  track: Track | null;
  /** Kept while muted. */
  volume: number;
  muted: boolean;
};

/** Payload of `player-position`. Seconds. */
export type PlayerPosition = {
  position: number;
  duration: number | null;
};

/** Payload of `player-error`. */
export type PlayerError = { songId: string | null; detail: string };

export const PLAYER_STATE_EVENT = "player-state";
export const PLAYER_POSITION_EVENT = "player-position";
export const PLAYER_ERROR_EVENT = "player-error";
export const QUEUE_CHANGED_EVENT = "queue-changed";

/** Mirrors `RepeatMode` in src-tauri/src/player/queue.rs. */
export type RepeatMode = "off" | "all" | "one";

/** Payload of `queue-changed`. Tracks are in playback order. */
export type QueueState = {
  label: string;
  /** Set when the queue was continued from another app (spec 5.6); shown instead of `label`. */
  continuingFrom: string | null;
  tracks: Track[];
  currentIndex: number | null;
  shuffle: boolean;
  repeat: RepeatMode;
};

export function listSongs(limit: number): Promise<SongSummary[]> {
  return invoke("list_songs", { limit });
}

/** Home sections from the local cache ("Listen again", favorites). */
export function getHomeLibrary(): Promise<HomeLibrary> {
  return invoke("get_home_library");
}

/** "Most played albums" from the server, or the local cache without connection. */
export function getHomeActivity(): Promise<HomeActivity> {
  return invoke("get_home_activity");
}

/** Every item of a Home section (no 20-card limit). */
export function getHomeSection(section: HomeSection): Promise<SectionItems> {
  return invoke("get_home_section", { section });
}

/** An album and its songs from the local cache; `null` if not in the library. */
export function getAlbumPage(albumId: string): Promise<AlbumPage | null> {
  return invoke("get_album_page", { albumId });
}

/** A playlist from the server, or the local cache offline; `null` if it was deleted. */
export function getPlaylistPage(playlistId: string): Promise<PlaylistPage | null> {
  return invoke("get_playlist_page", { playlistId });
}

/** Star / unstar an album on the server and in the local cache. */
export function setAlbumStarred(albumId: string, starred: boolean): Promise<void> {
  return invoke("set_album_starred", { albumId, starred });
}

/** "Shuffle" of a list: shuffle on, starting at a random song. */
export function playShuffled(songIds: string[], contextLabel: string): Promise<void> {
  return invoke("play_shuffled", { songIds, contextLabel });
}

/** Every album of the library, last added first. */
export function listAlbums(): Promise<AlbumListItem[]> {
  return invoke("list_albums");
}

/** Plays the albums shown: in order, in a random album order, or as a mix. */
export function playAlbums(albumIds: string[], mode: AlbumsPlayMode, contextLabel: string): Promise<void> {
  return invoke("play_albums", { albumIds, mode, contextLabel });
}

/** Plays a whole album in disc and track order. */
export function playAlbum(albumId: string): Promise<void> {
  return invoke("play_album", { albumId });
}

/** Playlists from the local cache, by name. */
export function listPlaylists(): Promise<PlaylistSummary[]> {
  return invoke("list_playlists");
}

/**
 * Replaces the queue with a whole context, in the order shown, and plays
 * from `startIndex` (spec 5.1). `contextLabel` is the "Playing from" text.
 */
export function playContext(songIds: string[], startIndex: number, contextLabel: string): Promise<void> {
  return invoke("play_context", { songIds, startIndex, contextLabel });
}

/**
 * Resumes the saved queue (local or from the server), paused at the saved
 * position (spec 5.6). Only the first call per library does something.
 */
export function restoreQueue(): Promise<void> {
  return invoke("restore_queue");
}

export function next(): Promise<void> {
  return invoke("next");
}

export function previous(): Promise<void> {
  return invoke("previous");
}

/** Click on a song of the queue list: only jumps to it. */
export function playQueueIndex(index: number): Promise<void> {
  return invoke("play_queue_index", { index });
}

export function setShuffle(enabled: boolean): Promise<void> {
  return invoke("set_shuffle", { enabled });
}

/** Off → repeat queue → repeat one. Resolves to the new mode. */
export function cycleRepeat(): Promise<RepeatMode> {
  return invoke("cycle_repeat");
}

export function getQueue(): Promise<QueueState> {
  return invoke("get_queue");
}

export function togglePause(): Promise<void> {
  return invoke("toggle_pause");
}

export function seek(seconds: number): Promise<void> {
  return invoke("seek", { seconds });
}

/** Jumps `seconds` forward (negative: back). */
export function seekBy(seconds: number): Promise<void> {
  return invoke("seek_by", { seconds });
}

/** Mutes or unmutes, keeping the volume. Resolves to the new state. */
export function toggleMute(): Promise<boolean> {
  return invoke("toggle_mute");
}

/** Star / unstar a song on the server and in the local cache. */
export function setSongStarred(songId: string, starred: boolean): Promise<void> {
  return invoke("set_song_starred", { songId, starred });
}

/** Resolves to the volume actually applied (0–100). */
export function setVolume(volume: number): Promise<number> {
  return invoke("set_volume", { volume });
}

export function getPlayerState(): Promise<PlayerState> {
  return invoke("get_player_state");
}
