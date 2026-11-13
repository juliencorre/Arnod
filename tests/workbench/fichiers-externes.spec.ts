/**
 * Lot I - Fichiers modifiés hors d'Arnod (docs/plan-mvp-i-fichiers-externes.md, écran 4.1-e) :
 * onglet relu, onglet fermé, renommage suivi, conflit réglé élément par élément au clavier.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EVENTS } from "@/lib/ipc";
import { AUTOSAVE_MS } from "@/stores/workbench";
import type { ProjectChange } from "@/types/ipc";

import { filesChangeLotI } from "../fakeFiles";
import { BOUTIQUE_TESTS } from "../fixtures";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

/** Changement diffusé par le surveillant du cœur (origine vide). */
function fromDisk(path: string): ProjectChange {
  return { projectId: BOUTIQUE_TESTS, scope: "request", path, origin: "" };
}

describe("Fichiers modifiés hors d'Arnod (lot I)", () => {
  it("relit un onglet sans brouillon et le signale", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    const stored = core.workbench.requests.get("Clients/lister.yml");
    if (stored) stored.url = "{{baseUrl}}/v2/clients";

    await core.emit(EVENTS.project, fromDisk("Clients/lister.yml"));
    await flushPromises();

    expect(app.get<HTMLInputElement>(".editor__url").element.value).toBe("{{baseUrl}}/v2/clients");
    expect(app.get(".editor__saved").text()).toContain("Rechargé depuis le disque");
  });

  it("ferme l'onglet d'une requête supprimée, avec une notification", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    core.workbench.requests.delete("Clients/lister.yml");

    await core.emit(
      EVENTS.files,
      filesChangeLotI(BOUTIQUE_TESTS, { removed: ["Clients/lister.yml"] }),
    );
    await flushPromises();

    expect(app.find(".editor__url").exists()).toBe(false);
    expect(document.body.textContent).toContain("supprimée hors d'Arnod");
  });

  it("garde le brouillon d'une requête supprimée : Restaurer la réécrit", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await app.get(".editor__url").setValue("{{baseUrl}}/brouillon");
    core.workbench.requests.delete("Clients/lister.yml");

    await core.emit(
      EVENTS.files,
      filesChangeLotI(BOUTIQUE_TESTS, { removed: ["Clients/lister.yml"] }),
    );
    await flushPromises();
    expect(app.get(".conflict__title").text()).toContain("supprimée hors d'Arnod");

    await click(app, ".conflict__actions button", "Restaurer");
    expect(core.workbench.requests.get("Clients/lister.yml")?.url).toBe("{{baseUrl}}/brouillon");
    expect(app.find(".conflict").exists()).toBe(false);
  });

  it("annonce un lot de changements dans une notification", async () => {
    let core;
    ({ app, core } = await openProject());
    await core.emit(
      EVENTS.files,
      filesChangeLotI(BOUTIQUE_TESTS, {
        modified: ["a.yml", "b.yml", "c.yml", "d.yml"],
        added: ["e.yml"],
      }),
    );
    await flushPromises();
    expect(document.body.textContent).toContain(
      "Rechargé depuis le disque : 4 requêtes modifiées, 1 ajoutée",
    );
  });

  it("suspend l'enregistrement automatique et règle le conflit au clavier", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    vi.useFakeTimers();
    await app.get(".editor__url").setValue("{{baseUrl}}/mien");
    const stored = core.workbench.requests.get("Clients/lister.yml");
    if (stored) {
      stored.url = "{{baseUrl}}/disque";
      stored.headers.push({
        name: "X-Request-Id",
        value: "{{$randomUUID}}",
        enabled: true,
        kind: "query",
        description: "",
      });
    }

    await core.emit(EVENTS.project, fromDisk("Clients/lister.yml"));
    await flushPromises();
    await vi.advanceTimersByTimeAsync(AUTOSAVE_MS * 3);
    // Rien n'est écrasé tant que le conflit n'est pas réglé.
    expect(core.workbench.requests.get("Clients/lister.yml")?.url).toBe("{{baseUrl}}/disque");
    expect(core.calls.filter((c) => c.cmd === "request_save")).toHaveLength(0);

    const rows = app.findAll(".conflict__row:not(.conflict__row--same)");
    expect(rows.map((r) => r.get(".conflict__label").text())).toEqual([
      "Adresse",
      "En-tête X-Request-Id",
    ]);
    // L'en-tête ajouté sur le disque est proposé « disque » ; l'adresse, changée des deux côtés,
    // attend un choix : « Appliquer » reste désactivé.
    const radios = (index: number) =>
      rows[index]?.findAll<HTMLInputElement>('input[type="radio"]') ?? [];
    expect(radios(0).map((r) => r.element.checked)).toEqual([false, false]);
    expect(radios(1)[0]?.element.checked).toBe(true);
    const applyButton = () =>
      app
        ?.findAll<HTMLButtonElement>(".conflict__actions button")
        .find((b) => b.text().includes("Appliquer"));
    expect(applyButton()?.attributes("disabled")).toBeDefined();

    // Au clavier : les flèches d'un groupe radio cochent l'autre version (événement change).
    for (const [index, side] of [
      [0, 1],
      [1, 1],
    ] as const) {
      const radio = radios(index)[side];
      radio?.element.focus();
      if (radio) {
        radio.element.checked = true;
        await radio.trigger("change");
      }
    }
    expect(radios(0)[1]?.element.checked).toBe(true);
    expect(radios(1)[1]?.element.checked).toBe(true);

    const apply = applyButton();
    expect(apply?.attributes("disabled")).toBeUndefined();
    apply?.element.focus();
    expect(document.activeElement).toBe(apply?.element);
    await apply?.trigger("click");
    await flushPromises();
    vi.useRealTimers();

    const written = core.workbench.requests.get("Clients/lister.yml");
    expect(written?.url).toBe("{{baseUrl}}/mien");
    expect(written?.headers).toEqual([]);
    expect(app.find(".conflict").exists()).toBe(false);
  });

  it("Tout prendre du disque puis appliquer reprend la version du disque", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await app.get(".editor__url").setValue("{{baseUrl}}/mien");
    const stored = core.workbench.requests.get("Clients/lister.yml");
    if (stored) stored.method = "PUT";

    await core.emit(EVENTS.project, fromDisk("Clients/lister.yml"));
    await flushPromises();
    expect(app.get(".conflict__count").text()).toContain("2 différences");

    await click(app, ".conflict__actions button", "Tout prendre du disque");
    await click(app, ".conflict__actions button", "Appliquer et enregistrer");
    expect(core.workbench.requests.get("Clients/lister.yml")?.url).toBe("{{baseUrl}}/clients");
    expect(app.get<HTMLSelectElement>(".editor__method").element.value).toBe("PUT");
  });

  it("le brouillon d'une requête renommée hors d'Arnod suit son nouveau chemin", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    const moved = core.workbench.requests.get("Clients/lister.yml");
    if (moved) {
      core.workbench.requests.delete("Clients/lister.yml");
      core.workbench.requests.set("Clients/liste.yml", moved);
    }
    await core.emit(
      EVENTS.files,
      filesChangeLotI(BOUTIQUE_TESTS, {
        renamed: [{ from: "Clients/lister.yml", to: "Clients/liste.yml" }],
      }),
    );
    await flushPromises();
    const store = (await import("@/stores/workbench")).useWorkbenchStore();
    expect(Object.values(store.editors).map((e) => e.path)).toContain("Clients/liste.yml");
  });
});
