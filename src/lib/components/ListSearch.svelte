<script lang="ts">
  // Quick search field of the list tools (spec 7): filters while typing.
  import Search from "@lucide/svelte/icons/search";
  import X from "@lucide/svelte/icons/x";
  import { m } from "#lib/paraglide/messages.js";

  let {
    value = $bindable(""),
    placeholder,
  }: {
    value?: string;
    placeholder: string;
  } = $props();
</script>

<label class="search">
  <Search size={16} />
  <span class="visually-hidden">{placeholder}</span>
  <input type="search" bind:value {placeholder} autocomplete="off" spellcheck="false" />
  {#if value}
    <button type="button" class="clear" onclick={() => (value = "")} aria-label={m.tracklist_clear()} title={m.tracklist_clear()}>
      <X size={14} />
    </button>
  {/if}
</label>

<style>
  .search {
    width: 280px;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid transparent;
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text-muted);
  }

  .search:focus-within {
    border-color: var(--color-border-focus);
  }

  input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--color-text);
    font: inherit;
    font-size: 14px;
    outline: none;
  }

  input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    display: grid;
    place-items: center;
    padding: 2px;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--color-text-muted);
    cursor: pointer;
  }

  .clear:hover {
    color: var(--color-text);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
