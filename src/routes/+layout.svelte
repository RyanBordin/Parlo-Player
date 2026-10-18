<script lang="ts">
  import "#lib/styles/theme.css";
  import "#lib/styles/global.css";
  import { getLocale } from "#lib/paraglide/runtime.js";

  let { children } = $props();

  // Lets the webview pick the right spell-checking, hyphenation, etc.
  document.documentElement.lang = getLocale();
</script>

{@render children()}
