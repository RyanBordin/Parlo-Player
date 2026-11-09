<script lang="ts">
  // Phone Topbar (spec 4.5): on the tab pages the Parlo logo, like YouTube
  // Music (the tabs below already name the page); a back arrow on the inner
  // pages (album, artist, playlist...); Profile on the right. On the Search
  // tab the search field takes the logo's place (spec 4.5, 6.9).
  import ArrowLeft from "@lucide/svelte/icons/arrow-left";
  import { goto } from "$app/navigation";
  import symbol from "#lib/assets/logo-symbol.png";
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
  {:else if inner}
    <span class="spacer"></span>
  {:else}
    <!-- The page's name stays for screen readers. -->
    <h1 class="visually-hidden">{title}</h1>
    <div class="brand" aria-hidden="true">
      <img src={symbol} alt="" width="13" height="32" />
      <span>{m.app_name()}</span>
    </div>
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

  .spacer {
    flex: 1;
  }

  .brand {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    /* In capitals, a third bigger than the large text, so it stands as tall
       as the symbol beside it (Ryan's choice); the symbol keeps its size. */
    font-size: 24px;
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .brand img {
    flex: none;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
