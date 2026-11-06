import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { IpcError } from "@/lib/ipc";
import { useCatalogStore } from "@/stores/catalog";
import type { CatalogSnapshot } from "@/types/ipc";

import {
  AUTH_SSO,
  BOUTIQUE,
  BOUTIQUE_TESTS,
  MES_PROJETS,
  boutiqueSnapshot,
  firstRunSnapshot,
} from "../fixtures";

/** Mock IPC qui renvoie `responses[cmd]` et note les appels. */
function mockCommands(responses: Record<string, unknown>): { cmd: string; args: unknown }[] {
  const calls: { cmd: string; args: unknown }[] = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    if (!(cmd in responses)) throw new Error(`commande inattendue : ${cmd}`);
    const response = responses[cmd];
    if (response instanceof Error) throw response;
    return response;
  });
  return calls;
}

async function loadedStore(snapshot: CatalogSnapshot) {
  mockCommands({ catalog_load: { snapshot, warning: null } });
  const catalog = useCatalogStore();
  await catalog.load();
  return catalog;
}

beforeEach(() => {
  setActivePinia(createPinia());
});

afterEach(() => {
  clearMocks();
});

describe("useCatalogStore", () => {
  it("est vide avant le chargement", () => {
    const catalog = useCatalogStore();

    expect(catalog.snapshot).toBeNull();
    expect(catalog.isFirstRun).toBe(false);
    expect(catalog.workspaces).toEqual([]);
  });

  it("charge l'instantané et renvoie l'avertissement éventuel", async () => {
    const warning = { kind: "corruptState", message: "L'état d'Arnod était illisible." };
    mockCommands({ catalog_load: { snapshot: firstRunSnapshot(), warning } });
    const catalog = useCatalogStore();

    await expect(catalog.load()).resolves.toEqual(warning);

    expect(catalog.isFirstRun).toBe(true);
    expect(catalog.defaultWorkspace?.name).toBe("Mes projets");
  });

  it("donne les projets d'un workspace dans son ordre", async () => {
    const snapshot = boutiqueSnapshot();
    const boutique = snapshot.workspaces[1];
    if (boutique) boutique.projectIds = [AUTH_SSO, BOUTIQUE_TESTS, "inconnu"];
    const catalog = await loadedStore(snapshot);

    expect(catalog.isFirstRun).toBe(false);
    expect(catalog.projectsOf(BOUTIQUE).map((p) => p.name)).toEqual(["auth-sso", "boutique-tests"]);
    expect(catalog.projectsOf("inconnu")).toEqual([]);
    expect(catalog.workspaceById(BOUTIQUE)?.name).toBe("Boutique");
    expect(catalog.projectById("inconnu")).toBeNull();
  });

  it("remplace l'état par l'instantané renvoyé par chaque modification", async () => {
    const catalog = await loadedStore(boutiqueSnapshot());
    const renamed = boutiqueSnapshot();
    const boutique = renamed.workspaces[1];
    if (boutique) boutique.name = "Boutique v2";
    const calls = mockCommands({ workspace_rename: renamed });

    await catalog.renameWorkspace(BOUTIQUE, "Boutique v2");

    expect(calls).toEqual([
      { cmd: "workspace_rename", args: { workspaceId: BOUTIQUE, name: "Boutique v2" } },
    ]);
    expect(catalog.workspaceById(BOUTIQUE)?.name).toBe("Boutique v2");
  });

  it("renvoie les identifiants créés et les jetons d'annulation", async () => {
    const catalog = await loadedStore(firstRunSnapshot());
    mockCommands({
      workspace_create: { snapshot: boutiqueSnapshot(), workspaceId: BOUTIQUE },
      project_create: { snapshot: boutiqueSnapshot(), projectId: BOUTIQUE_TESTS },
      workspace_delete: { snapshot: firstRunSnapshot(), undoToken: "jeton-1" },
      workspace_remove_project: { snapshot: boutiqueSnapshot(), undoToken: "jeton-2" },
    });

    await expect(catalog.createWorkspace("Boutique")).resolves.toBe(BOUTIQUE);
    await expect(
      catalog.createProject({ name: "boutique-tests", location: null, workspaceId: MES_PROJETS }),
    ).resolves.toBe(BOUTIQUE_TESTS);
    await expect(catalog.deleteWorkspace(BOUTIQUE)).resolves.toBe("jeton-1");
    expect(catalog.isFirstRun).toBe(true);
    await expect(catalog.removeProject(BOUTIQUE, AUTH_SSO)).resolves.toBe("jeton-2");
    expect(catalog.isFirstRun).toBe(false);
  });

  it("laisse l'état intact et remonte l'erreur du cœur natif", async () => {
    const catalog = await loadedStore(boutiqueSnapshot());
    const before = catalog.snapshot;
    mockIPC(() => {
      throw { kind: "nameTaken", message: "Un workspace « Mes projets » existe déjà." };
    });

    const error = await catalog.renameWorkspace(BOUTIQUE, "mes projets").catch((e: unknown) => e);

    expect(error).toBeInstanceOf(IpcError);
    expect(error).toMatchObject({
      kind: "nameTaken",
      message: "Un workspace « Mes projets » existe déjà.",
    });
    expect(catalog.snapshot).toStrictEqual(before);
  });
});
