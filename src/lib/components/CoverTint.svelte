<script lang="ts">
  // The cover's predominant color behind a phone surface (spec 4.5, after
  // YouTube Music): full at the top, darker towards the bottom. Used by Now
  // Playing (the whole screen) and the Mini Player (the bar), so both show
  // the same tone and fade. A new song's color fades in (at once with
  // reduced motion); with no color yet, or none found, the previous one
  // stays. The parent decides whether to show it (the Appearance option).
  import { coverColor } from "#lib/cover-color.ts";
  import { reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  let {
    coverId,
    fixed = false,
  }: {
    coverId: string | null | undefined;
    /** Fills the screen (Now Playing) rather than its parent (the Mini Player). */
    fixed?: boolean;
  } = $props();

  let tint = $state<string | null>(null);

  $effect(() => {
    const id = coverId;
    if (!id) return;
    let current = true;
    coverColor(id).then((color) => {
      if (current && color) tint = color;
    });
    return () => {
      current = false;
    };
  });
</script>

<div class="tint" class:fixed class:instant={reducedMotion()} style:--tint={tint ?? "transparent"} aria-hidden="true"></div>

<style>
  /* Under the parent's content: the parent isolates its stacking. */
  .tint {
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    background-color: var(--tint);
    transition: background-color 600ms ease-out;
    -webkit-mask-image: linear-gradient(to bottom, #000 0%, #000 25%, rgb(0 0 0 / 45%) 100%);
    mask-image: linear-gradient(to bottom, #000 0%, #000 25%, rgb(0 0 0 / 45%) 100%);
  }

  .tint.fixed {
    position: fixed;
  }

  .tint.instant {
    transition: none;
  }
</style>
