<script lang="ts">
  // Language of the sign-in screen: the same preference as in Settings > General.
  import type { AppSettings } from "#lib/api.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { locales } from "#lib/paraglide/runtime.js";
  import { appSettings, setLanguage } from "#lib/settings/app-settings.svelte.ts";

  // Saved in the preferences; the page reloads in the new language.
  function onchange(event: Event) {
    setLanguage((event.currentTarget as HTMLSelectElement).value as AppSettings["language"]);
  }
</script>

<select class="language" aria-label={m.language_label()} value={appSettings.language} {onchange}>
  <option value="system">{m.settings_follow_system()}</option>
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
