import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";

import { IpcError, ipc } from "@/lib/ipc";
import type { CommandError, HelloResponse } from "@/types/ipc";

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
      kind: null,
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

describe("IpcError et CommandError", () => {
  async function failWith(cause: unknown): Promise<IpcError> {
    mockIPC(() => {
      throw cause;
    });
    const error = await ipc.hello().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(IpcError);
    return error as IpcError;
  }

  it("reprend la nature et le message d'une CommandError renvoyée par Rust", async () => {
    const cause: CommandError = {
      kind: "nameTaken",
      message: "Le nom « Boutique » est déjà utilisé.",
    };

    const error = await failWith(cause);

    expect(error).toMatchObject({
      name: "IpcError",
      command: "hello",
      kind: "nameTaken",
      message: "Le nom « Boutique » est déjà utilisé.",
      cause,
    });
  });

  it.each([
    ["une nature inconnue", { kind: "inconnue", message: "Texte" }],
    ["un message absent", { kind: "io" }],
    ["un message qui n'est pas un texte", { kind: "io", message: 42 }],
    ["une nature héritée du prototype", { kind: "toString", message: "Texte" }],
    ["null", null],
  ])("ne reconnaît pas de CommandError pour %s", async (_label, cause) => {
    const error = await failWith(cause);

    expect(error.kind).toBeNull();
    expect(error.message).toBe(`La commande « hello » a échoué : ${JSON.stringify(cause)}`);
  });
});
