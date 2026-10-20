// Home data kept between visits: going back to the Home (or to a "See more"
// grid) draws the last data at once, so the scroll position can be
// restored, and then refreshes it (spec 6.1).
import {
  getHomeActivity,
  getHomeLibrary,
  getHomeSection,
  type HomeActivity,
  type HomeLibrary,
  type HomeSection,
  type SectionItems,
} from "#lib/api.ts";
import { describeError, type ErrorMessage } from "#lib/errors.ts";
import { m } from "#lib/paraglide/messages.js";
import { appSettings } from "#lib/settings/app-settings.svelte.ts";
import { arrange, keepVisible } from "#lib/settings/layout.ts";

/** The Home sections in their default order (spec 6.1). */
export const HOME_ROWS = [
  { id: "listen-again", label: m.home_listen_again },
  { id: "favorite-songs", label: m.home_starred_songs },
  { id: "most-played-albums", label: m.home_most_played_albums },
  { id: "favorite-albums", label: m.home_starred_albums },
  { id: "forgotten-favorites", label: m.home_forgotten_starred },
  { id: "favorite-artists", label: m.home_starred_artists },
] as const;

export type HomeRowId = (typeof HOME_ROWS)[number]["id"];

/** The sections in the user's order, each shown or not; at least one shown (spec 6.10). */
export function homeLayout() {
  return keepVisible(arrange(HOME_ROWS, appSettings.homeSections), 1);
}

/** Cards drawn at first on a grid, and added each time its end shows. */
export const GRID_BATCH = 60;

export const home = $state<{
  library: HomeLibrary | null;
  activity: HomeActivity | null;
  error: ErrorMessage | null;
}>({ library: null, activity: null, error: null });

export type SectionState = SectionItems & {
  /** Cards drawn so far (continuous scrolling). */
  shown: number;
};

export const sections = $state<Partial<Record<HomeSection, SectionState>>>({});

/** Reads the Home rows again; the local ones come first. */
export function refreshHome(): void {
  home.error = null;
  getHomeLibrary()
    .then((rows) => (home.library = rows))
    .catch((e) => (home.error = describeError(e)));
  getHomeActivity()
    .then((rows) => (home.activity = rows))
    .catch((e) => (home.error = describeError(e)));
}

/** Reads a section's grid again, keeping how far it was drawn. */
export async function refreshSection(section: HomeSection): Promise<void> {
  const data = await getHomeSection(section);
  sections[section] = { ...data, shown: sections[section]?.shown ?? GRID_BATCH };
}

/** Sign out: the next account must not see this one's data. */
export function clearHome(): void {
  home.library = null;
  home.activity = null;
  home.error = null;
  for (const key of Object.keys(sections) as HomeSection[]) delete sections[key];
}
