<script lang="ts">
  // Settings > Server and account (spec 6.10): address (with the warning of
  // spec 12.5 when it is not HTTPS), status, version and OpenSubsonic
  // extensions, asked when the sector opens and by "Test connection"; the
  // user and "Sign out".
  import { getServerStatus, type ServerStatus } from "#lib/api.ts";
  import { describeError } from "#lib/errors.ts";
  import { m } from "#lib/paraglide/messages.js";
  import { app, signOut } from "#lib/session.svelte.ts";
  import ActionSetting from "#lib/settings/ActionSetting.svelte";
  import InfoSetting from "#lib/settings/InfoSetting.svelte";
  import SettingRow from "#lib/settings/SettingRow.svelte";
  import SettingsGroup from "#lib/settings/SettingsGroup.svelte";

  let session = $derived(app.view.name === "connected" ? app.view.session : null);
  let status = $state<ServerStatus | null>(null);
  let testing = $state(false);

  async function test() {
    testing = true;
    try {
      status = await getServerStatus();
    } catch (e) {
      status = { online: false, error: null, server: null, extensions: [] };
      console.error("settings: could not test the server", e);
    } finally {
      testing = false;
    }
  }

  $effect(() => {
    test();
  });

  let insecure = $derived(session?.serverUrl.toLowerCase().startsWith("http://") ?? false);
  let version = $derived.by(() => {
    const server = status?.server;
    if (!server) return null;
    const name = [server.serverType, server.serverVersion].filter(Boolean).join(" ");
    return m.settings_server_version({ server: name || m.settings_server_unknown(), api: server.apiVersion });
  });
</script>

<SettingsGroup title={m.settings_group_server()} options={["server-address", "server-status", "server-extensions"]}>
  <SettingRow option="server-address">
    <InfoSetting value={session?.serverUrl ?? ""}>
      {#if insecure}<span class="warning">{m.settings_server_http_warning()}</span>{/if}
    </InfoSetting>
  </SettingRow>
  <SettingRow option="server-status">
    <div class="status">
      {#if status}
        <InfoSetting
          value={testing ? m.settings_server_testing() : status.online ? m.settings_server_online() : m.settings_server_offline()}
          tone={testing ? "normal" : status.online ? "ok" : "error"}
        >
          {#if version}{version}{:else if status.error}{describeError(status.error).message}{/if}
        </InfoSetting>
      {:else}
        <InfoSetting value={m.settings_server_testing()} />
      {/if}
      <ActionSetting label={m.settings_server_test()} disabled={testing} onclick={test} />
    </div>
  </SettingRow>
  <SettingRow option="server-extensions" stacked>
    {#if status?.extensions.length}
      <ul class="extensions">
        {#each status.extensions as extension (extension.name)}
          <li title={m.settings_server_extension_versions({ versions: extension.versions.join(", ") })}>{extension.name}</li>
        {/each}
      </ul>
    {:else if status?.online}
      <p class="muted">{m.settings_server_no_extensions()}</p>
    {:else}
      <p class="muted">{m.settings_server_extensions_unknown()}</p>
    {/if}
  </SettingRow>
</SettingsGroup>

<SettingsGroup title={m.settings_group_account()} options={["account"]}>
  <SettingRow option="account">
    <div class="status">
      <InfoSetting value={session?.username ?? ""} />
      <ActionSetting label={m.home_sign_out()} onclick={() => signOut()} />
    </div>
  </SettingRow>
</SettingsGroup>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }

  .warning {
    color: var(--color-warning);
  }

  .extensions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .extensions li {
    padding: 2px var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-pill);
    background: var(--color-bg);
    font-size: var(--font-size-sm);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }
</style>
