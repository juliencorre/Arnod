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

describe("commandes du catalogue", () => {
  /** Appelle `run` sous un mock IPC qui renvoie `response` et capture l'appel. */
  async function capture(
    run: () => Promise<unknown>,
    response: unknown = null,
  ): Promise<{ cmd: string; args: unknown; result: unknown }> {
    const calls: { cmd: string; args: unknown }[] = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      return response;
    });
    const result = await run();
    expect(calls).toHaveLength(1);
    const [call] = calls;
    if (call === undefined) throw new Error("aucun appel IPC");
    return { ...call, result };
  }

  it.each([
    ["catalogLoad", () => ipc.catalogLoad(), "catalog_load", {}],
    ["undo", () => ipc.undo("jeton"), "undo", { undoToken: "jeton" }],
    [
      "projectSuggestFolder",
      () => ipc.projectSuggestFolder("boutique", null),
      "project_suggest_folder",
      { name: "boutique", location: null },
    ],
    [
      "projectPickFolder",
      () => ipc.projectPickFolder("/home/julien/dev"),
      "project_pick_folder",
      { location: "/home/julien/dev" },
    ],
    [
      "projectCreate",
      () => ipc.projectCreate({ name: "boutique", location: null, workspaceId: "ws" }),
      "project_create",
      { name: "boutique", location: null, workspaceId: "ws" },
    ],
    [
      "projectAddExisting",
      () => ipc.projectAddExisting("/home/julien/dev/legacy", "ws"),
      "project_add_existing",
      { path: "/home/julien/dev/legacy", workspaceId: "ws" },
    ],
    [
      "projectCreate sans workspace",
      () => ipc.projectCreate({ name: "boutique", location: null, workspaceId: null }),
      "project_create",
      { name: "boutique", location: null, workspaceId: null },
    ],
    ["projectForget", () => ipc.projectForget("p1"), "project_forget", { projectId: "p1" }],
    ["projectOpen", () => ipc.projectOpen("p1"), "project_open", { projectId: "p1" }],
    ["workspaceOpen", () => ipc.workspaceOpen("ws"), "workspace_open", { workspaceId: "ws" }],
    [
      "workspaceOpen (Tous les projets)",
      () => ipc.workspaceOpen(null),
      "workspace_open",
      { workspaceId: null },
    ],
    ["projectLastRuns", () => ipc.projectLastRuns(), "project_last_runs", {}],
    [
      "workspaceCreate",
      () => ipc.workspaceCreate("Boutique"),
      "workspace_create",
      { name: "Boutique" },
    ],
    [
      "workspaceRename",
      () => ipc.workspaceRename("ws", "Boutique"),
      "workspace_rename",
      { workspaceId: "ws", name: "Boutique" },
    ],
    [
      "workspaceSetColor",
      () => ipc.workspaceSetColor("ws", "teal"),
      "workspace_set_color",
      { workspaceId: "ws", color: "teal" },
    ],
    [
      "workspaceAddProjects",
      () => ipc.workspaceAddProjects("ws", ["p1", "p2"]),
      "workspace_add_projects",
      { workspaceId: "ws", projectIds: ["p1", "p2"] },
    ],
    [
      "workspaceRemoveProject",
      () => ipc.workspaceRemoveProject("ws", "p1"),
      "workspace_remove_project",
      { workspaceId: "ws", projectId: "p1" },
    ],
    ["workspaceDelete", () => ipc.workspaceDelete("ws"), "workspace_delete", { workspaceId: "ws" }],
    [
      "windowSetTitle",
      () => ipc.windowSetTitle("Boutique"),
      "window_set_title",
      { subject: "Boutique" },
    ],
  ] as const)("%s appelle `%s` avec ses arguments en camelCase", async (_name, run, cmd, args) => {
    const call = await capture(run);

    expect(call.cmd).toBe(cmd);
    expect(call.args).toEqual(args);
  });

  it("renvoie la réponse du cœur natif", async () => {
    const response = { snapshot: { projects: [] }, undoToken: "jeton" };

    const call = await capture(() => ipc.workspaceDelete("ws"), response);

    expect(call.result).toEqual(response);
  });

  it("windowSetTitle ne renvoie rien", async () => {
    const call = await capture(() => ipc.windowSetTitle(null));

    expect(call.args).toEqual({ subject: null });
    expect(call.result).toBeUndefined();
  });
});
