// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
  namespace App {
    interface PageState {
      /** Phone (spec 4.5): Now Playing is open over this page, so Back closes it. */
      nowPlaying?: boolean;
    }
  }
}

export {};
