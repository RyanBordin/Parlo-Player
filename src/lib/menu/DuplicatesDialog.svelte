<script lang="ts">
  // Some songs are already in the playlist (spec 8.2). One song: "Add
  // anyway" or "Skip"; several: "Add only the new ones" or "Add all". Esc
  // adds nothing.
  import type { Duplicates } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";

  let {
    count,
    total,
    playlist,
    onchoose,
    onclose,
  }: {
    count: number;
    total: number;
    playlist: string;
    /** `null`: nothing is added. */
    onchoose: (choice: Duplicates | null) => void;
    onclose: () => void;
  } = $props();

  const id = $props.id();
  let dialog = $state<HTMLDialogElement>();
  let chosen: Duplicates | null = null;

  $effect(() => {
    dialog?.showModal();
  });

  function choose(choice: Duplicates | null) {
    chosen = choice;
    dialog?.close();
  }

  function closed() {
    onchoose(chosen);
    onclose();
  }
</script>

<dialog bind:this={dialog} onclose={closed} aria-labelledby={`${id}-title`} aria-describedby={`${id}-text`}>
  <h2 id={`${id}-title`}>{m.duplicates_title()}</h2>
  <p id={`${id}-text`} class="muted">
    {total === 1 ? m.duplicates_one({ name: playlist }) : m.duplicates_many({ count, total, name: playlist })}
  </p>
  <div class="buttons">
    {#if total === 1}
      <button type="button" class="button button-secondary" onclick={() => choose(null)}>{m.duplicates_skip()}</button>
      <button type="button" class="button button-primary" onclick={() => choose("addAll")}>
        {m.duplicates_add_anyway()}
      </button>
    {:else}
      <button type="button" class="button button-secondary" onclick={() => choose("addAll")}>
        {m.duplicates_add_all()}
      </button>
      <button type="button" class="button button-primary" onclick={() => choose("onlyNew")}>
        {m.duplicates_only_new()}
      </button>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(440px, calc(100vw - 48px));
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--color-backdrop);
  }

  h2 {
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  .muted {
    margin-top: var(--space-2);
    color: var(--color-text-muted);
    font-size: 14px;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }
</style>
