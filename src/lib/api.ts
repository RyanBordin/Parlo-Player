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
  | { kind: "playback"; detail: string }
  | { kind: "testLibrary" };

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

/** Starts only the light sync (starred items, playlists). `false` if a sync is running. */
export function syncLight(): Promise<boolean> {
  return invoke("sync_light");
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
  playCount: number | null;
};

/** A card of the Favorites "Recent" row. Mirrors `FavoriteItem` in src-tauri/src/db/favorites.rs. */
export type FavoriteItem =
  | ({ kind: "album" } & AlbumCard)
  | ({ kind: "artist" } & ArtistCard)
  | ({ kind: "song" } & TrackListSong);

/** The Favorites page (spec 6.2): every list is last starred first. */
export type Favorites = {
  /** The last 20 items starred, of any kind. */
  recent: FavoriteItem[];
  albums: AlbumCard[];
  artists: ArtistCard[];
  songs: TrackListSong[];
};

/** Columns the Songs page sorts by. Mirrors `SongSortKey` in src-tauri/src/db/songs.rs. */
export type SongSortKey =
  | "added"
  | "title"
  | "artist"
  | "album"
  | "duration"
  | "year"
  | "genre"
  | "bitRate"
  | "playCount"
  | "played"
  | "random";
/** `seed`: the draw of "random" (ignored by the other keys). */
export type SongSort = { key: SongSortKey; direction: "asc" | "desc"; seed?: number };

/** The Songs page: IDs in the order shown, and the size of the library. */
export type SongList = { ids: string[]; total: number };

/** Mirrors `AlbumResult`: an album found by the global search. */
export type AlbumResult = AlbumCard & { starred: boolean; songCount: number | null; year: number | null };

/** Mirrors `ArtistResult`: an artist found by the global search. */
export type ArtistResult = ArtistCard & { starred: boolean; albumCount: number };

export type ResultKind = "song" | "album" | "artist";

