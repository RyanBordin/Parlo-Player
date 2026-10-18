<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  // Mirrors `ServerInfo` in src-tauri/src/subsonic/client.rs.
  type ServerInfo = {
    apiVersion: string;
    serverType: string | null;
    serverVersion: string | null;
    openSubsonic: boolean;
  };

  // Temporary connection form (step 2). The real login screen comes in step 3.
  let url = $state("");
  let user = $state("");
  let password = $state("");
  let testing = $state(false);
  let result = $state<{ ok: boolean; message: string } | null>(null);

  async function testConnection(event: SubmitEvent) {
    event.preventDefault();
    testing = true;
    result = null;
    try {
      const info = await invoke<ServerInfo>("test_connection", { url, user, password });
      const server = [info.serverType ?? "Servidor Subsonic", info.serverVersion]
        .filter(Boolean)
        .join(" ");
      result = { ok: true, message: `Conectado: ${server} (API ${info.apiVersion})` };
    } catch (error) {
      result = { ok: false, message: String(error) };
    } finally {
      testing = false;
    }
  }
</script>

<main>
  <h1>Parlo Player</h1>

  <form onsubmit={testConnection}>
    <label>
      URL do servidor
      <input type="url" bind:value={url} placeholder="https://" required />
    </label>
    <label>
      Usuário
      <input type="text" bind:value={user} autocomplete="username" required />
    </label>
    <label>
      Senha
      <input type="password" bind:value={password} autocomplete="current-password" required />
    </label>
    <button type="submit" disabled={testing}>
      {testing ? "Testando..." : "Testar conexão"}
    </button>
  </form>

  {#if result}
    <p class:ok={result.ok} class:error={!result.ok}>{result.message}</p>
  {/if}
</main>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: #000;
    color: #fff;
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }

  main {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }

  h1 {
    font-weight: 700;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 320px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 14px;
  }

  input,
  button {
    padding: 8px;
    font: inherit;
  }

  p {
    max-width: 320px;
    text-align: center;
  }

  .ok {
    color: #4ade80;
  }

  .error {
    color: #f87171;
  }
</style>
