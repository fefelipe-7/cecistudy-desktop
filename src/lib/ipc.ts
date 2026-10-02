import { invoke as tauriInvoke } from "@tauri-apps/api/core";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isIpcAvailable(): boolean {
  return typeof window !== "undefined" && window.__TAURI_INTERNALS__ !== undefined;
}

export class IpcUnavailableError extends Error {
  constructor(command: string) {
    super(`O comando "${command}" exige o shell Tauri. Rode com "npm run tauri:dev".`);
    this.name = "IpcUnavailableError";
  }
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isIpcAvailable()) {
    throw new IpcUnavailableError(command);
  }
  return tauriInvoke<T>(command, args);
}
