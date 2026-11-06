import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { IpcError, ipc } from "@/lib/ipc";
import type { HelloResponse } from "@/types/ipc";

afterEach(() => {
  clearMocks();
});

describe("ipc.hello", () => {
  it("appelle la commande Rust `hello` sans argument et renvoie sa réponse", async () => {
    const response: HelloResponse = { message: "Hello World", timestampMs: 1_762_400_000_123 };
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return response;
    });

    await expect(ipc.hello()).resolves.toEqual(response);
    expect(calls).toEqual([{ cmd: "hello", args: {} }]);
  });

  it("encapsule l'erreur du cœur natif dans une IpcError", async () => {
    mockIPC(() => {
      throw "cœur indisponible";
    });

    const error = await ipc.hello().catch((e: unknown) => e);

    expect(error).toBeInstanceOf(IpcError);
    expect(error).toMatchObject({
      name: "IpcError",
      command: "hello",
      cause: "cœur indisponible",
      message: "La commande « hello » a échoué : cœur indisponible",
    });
  });

  it("décrit une erreur structurée renvoyée par Rust", async () => {
    mockIPC(() => {
      throw { kind: "io", detail: "disque plein" };
    });

    await expect(ipc.hello()).rejects.toThrow(
      'La commande « hello » a échoué : {"kind":"io","detail":"disque plein"}',
    );
  });
});
