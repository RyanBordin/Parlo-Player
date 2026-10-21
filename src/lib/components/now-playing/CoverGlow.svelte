<script lang="ts">
  // Glow behind the Now Playing cover (spec 4.4, "Cover backdrop"): the
  // current cover enlarged, heavily blurred and dimmed, fading to black at
  // the edges of its column. The text under the cover has its own shade
  // (NowPlaying), so the glow can stay bright. A new song's glow fades in over the old
  // one (600 ms; at once with reduced motion). No cover: no glow.
  //
  // Performance: the blur is set once per image and never animated. It runs
  // on a small copy that the GPU then scales up, and a switch only changes a
  // layer's opacity. The glow is contained, so what moves elsewhere on the
  // screen (progress, lyrics, queue) does not redraw it.
  import { untrack } from "svelte";
  import { coverUrl } from "#lib/covers.ts";
  import { reducedMotion } from "#lib/settings/app-settings.svelte.ts";

  /** The image asked for, in real pixels (`size=300`): it is blurred anyway. */
  const IMAGE_PIXELS = 300;

  let { coverId }: { coverId: string | null | undefined } = $props();

  type Layer = { key: number; src: string; shown: boolean };
  let layers = $state<Layer[]>([]);
  let nextKey = 0;

  // A new cover: a new layer on top, shown once its image has loaded. Only
  // the cover is watched: a layer dropped after an error is not asked again.
  $effect(() => {
    const src = coverId ? coverUrl(coverId, IMAGE_PIXELS / (window.devicePixelRatio || 1)) : null;
    untrack(() => {
      if (src === null) layers = [];
      else if (layers.at(-1)?.src !== src) layers.push({ key: nextKey++, src, shown: false });
    });
  });

  function show(layer: Layer, event: Event) {
    // The layer's hidden state is drawn first, so the fade runs even when
    // the image came from the cache at once.
    const element = (event.currentTarget as HTMLElement).parentElement;
    if (element) void getComputedStyle(element).opacity;
    layer.shown = true;
    if (reducedMotion()) settle(layer);
  }

  /** The layer is fully in: the ones under it go. */
  function settle(layer: Layer) {
    const at = layers.indexOf(layer);
    if (at > 0) layers.splice(0, at);
  }

  /** The image did not load (offline, not cached): the glow before it stays. */
  function drop(layer: Layer) {
    const at = layers.indexOf(layer);
    if (at >= 0) layers.splice(at, 1);
  }
</script>

<div class="glow" aria-hidden="true">
  <div class="images">
    {#each layers as layer (layer.key)}
      <div class="layer" class:shown={layer.shown} ontransitionend={() => settle(layer)}>
        <img src={layer.src} alt="" decoding="async" onload={(event) => show(layer, event)} onerror={() => drop(layer)} />
      </div>
    {/each}
  </div>
</div>

<style>
  /*
   * Fills the column, behind the cover and the text. Full up to 70% of the
   * way to the column's edges (the ring around the cover is what shows),
   * then transparent at the edges: black before the tabs, without a border.
   */
  .glow {
    position: absolute;
    inset: 0;
    z-index: -1;
    overflow: hidden;
    pointer-events: none;
    contain: strict;
    container-type: size;
    -webkit-mask-image: radial-gradient(closest-side, #000 70%, transparent 100%);
    mask-image: radial-gradient(closest-side, #000 70%, transparent 100%);
  }

  /*
   * 60% opacity for the whole glow, not per layer: during a switch the new
   * layer covers the old one (0 -> 100% inside), so the glow never gets
   * brighter than one cover.
   */
  .images {
    position: absolute;
    inset: 0;
    opacity: 0.6;
  }

  .layer {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    opacity: 0;
    transition: opacity 600ms ease-out;
    will-change: opacity;
  }

  .layer.shown {
    opacity: 1;
  }

  /*
   * A quarter of the final size, blurred, then scaled up 4x: the blur runs on
   * few pixels (12 px here is about 48 px on screen) and the scaling is the
   * GPU's. 30% of the column's longer side x 4 covers the whole column.
   */
  img {
    width: 30cqmax;
    height: 30cqmax;
    object-fit: cover;
    filter: blur(12px);
    transform: scale(4);
  }
</style>
