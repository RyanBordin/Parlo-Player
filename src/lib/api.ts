// Typed wrappers around the Tauri commands in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";

/** Mirrors `ServerInfo` in src-tauri/src/subsonic/client.rs. */
export type ServerInfo = {
  apiVersion: string;
  serverType: string | null;
  serverVersion: string | null;
  openSubsonic: boolean;
};

/** Mirrors `SavedSession` in src-tauri/src/session.rs. */
export type SavedSession = {
  serverUrl: string;
  username: string;
};

/** Mirrors `CommandError` in src-tauri/src/commands.rs. */
export type CommandError =
  | { kind: "invalidUrl" }
  | { kind: "timeout" }
  | { kind: "httpStatus"; status: number }
  | { kind: "network"; detail: string }
  | { kind: "invalidResponse" }
  | { kind: "api"; code: number; message: string }
  | { kind: "storage"; detail: string };

export type Credentials = {
  url: string;
  user: string;
  password: string;
};

export function testConnection(credentials: Credentials): Promise<ServerInfo> {
  return invoke("test_connection", credentials);
}

export function getSession(): Promise<SavedSession | null> {
  return invoke("get_session");
}

export function login(credentials: Credentials): Promise<SavedSession> {
  return invoke("login", credentials);
}

export function logout(): Promise<void> {
  return invoke("logout");
}
