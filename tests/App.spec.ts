import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { afterEach, describe, expect, it } from "vitest";

import App from "@/App.vue";
import type { CatalogSnapshot, CommandError } from "@/types/ipc";

import { createFakeWindows, handleWindows } from "./fakeWindows";
import { boutiqueSnapshot, firstRunSnapshot } from "./fixtures";

let titles: (string | null)[];

/** Mock du cœur : `catalog_load` renvoie `load` (ou lève `failure`), les titres sont notés. */
function mockCore(options: {
  snapshot?: CatalogSnapshot;
  warning?: CommandError;
  failure?: CommandError;
}): void {
  titles = [];
  const windows = createFakeWindows();
  mockIPC(
    (cmd, args) => {
      if (cmd === "window_set_title") {
        titles.push((args as { subject: string | null }).subject);
        return null;
      }
      if (cmd === "windows_load") return handleWindows(windows, cmd, {});
      if (cmd !== "catalog_load") throw new Error(`commande inattendue : ${cmd}`);
      if (options.failure) throw options.failure;
      return { snapshot: options.snapshot, warning: options.warning ?? null };
    },
    { shouldMockEvents: true },
  );
}

async function mountApp() {
  const wrapper = mount(App, { global: { plugins: [createPinia()] }, attachTo: document.body });
  await flushPromises();
  return wrapper;
}

afterEach(() => {
  clearMocks();
  document.body.innerHTML = "";
});

describe("App", () => {
  it("premier lancement : seul l'onglet « Tous les projets » et l'invitation à créer (1a, R3)", async () => {
    mockCore({ snapshot: firstRunSnapshot() });

    const wrapper = await mountApp();

    expect(wrapper.get("main h1").text()).toBe("Créez votre premier projet");
    expect(wrapper.findAll('[role="tab"]').map((t) => t.text())).toEqual(["Tous les projets"]);
    expect(wrapper.get('[role="tab"]').attributes("aria-selected")).toBe("true");
    expect(wrapper.get("aside").text()).toBe("Aucun projet pour l'instant.");
    expect(wrapper.get("main").text()).toContain("Importer…");
    expect(titles).toEqual([null]);
  });

  it("rouvre « Tous les projets » si c'était le dernier onglet (R7)", async () => {
    const snapshot = boutiqueSnapshot();
    snapshot.lastWorkspaceId = null;
    mockCore({ snapshot });

    const wrapper = await mountApp();

    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 2");
    expect(titles.at(-1)).toBe("Tous les projets");
  });

  it("rouvre le dernier workspace utilisé (R7)", async () => {
    mockCore({ snapshot: boutiqueSnapshot() });

    const wrapper = await mountApp();

    expect(wrapper.get("main h1").text()).toBe("Boutique");
    expect(wrapper.get("main").text()).toContain(
      "2 projets, regroupés ici pour travailler ensemble.",
    );
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe("Boutique");
    expect(wrapper.get("aside").text()).toContain("boutique-tests");
    expect(titles.at(-1)).toBe("Boutique");
  });

  it("notifie un état illisible mis de côté", async () => {
    const warning: CommandError = {
      kind: "corruptState",
      message: "L'état d'Arnod était illisible (JSON invalide). Arnod repart d'un état neuf.",
    };
    mockCore({ snapshot: firstRunSnapshot(), warning });

    const wrapper = await mountApp();

    const toast = wrapper.get('[role="status"] .toast');
    expect(toast.text()).toBe(warning.message);
    expect(toast.classes()).toContain("toast--error");
    wrapper.unmount();
  });

  it("bloque avec le message du cœur si l'état est inutilisable", async () => {
    const failure: CommandError = {
      kind: "newerVersion",
      message: "L'état d'Arnod a été écrit par une version plus récente (format 2).",
    };
    mockCore({ failure });

    const wrapper = await mountApp();

    const alert = wrapper.get('[role="alert"]');
    expect(alert.text()).toContain("Arnod ne peut pas démarrer");
    expect(alert.text()).toContain(failure.message);
    expect(wrapper.find("aside").exists()).toBe(false);
  });
});
