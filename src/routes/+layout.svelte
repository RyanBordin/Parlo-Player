<script lang="ts">
  import "@fontsource-variable/inter";
  import "#lib/styles/theme.css";
  import "#lib/styles/global.css";
  import { onMount } from "svelte";
  import { setLucideProps } from "@lucide/svelte/context";
  import { getLocale } from "#lib/paraglide/runtime.js";
  import FirstSyncScreen from "#lib/screens/FirstSyncScreen.svelte";
  import LoginScreen from "#lib/screens/LoginScreen.svelte";
  import { app, finishFirstSync, openLibrary, start } from "#lib/session.svelte.ts";
  import AppShell from "#lib/shell/AppShell.svelte";

  let { children } = $props();

  // Lets the webview pick the right spell-checking, hyphenation, etc.
  document.documentElement.lang = getLocale();

  // Every icon: same size and the same thin stroke.
  setLucideProps({ size: 18, strokeWidth: 1.75 });

  onMount(start);
</script>

<!-- Sign in and first sync have no route: the pages need a library. -->
{#if app.view.name === "connected"}
  <AppShell session={app.view.session}>
    {@render children()}
  </AppShell>
{:else if app.view.name === "firstSync"}
  <FirstSyncScreen onDone={finishFirstSync} />
{:else if app.view.name === "login"}
  <LoginScreen onLogin={openLibrary} />
{/if}
