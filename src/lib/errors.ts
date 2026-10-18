import type { CommandError } from "#lib/api.ts";
import { m } from "#lib/paraglide/messages.js";

/** Subsonic error code for a wrong username or password. */
const WRONG_CREDENTIALS = 40;

export type ErrorMessage = {
  message: string;
  /** Technical detail (not translated), shown in smaller text. */
  detail?: string;
};

/** Turns an error rejected by a Tauri command into a translated message. */
export function describeError(error: unknown): ErrorMessage {
  if (!isCommandError(error)) {
    return { message: m.error_unknown(), detail: String(error) };
  }

  switch (error.kind) {
    case "invalidUrl":
      return { message: m.error_invalid_url() };
    case "timeout":
      return { message: m.error_timeout() };
    case "httpStatus":
      return { message: m.error_http_status({ status: error.status }) };
    case "network":
      return { message: m.error_network(), detail: error.detail };
    case "invalidResponse":
      return { message: m.error_invalid_response() };
    case "api":
      if (error.code === WRONG_CREDENTIALS) {
        return { message: m.error_wrong_credentials() };
      }
      return { message: m.error_api({ code: error.code, message: error.message }) };
    case "storage":
      return { message: m.error_storage(), detail: error.detail };
  }
}

function isCommandError(error: unknown): error is CommandError {
  return typeof error === "object" && error !== null && "kind" in error;
}
