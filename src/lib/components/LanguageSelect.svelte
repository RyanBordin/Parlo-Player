<script lang="ts">
  import { m } from "#lib/paraglide/messages.js";
  import { getLocale, locales, setLocale, type Locale } from "#lib/paraglide/runtime.js";

  // setLocale saves the choice (localStorage) and reloads the page in the new language.
  function onchange(event: Event) {
    setLocale((event.currentTarget as HTMLSelectElement).value as Locale);
  }
</script>

<select class="language" aria-label={m.language_label()} value={getLocale()} {onchange}>
  {#each locales as locale (locale)}
    <!-- Each language is shown in its own language, e.g. "English", "Português (Brasil)". -->
    <option value={locale}>{m.language_name({}, { locale })}</option>
  {/each}
</select>

<style>
  .language {
    padding: var(--space-2) var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-surface);
    color: var(--color-text);
    font: inherit;
    font-size: var(--font-size-sm);
    cursor: pointer;
  }

  .language:hover {
    background: var(--color-surface-hover);
  }
</style>
