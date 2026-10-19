// Performance test mode (`npm run tauri:perf`, development only): the app
// runs on a made-up library of 50 000 songs, apart from the real one.
import { isTestLibrary } from "#lib/api.ts";

export const testLibrary = $state({ enabled: false });

let checked = false;

/** Asks the backend once. */
export function checkTestLibrary(): void {
  if (checked) return;
  checked = true;
  isTestLibrary()
    .then((enabled) => (testLibrary.enabled = enabled))
    .catch(() => {
      // Unknown: behave as the normal app.
    });
}
