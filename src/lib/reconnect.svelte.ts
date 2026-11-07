// Back from offline: the "no connection" state of the Home comes from one
// call to the server, so while it lasts the server is asked again, on a
// backoff and whenever the network or the app comes back (spec 4.5).
import { home, refreshHome } from "#lib/home.svelte.ts";
import { startLightSync } from "#lib/sync.svelte.ts";

/** Waits between tries; the last one repeats. */
const RETRY_SECONDS = [5, 10, 30, 60];

/** Starts asking; call the returned function to stop. */
export function watchConnection(): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let attempt = 0;
  let checking = false;

  function schedule() {
    const seconds = RETRY_SECONDS[Math.min(attempt, RETRY_SECONDS.length - 1)];
    attempt += 1;
    timer = setTimeout(retry, seconds * 1000);
  }

  async function retry() {
    if (checking) return;
    clearTimeout(timer);
    checking = true;
    await refreshHome();
    checking = false;
    if (home.activity?.offline) schedule();
    else startLightSync();
  }

  /** The network or the app came back: ask now, and start the waits over. */
  function retryNow() {
    if (document.visibilityState !== "visible") return;
    attempt = 0;
    retry();
  }

  window.addEventListener("online", retryNow);
  document.addEventListener("visibilitychange", retryNow);
  schedule();

  return () => {
    clearTimeout(timer);
    window.removeEventListener("online", retryNow);
    document.removeEventListener("visibilitychange", retryNow);
  };
}
