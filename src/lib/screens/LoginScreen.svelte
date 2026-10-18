<script lang="ts">
  import { login, testConnection, type SavedSession, type ServerInfo } from "#lib/api.ts";
  import logoWordmark from "#lib/assets/logo-wordmark.png";
  import LanguageSelect from "#lib/components/LanguageSelect.svelte";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";

  let { onLogin }: { onLogin: (session: SavedSession) => void } = $props();

  let url = $state("");
  let user = $state("");
  let password = $state("");

  /** Server that answered the last successful test. "Sign in" requires it. */
  let testedServer = $state<ServerInfo | null>(null);
  let busy = $state<"testing" | "signingIn" | null>(null);
  let error = $state<ErrorMessage | null>(null);

  let isHttp = $derived(url.trim().toLowerCase().startsWith("http://"));

  // Editing any field invalidates the previous test.
  function invalidateTest() {
    testedServer = null;
    error = null;
  }

  async function runTest() {
    busy = "testing";
    error = null;
    try {
      testedServer = await testConnection({ url, user, password });
    } catch (e) {
      testedServer = null;
      error = describeError(e);
    } finally {
      busy = null;
    }
  }

  async function signIn() {
    busy = "signingIn";
    error = null;
    try {
      onLogin(await login({ url, user, password }));
    } catch (e) {
      testedServer = null;
      error = describeError(e);
    } finally {
      busy = null;
    }
  }

  // Enter key: test first, sign in once the test passed.
  function onsubmit(event: SubmitEvent) {
    event.preventDefault();
    if (testedServer) signIn();
    else runTest();
  }

  function serverLabel(info: ServerInfo): string {
    const name = [info.serverType, info.serverVersion].filter(Boolean).join(" ");
    return name || m.login_unknown_server();
  }
</script>

<div class="screen">
  <div class="language">
    <LanguageSelect />
  </div>

  <form class="card" {onsubmit}>
    <header>
      <img class="logo" src={logoWordmark} alt={m.login_logo_alt()} width="160" height="184" />
      <h1>{m.login_title()}</h1>
      <p class="muted">{m.login_subtitle()}</p>
    </header>

    <label>
      <span>{m.login_server_url()}</span>
      <input
        class="input"
        type="url"
        placeholder="https://"
        autocomplete="url"
        required
        bind:value={url}
        oninput={invalidateTest}
      />
    </label>
    {#if isHttp}
      <p class="warning">{m.login_http_warning()}</p>
    {/if}

    <label>
      <span>{m.login_username()}</span>
      <input
        class="input"
        type="text"
        autocomplete="username"
        required
        bind:value={user}
        oninput={invalidateTest}
      />
    </label>

    <label>
      <span>{m.login_password()}</span>
      <input
        class="input"
        type="password"
        autocomplete="current-password"
        required
        bind:value={password}
        oninput={invalidateTest}
      />
    </label>

    <div class="status" aria-live="polite">
      {#if error}
        <p class="error">{error.message}</p>
        {#if error.detail}
          <p class="detail">{error.detail}</p>
        {/if}
      {:else if testedServer}
        <p class="success">
          {m.login_connected({ server: serverLabel(testedServer), apiVersion: testedServer.apiVersion })}
        </p>
      {:else}
        <p class="muted">{m.login_test_first()}</p>
      {/if}
    </div>

    <div class="actions">
      <button class="button button-secondary" type="submit" disabled={busy !== null}>
        {busy === "testing" ? m.login_testing() : m.login_test_connection()}
      </button>
      <button
        class="button button-primary"
        type="button"
        disabled={!testedServer || busy !== null}
        onclick={signIn}
      >
        {busy === "signingIn" ? m.login_signing_in() : m.login_sign_in()}
      </button>
    </div>
  </form>
</div>

<style>
  .screen {
    position: relative;
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-6) var(--space-4);
  }

  .language {
    position: absolute;
    top: var(--space-4);
    right: var(--space-4);
  }

  .card {
    width: 100%;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  header {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }

  .logo {
    align-self: center;
    width: 160px;
    height: auto;
    margin-bottom: var(--space-4);
  }

  h1 {
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-medium);
  }

  .status {
    min-height: 2.5em;
    font-size: var(--font-size-sm);
  }

  .actions {
    display: flex;
    gap: var(--space-3);
  }

  .actions .button {
    flex: 1;
  }

  .muted {
    color: var(--color-text-muted);
  }

  .warning {
    margin-top: calc(-1 * var(--space-2));
    color: var(--color-warning);
    font-size: var(--font-size-xs);
  }

  .success {
    color: var(--color-success);
  }

  .error {
    color: var(--color-error);
  }

  .detail {
    margin-top: var(--space-1);
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    overflow-wrap: anywhere;
  }
</style>
