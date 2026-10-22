// What the menus need to know about the server: "Open in Navidrome" only
// shows on a Navidrome (spec 8.2, `type` of `ping`).
import { isNavidrome } from "#lib/api.ts";

export const server = $state<{ navidrome: boolean }>({ navidrome: false });

/** Asked on open and after each sync, so a server unreachable at first is asked again. */
export async function checkServer(): Promise<void> {
  if (server.navidrome) return;
  try {
    server.navidrome = await isNavidrome();
  } catch {
    // Not signed in: keep it off.
  }
}

/** Sign out: the next server may be another. */
export function clearServer(): void {
  server.navidrome = false;
}
