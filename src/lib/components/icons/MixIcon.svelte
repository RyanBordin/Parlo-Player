<script lang="ts">
  // "Awesome Mix" icon (spec 9): Lucide's shuffle arrows with a sparkle in
  // the free top-left corner, so it never looks like the Player Bar's
  // Shuffle mode. Same stroke as the Lucide icons around it.
  let { size = 24, class: className = "" }: { size?: number; class?: string } = $props();
</script>

<svg
  class={className}
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d="m18 14 4 4-4 4" />
  <path d="m18 2 4 4-4 4" />
  <path d="M2 18h1.4c1.3 0 2.5-.6 3.3-1.7l6.1-8.6c.7-1.1 2-1.7 3.3-1.7H22" />
  <path d="M22 18h-5.9c-1.3 0-2.6-.7-3.3-1.8l-.5-.8" />
  <path d="M2 6h1.9c1.5 0 2.9.9 3.6 2.2" />
  <!-- The sparkle, above the top-left line. -->
  <path d="M4.5 0.4 5.1 1.8 6.5 2.4 5.1 3 4.5 4.4 3.9 3 2.5 2.4 3.9 1.8Z" fill="currentColor" stroke-width="0.8" />
</svg>
