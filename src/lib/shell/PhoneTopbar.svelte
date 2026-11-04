<script lang="ts">
  // Phone Topbar (spec 4.5): the page title, a back arrow on the inner pages
  // (album, artist, playlist...) and Profile on the right. On the Search tab
  // the search field takes the title's place (spec 4.5, 6.9).
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import type { SavedSession } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { canGoBack } from "#lib/shell/history.svelte.ts";
  import { activeTab, isTabPage, PHONE_TABS } from "#lib/shell/phone.svelte.ts";
  import ProfileMenu from "#lib/shell/ProfileMenu.svelte";
  import SearchField from "#lib/shell/SearchField.svelte";

  let { session }: { session: SavedSession } = $props();

  let pathname = $derived(page.url.pathname);
  let inner = $derived(!isTabPage(pathname));
  // Inner pages show their name large at the top: only the arrow here.
  let title = $derived(inner ? "" : (PHONE_TABS.find((tab) => tab.id === activeTab(pathname))?.label() ?? ""));

  function back() {
    if (canGoBack()) history.back();
    else goto("/");
  }
</script>

<header class="topbar">
  {#if inner}
    <button class="back" type="button" onclick={back} aria-label={m.topbar_back()} title={m.topbar_back()}>
      <ArrowLeft />
    </button>
  {/if}

  {#if pathname === "/search"}
    <SearchField phone />
  {:else}
    <h1 class="title">{title}</h1>
  {/if}

  <ProfileMenu {session} />
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 60px;
    padding: var(--space-3) var(--page-gutter);
  }

  .back {
    flex: none;
    width: 40px;
    height: 40px;
    margin-left: calc(-1 * var(--space-2));
    display: grid;
    place-items: center;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text);
    -webkit-tap-highlight-color: transparent;
  }

  .title {
    flex: 1;
    min-width: 0;
    margin: 0;
    overflow: hidden;
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    white-space: nowrap;
    text-overflow: ellipsis;
  }
</style>