/** Mirrors `SearchResults` (spec 6.9): every list best first. */
export type SearchResults = {
  /** Every song found: the queue of a click. */
  songIds: string[];
  /** The first 4 of them, for the "Top" tab. */
  topSongs: TrackListSong[];
  albums: AlbumResult[];
  artists: ArtistResult[];
  /** The list whose first item is the best result. */
  best: ResultKind | null;
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

/** An artist of the Artists page. Mirrors `ArtistListItem` in src-tauri/src/db/artists.rs. */
export type ArtistListItem = ArtistCard & {
  sortName: string | null;
  starred: boolean;
  albumCount: number;
  /** Plays of the artist's songs. */
  playCount: number;
  /** When the artist's latest song was added (ISO 8601). */
  added: string | null;
  /** Album artist (the default filter); otherwise only featured on songs. */
  albumArtist: boolean;
};

/** Mirrors `ArtistHeader` in src-tauri/src/db/artist_page.rs. */
export type ArtistHeader = ArtistCard & { starred: boolean };

/** The artist page from the local cache (spec 6.7). Mirrors `ArtistPage`. */
export type ArtistPage = {
  artist: ArtistHeader;
  /** Every song the artist appears on, most played first. */
  songs: TrackListSong[];
  /** Newest first; singles and EPs apart when the server gives release types. */
  albums: AlbumCard[];
  singlesAndEps: AlbumCard[];
  /** Albums of other artists with songs of this one. */
  appearsOn: AlbumCard[];
};

/** The artist page's parts from the server. Mirrors `ArtistAbout` in src-tauri/src/artist_page.rs. */
export type ArtistAbout = {
  /** Plain text (HTML removed by the backend). */
  biography: string | null;
  /** Similar artists that are in the library. */
  similar: ArtistCard[];
  /** The photo is real (not the server's generic image); `null`: unknown. */
  hasPhoto: boolean | null;
  /** Every song in the `getTopSongs` order, while the artist has no plays. */
  songOrder: string[] | null;
};

/** A Home section or sidebar item, in the order the user left them. */
export type LayoutItem = { id: string; visible: boolean };

/** App preferences (spec 6.10), in settings.json. Mirrors `AppSettings` in src-tauri/src/settings.rs. */
export type AppSettings = {
  language: "system" | "pt-BR" | "en";
  /** Empty: the default order, all shown. */
  homeSections: LayoutItem[];
  /** Settings excluded (always last). Empty: the default order, all shown. */
  sidebarItems: LayoutItem[];
  density: "comfortable" | "compact";
  /** Percent: 90, 100, 110 or 125. */
  scale: number;
  reduceMotion: "system" | "on" | "off";
  /** Auto: album gain when the queue is one album in order, track gain otherwise. */
  replayGain: "off" | "track" | "album" | "auto";
  /** dB, −15 to +15. */
  replayGainPreamp: number;
  avoidClipping: boolean;
  /** mpv's device name, or "auto" (the system's default). */
  audioDevice: string;
  /** Windows only. */
  exclusiveAudio: boolean;
  /** 50, 100, 200 or 500. */
  mixSize: number;
  stream: StreamQuality;
  /** Cover cache limit, MB: 250, 500, 1024 or 2048. */
  coverCacheMb: number;
  /** Now Playing: a soft glow of the cover behind it (spec 4.4). */
  nowPlayingBackdrop: boolean;
};

/** Streaming quality (spec 6.10). Mirrors `StreamQuality` in src-tauri/src/settings.rs. */
export type StreamQuality = {
  mode: "original" | "converted";
  format: "opus" | "mp3";
  /** kbps: 320, 256, 192 or 128. */
  maxBitRate: number;
};

/** Preferences of the signed-in server. Mirrors `LibrarySettings` in src-tauri/src/db/settings.rs. */
export type LibrarySettings = {
  /** Send plays to the server; off, the local history is still kept. */
  scrobble: boolean;
  /** Minutes between automatic syncs: 15, 30, 60, or 0 (manual only). */
  syncIntervalMinutes: number;
  /** Resume a queue another app saved on the server later (spec 5.6). */
  continueOtherDevice: boolean;
  /** Music folder IDs left out of the library. */
  hiddenFolders: string[];
};

/** Mirrors `ServerInfo` + extensions: Settings > Server and account. */
export type ServerStatus = {
  online: boolean;
  /** Why the server could not be reached. */
  error: CommandError | null;
  server: ServerInfo | null;
  extensions: { name: string; versions: number[] }[];
};

/** Mirrors `SyncStatus` in src-tauri/src/commands.rs. */
export type SyncStatus = {
  /** Unix seconds. */
  lastSyncAt: number | null;
  /** The error of the last sync, if it failed. */
  lastError: CommandError | null;
  pendingScrobbles: number;
  syncing: boolean;
};

/** Mirrors `LibrarySummary` in src-tauri/src/commands.rs. */
export type LibrarySummary = {
  songs: number;
  albums: number;
  artists: number;
  databaseBytes: number;
  coverCacheBytes: number;
};

/** A music folder of the server (a library on Navidrome). */
export type MusicFolder = { id: string; name: string };

/** Links of Settings > About; the backend knows the addresses. */
export type ExternalLink = "repository" | "email";

/** An audio output, as mpv lists it. "auto" is the system's default. */
export type AudioDevice = { name: string; description: string };

/** Payload: `AudioDevice[]`, when a device is plugged in or removed. */
export const AUDIO_DEVICES_EVENT = "audio-devices";

/** The buttons of the Albums page (spec 6.4). */
export type AlbumsPlayMode = "inOrder" | "shuffledAlbums" | "mix";

/** Lyrics of a song (Now Playing). Mirrors `Lyrics` in src-tauri/src/now_playing.rs. */
export type Lyrics =
  | {
      kind: "synced";
      lines: { startMs: number; text: string }[];
      /** Milliseconds; positive means the lines show sooner. */
      offsetMs: number;
    }
  | { kind: "plain"; text: string }
  | { kind: "none" };

/** The Related tab's sections, in order. */
export type Related = {
  similar: SongRow;
  artist: string | null;
  artistSongs: SongRow;
  artistAlbums: AlbumCard[];
  genre: string | null;
  genreAlbums: AlbumCard[];
};

/** Mirrors `PlaylistSummary` in src-tauri/src/db/library.rs. */
export type PlaylistSummary = {
  id: string;
  name: string;
  owner: string | null;
  songCount: number | null;
  /** Description. */
  comment: string | null;
  public: boolean | null;
  /** Last change on the server (ISO 8601). */
  changed: string | null;
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

/** Sent instead of `queue-changed` when only the current song or the modes changed. */
export const QUEUE_POSITION_EVENT = "queue-position";

/** Payload of `queue-position`: `QueueState` without the songs. */
export type QueuePosition = Omit<QueueState, "tracks">;

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

/** Everything starred, from the local cache. */
export function getFavorites(): Promise<Favorites> {
  return invoke("get_favorites");
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

/** The app preferences, or the defaults. */
export function getAppSettings(): Promise<AppSettings> {
  return invoke("get_app_settings");
}

/** Saves the app preferences; the backend applies the interface scale. Resolves to what was saved. */
export function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke("save_app_settings", { settings });
}

/** The audio outputs, the system's default first. */
export function getAudioDevices(): Promise<AudioDevice[]> {
  return invoke("get_audio_devices");
}

export function getLibrarySettings(): Promise<LibrarySettings> {
  return invoke("get_library_settings");
}

export function saveLibrarySettings(settings: LibrarySettings): Promise<void> {
  return invoke("save_library_settings", { settings });
}

/** "Full sync": rebuilds the library from scratch, keeping the play history. `false` if a sync is running. */
export function rebuildLibrary(): Promise<boolean> {
  return invoke("rebuild_library");
}

/** Pings the server and lists its OpenSubsonic extensions. */
export function getServerStatus(): Promise<ServerStatus> {
  return invoke("get_server_status");
}

export function getSyncStatus(): Promise<SyncStatus> {
  return invoke("get_sync_status");
}

/** Sends every pending play. Resolves to how many were sent. */
export function resendScrobbles(): Promise<number> {
  return invoke("resend_scrobbles");
}

export function getLibrarySummary(): Promise<LibrarySummary> {
  return invoke("get_library_summary");
}

export function getMusicFolders(): Promise<MusicFolder[]> {
  return invoke("get_music_folders");
}

export function clearCoverCache(): Promise<void> {
  return invoke("clear_cover_cache");
}

export function openLink(link: ExternalLink): Promise<void> {
  return invoke("open_link", { link });
}

export function openLogsFolder(): Promise<void> {
  return invoke("open_logs_folder");
}

/** Whether this build is the performance test mode (`npm run tauri:perf`). */
export function isTestLibrary(): Promise<boolean> {
  return invoke("is_test_library");
}

/** Songs that match every word of `query` (all metadata), in `sort` order. */
export function listSongIds(query: string, sort: SongSort): Promise<SongList> {
  return invoke("list_song_ids", { query, sort });
}

/** The lines of these songs, in the same order. */
export function getSongRows(ids: string[]): Promise<TrackListSong[]> {
  return invoke("get_song_rows", { ids });
}

/** Global search of the Topbar (spec 6.9). */
export function search(query: string): Promise<SearchResults> {
  return invoke("search", { query });
}

/** The last searches, newest first. */
export function getRecentSearches(): Promise<string[]> {
  return invoke("get_recent_searches");
}

/** Puts a search first in the recent list; resolves to the new list. */
export function addRecentSearch(query: string): Promise<string[]> {
  return invoke("add_recent_search", { query });
}

/** Removes a search from the recent list; resolves to the new list. */
export function removeRecentSearch(query: string): Promise<string[]> {
  return invoke("remove_recent_search", { query });
}

/** At most 100 of these songs, in a random order (spec 9). */
export function playMix(songIds: string[], contextLabel: string): Promise<void> {
  return invoke("play_mix", { songIds, contextLabel });
}

/** Every album of the library, last added first. */
export function listAlbums(): Promise<AlbumListItem[]> {
  return invoke("list_albums");
}

/** Plays the albums shown: in order, in a random album order, or as a mix. */
export function playAlbums(albumIds: string[], mode: AlbumsPlayMode, contextLabel: string): Promise<void> {
  return invoke("play_albums", { albumIds, mode, contextLabel });
}

/** Album artists and featured artists, by name. */
export function listArtists(): Promise<ArtistListItem[]> {
  return invoke("list_artists");
}

/** At most 100 songs of these artists, in a random order (spec 6.6). */
export function playArtistsMix(artistIds: string[], contextLabel: string): Promise<void> {
  return invoke("play_artists_mix", { artistIds, contextLabel });
}

/** An artist page from the local cache; `null` if not in the library. */
export function getArtistPage(artistId: string): Promise<ArtistPage | null> {
  return invoke("get_artist_page", { artistId });
}

/** Biography, similar artists and photo (kept 7 days); `null` if not in the library. */
export function getArtistAbout(artistId: string): Promise<ArtistAbout | null> {
  return invoke("get_artist_about", { artistId });
}

/** Star / unstar an artist on the server and in the local cache. */
export function setArtistStarred(artistId: string, starred: boolean): Promise<void> {
  return invoke("set_artist_starred", { artistId, starred });
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

/** Lyrics of a song: synced, plain or none. */
export function getLyrics(songId: string): Promise<Lyrics> {
  return invoke("get_lyrics", { songId });
}

/** The Related tab around a song; `null` if it is not in the library. */
export function getRelated(songId: string): Promise<Related | null> {
  return invoke("get_related", { songId });
}

/** Up Next: drags a song to another position of the queue. */
export function moveInQueue(from: number, to: number): Promise<void> {
  return invoke("move_in_queue", { from, to });
}

/** Up Next: removes songs (one, or a selection) from the queue. */
export function removeFromQueue(positions: number[]): Promise<void> {
  return invoke("remove_from_queue", { positions });
}

/**
 * Saves the item's songs as a new playlist on the server. Resolves to its ID,
 * or `null` when the server did not send it back: it shows in the sidebar
 * after a sync.
 */
export function createPlaylist(name: string, target: ItemTarget): Promise<string | null> {
  return invoke("create_playlist", { name, target });
}

/** What a "..." menu acts on. Mirrors `ItemTarget` in src-tauri/src/commands.rs. */
export type ItemTarget =
  | { kind: "songs"; ids: string[] }
  | { kind: "album"; id: string }
  | { kind: "artist"; id: string }
  | { kind: "playlist"; id: string };

/** "Play", "Play shuffled" and "Random mix" of a menu. */
export type ItemPlayMode = "inOrder" | "shuffled" | "mix";

/** "Play next" or "Add to queue". */
export type Placement = "next" | "end";

/** A playlist song as shown: its ID and which of its occurrences it is (0 = first). */
export type PlaylistEntry = { id: string; occurrence: number };

/** The choice when some songs are already in the playlist (spec 8.2). */
export type Duplicates = "addAll" | "onlyNew";

/** Answer of `addToPlaylist`. */
export type AddOutcome = { kind: "duplicates"; count: number; total: number } | { kind: "added"; count: number };

export type ArtistRef = { id: string; name: string };

/** "File information" of a song. Mirrors `SongInfo` in src-tauri/src/db/items.rs. */
export type SongInfo = {
  title: string;
  artist: string | null;
  album: string | null;
  suffix: string | null;
  contentType: string | null;
  /** kbps. */
  bitRate: number | null;
  /** Hz. */
  samplingRate: number | null;
  bitDepth: number | null;
  channelCount: number | null;
  /** Bytes. */
  size: number | null;
  /** Seconds. */
  duration: number | null;
  path: string | null;
  albumId: string | null;
  starred: boolean;
  /** Artists in the library, in the song's order. */
  artists: ArtistRef[];
};

/** Mirrors `AlbumInfo` in src-tauri/src/db/items.rs. */
export type AlbumInfo = {
  name: string;
  artist: string | null;
  year: number | null;
  labels: string[];
  genres: string[];
  formats: string[];
  songCount: number;
  /** Seconds. */
  duration: number | null;
  starred: boolean;
};

/** Mirrors `ArtistInfo` in src-tauri/src/db/items.rs. */
export type ArtistInfo = {
  name: string;
  biography: string | null;
  albumCount: number;
  songCount: number;
  starred: boolean;
};

/** Plays an album, artist or playlist from a menu. */
export function playItem(target: ItemTarget, mode: ItemPlayMode, contextLabel: string): Promise<void> {
  return invoke("play_item", { target, mode, contextLabel });
}

/** "Play next" / "Add to queue"; with nothing queued, the songs start playing. */
export function enqueue(target: ItemTarget, placement: Placement, contextLabel: string): Promise<void> {
  return invoke("enqueue", { target, placement, contextLabel });
}

/** "Play similar songs" of a song, with a local fallback. */
export function playSimilarSongs(songId: string, contextLabel: string): Promise<void> {
  return invoke("play_similar_songs", { songId, contextLabel });
}

/** "Artist radio", with a local fallback. */
export function playArtistRadio(artistId: string, contextLabel: string): Promise<void> {
  return invoke("play_artist_radio", { artistId, contextLabel });
}

/** Without `duplicates`, nothing is added when some songs are already there: the answer says how many. */
export function addToPlaylist(playlistId: string, target: ItemTarget, duplicates: Duplicates | null): Promise<AddOutcome> {
  return invoke("add_to_playlist", { playlistId, target, duplicates });
}

/** Changes a playlist's name, description or visibility; `null` keeps a field. */
export function updatePlaylist(
  playlistId: string,
  changes: { name?: string; comment?: string; public?: boolean },
): Promise<void> {
  return invoke("update_playlist", {
    playlistId,
    name: changes.name ?? null,
    comment: changes.comment ?? null,
    public: changes.public ?? null,
  });
}

export function removeFromPlaylist(playlistId: string, entries: PlaylistEntry[]): Promise<void> {
  return invoke("remove_from_playlist", { playlistId, entries });
}

export function deletePlaylist(playlistId: string): Promise<void> {
  return invoke("delete_playlist", { playlistId });
}

/** `null` if the song is not in the library. */
export function getSongInfo(songId: string): Promise<SongInfo | null> {
  return invoke("get_song_info", { songId });
}

export function getAlbumInfo(albumId: string): Promise<AlbumInfo | null> {
  return invoke("get_album_info", { albumId });
}

export function getArtistInfo(artistId: string): Promise<ArtistInfo | null> {
  return invoke("get_artist_info", { artistId });
}

/** "Favorite all" of a selection: one request. */
export function setSongsStarred(songIds: string[], starred: boolean): Promise<void> {
  return invoke("set_songs_starred", { songIds, starred });
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

/** What "Save to computer..." saves. Mirrors `SaveTarget` in src-tauri/src/commands.rs. */
export type SaveTarget = { kind: "songs"; ids: string[] } | { kind: "album"; id: string } | { kind: "playlist"; id: string };

/** Payload of `save-progress`: songs done (saved, skipped or failed) of all. */
export type SaveProgress = { done: number; total: number };

/** Payload of `save-finished`. Mirrors `SaveFinished` in src-tauri/src/downloads/mod.rs. */
export type SaveFinished = {
  saved: number;
  /** Already in the folder with the same size. */
  skipped: number;
  /** File names that could not be saved. */
  failed: string[];
  cancelled: boolean;
  /** The server refused: no download permission. */
  forbidden: boolean;
};

export const SAVE_PROGRESS_EVENT = "save-progress";
export const SAVE_FINISHED_EVENT = "save-finished";

/**
 * Asks where with the system's dialog, then saves the original files in the
 * background. Resolves to the number of songs, or `null` if the dialog was closed.
 */
export function saveToComputer(target: SaveTarget): Promise<number | null> {
  return invoke("save_to_computer", { target });
}

export function cancelSaves(): Promise<void> {
  return invoke("cancel_saves");
}

/** Opens the folder of the last save that finished. */
export function openSaveFolder(): Promise<void> {
  return invoke("open_save_folder");
}

/** Whether the server is a Navidrome (`false` while it cannot be reached). */
export function isNavidrome(): Promise<boolean> {
  return invoke("is_navidrome");
}

/** The item's page in Navidrome's web interface, in the default browser. A song opens its album. */
export function openInNavidrome(kind: "song" | "album" | "artist", id: string): Promise<void> {
  return invoke("open_in_navidrome", { kind, id });
}
