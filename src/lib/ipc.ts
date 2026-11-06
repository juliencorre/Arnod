/**
 * Wrapper typé autour d'`invoke` : seul point d'accès aux commandes Rust.
 *
 * Les composants et stores n'importent jamais `@tauri-apps/api/core`
 * directement ; ils passent par `ipc`.
 */
import { invoke, type InvokeArgs } from "@tauri-apps/api/core";

import type { CommandName, HelloResponse } from "@/types/ipc";

/** Erreur levée quand une commande Rust échoue ou que l'IPC est indisponible. */
export class IpcError extends Error {
  readonly command: CommandName;

  constructor(command: CommandName, cause: unknown) {
    super(`La commande « ${command} » a échoué : ${describe(cause)}`, { cause });
    this.name = "IpcError";
    this.command = command;
  }
}

function describe(cause: unknown): string {
  if (cause instanceof Error) return cause.message;
  if (typeof cause === "string") return cause;
  try {
    return JSON.stringify(cause);
  } catch {
    return String(cause);
  }
}

async function call<T>(command: CommandName, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    throw new IpcError(command, cause);
  }
}

/** Commandes du cœur natif, une fonction par commande. */
export const ipc = {
  /** Renvoie « Hello World » et l'heure du cœur natif. */
  hello: (): Promise<HelloResponse> => call<HelloResponse>("hello"),
};
