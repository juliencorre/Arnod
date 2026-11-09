import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";

import { useCatalogStore } from "@/stores/catalog";
import { useNavigationStore } from "@/stores/navigation";
import type { CatalogSnapshot } from "@/types/ipc";

import {
  BOUTIQUE,
  BOUTIQUE_TESTS,
  MES_PROJETS,
  boutiqueSnapshot,
  firstRunSnapshot,
} from "../fixtures";

let titles: (string | null)[];
let opened: (string | null)[];

/** Mock IPC : chargement de `snapshot`, ouverture de workspace et titres de fenêtre notés. */
function mockCore(snapshot: CatalogSnapshot): void {
  mockIPC((cmd, args) => {
    const payload = args as Record<string, unknown>;
    switch (cmd) {
      case "catalog_load":
        return { snapshot, warning: null };
      case "workspace_open": {
        const workspaceId = (payload.workspaceId as string | null) ?? null;
        opened.push(workspaceId);
        return { ...snapshot, lastWorkspaceId: workspaceId };
      }
      case "window_set_title":
        titles.push(payload.subject as string | null);
        return null;
      default:
        throw new Error(`commande inattendue : ${cmd}`);
    }
  });
}

async function started(snapshot: CatalogSnapshot) {
  mockCore(snapshot);
  const catalog = useCatalogStore();
  await catalog.load();
  const navigation = useNavigationStore();
  navigation.start();
  await nextTick();
  return { catalog, navigation };
}

beforeEach(() => {
  setActivePinia(createPinia());
  titles = [];
  opened = [];
});

afterEach(() => {
  clearMocks();
});

describe("useNavigationStore", () => {
  it("affiche le premier lancement sur « Tous les projets », titre « Arnod » seul", async () => {
    const { navigation } = await started(firstRunSnapshot());

    expect(navigation.view).toEqual({ kind: "all", highlightProjectId: null });
    expect(navigation.currentWorkspaceId).toBeNull();
    expect(titles).toEqual([null]);
  });

  it("rouvre le dernier workspace utilisé (W2) et le met dans le titre", async () => {
    const { navigation } = await started(boutiqueSnapshot());

    expect(navigation.view).toEqual({ kind: "workspace", workspaceId: BOUTIQUE });
    expect(navigation.currentWorkspaceId).toBe(BOUTIQUE);
    expect(titles.at(-1)).toBe("Boutique");
    expect(opened).toEqual([]);
  });

  it("ouvrir un workspace le mémorise côté cœur", async () => {
    const { navigation } = await started(boutiqueSnapshot());

    await navigation.showWorkspace(MES_PROJETS);
    await nextTick();

    expect(opened).toEqual([MES_PROJETS]);
    expect(navigation.view).toEqual({ kind: "workspace", workspaceId: MES_PROJETS });
    expect(titles.at(-1)).toBe("Mes projets");
  });

  it("titre du projet ouvert, de « Tous les projets » et de la gestion", async () => {
    const { navigation } = await started(boutiqueSnapshot());

    navigation.showProject(BOUTIQUE_TESTS, BOUTIQUE);
    await nextTick();
    expect(titles.at(-1)).toBe("boutique-tests");
    expect(navigation.currentWorkspaceId).toBe(BOUTIQUE);

    navigation.showManage();
    await nextTick();
    expect(titles.at(-1)).toBe("Workspaces");
    expect(navigation.view).toEqual({ kind: "manage", workspaceId: BOUTIQUE, returnTo: BOUTIQUE });
    expect(navigation.currentWorkspaceId).toBe(BOUTIQUE);

    await navigation.showAll();
    await nextTick();
    expect(titles.at(-1)).toBe("Tous les projets");
    expect(opened.at(-1)).toBeNull();
    expect(navigation.currentWorkspaceId).toBeNull();
  });

  it("suit un renommage dans le titre", async () => {
    const { catalog } = await started(boutiqueSnapshot());
    const renamed = boutiqueSnapshot();
    const boutique = renamed.workspaces[1];
    if (boutique) boutique.name = "Boutique v2";

    catalog.snapshot = renamed;
    await nextTick();

    expect(titles.at(-1)).toBe("Boutique v2");
  });

  it("revient sur « Tous les projets » si le workspace affiché disparaît (R8)", async () => {
    const { catalog, navigation } = await started(boutiqueSnapshot());
    const next = boutiqueSnapshot();
    next.workspaces = next.workspaces.filter((w) => w.id !== BOUTIQUE);
    next.lastWorkspaceId = null;

    catalog.snapshot = next;
    await nextTick();

    expect(navigation.view).toEqual({ kind: "all", highlightProjectId: null });
  });

  it("un projet ouvert depuis un workspace supprimé reste ouvert, sous « Tous les projets »", async () => {
    const { catalog, navigation } = await started(boutiqueSnapshot());
    navigation.showProject(BOUTIQUE_TESTS, BOUTIQUE);
    const next = boutiqueSnapshot();
    next.workspaces = next.workspaces.filter((w) => w.id !== BOUTIQUE);

    catalog.snapshot = next;
    await nextTick();

    expect(navigation.view).toEqual({
      kind: "project",
      projectId: BOUTIQUE_TESTS,
      workspaceId: null,
    });
  });

  it("un échec du titre de fenêtre n'interrompt rien", async () => {
    mockIPC((cmd) => {
      if (cmd === "window_set_title") throw new Error("fenêtre fermée");
      return { snapshot: boutiqueSnapshot(), warning: null };
    });
    const catalog = useCatalogStore();
    await catalog.load();
    const navigation = useNavigationStore();

    navigation.start();
    await nextTick();

    expect(navigation.view).toEqual({ kind: "workspace", workspaceId: BOUTIQUE });
  });
});
