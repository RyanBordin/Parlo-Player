// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
  namespace App {
    interface PageState {
      /** Phone (spec 4.5): Now Playing is open over this page, so Back closes it. */
      nowPlaying?: boolean;
      /** Its drawer (Up next, Lyrics, Related) is open too: Back closes the drawer first. */
      sheet?: boolean;
      /** Phone: songs are being selected with long presses, so Back ends the selection. */
      selection?: boolean;
    }
  }
}

export {};
