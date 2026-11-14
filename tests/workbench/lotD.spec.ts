/**
 * Lot D - Barre latérale (docs/plan-mvp-d-barre-laterale.md) : filtre du référentiel (1.2-e),
 * projet créé (1.2-d), menu « + » (1.2-a ter), réponses attendues dans l'arborescence (3.2-a3) et
 * nombre de contrats sur les cartes (1.1-a). Clavier compris.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { Example } from "@/types/ipc";

import type { FakeCore } from "../fakeCore";
import { PETSTORE_OPERATIONS, PETSTORE_SOURCE } from "../fakeContracts";
import { newRequest } from "../fakeWorkbench";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function wrapper(): AppWrapper {
  if (!app) throw new Error("application non montée");
  return app;
}

const PETSTORE = "contracts/petstore.yaml";

function example(status: number, name: string): Example {
  return { name, description: "", status, statusText: "", headers: [], bodyKind: "none", body: "" };
}

/** Référentiel « commandes/v2 » à filtrer, comme l'écran 1.2-e. */
function commandes(core: FakeCore): void {
  const wb = core.workbench;
  wb.folders.add("commandes");
  wb.folders.add("commandes/v2");
  wb.requests.set(
    "commandes/v2/lister.yml",
    newRequest("Lister les commandes", { url: "{{baseUrl}}/orders?statut=ouvert" }),
  );
  wb.requests.set(
    "commandes/v2/changer.yml",
    newRequest("Changer le statut", {
      method: "PATCH",
      url: "{{baseUrl}}/orders/{{orderId}}/status",
      tags: ["critique"],
    }),
  );
}

function treeNames(): string[] {
  return wrapper()
    .findAll('.tree [role="treeitem"] .row__name')
    .map((n) => n.text());
}

function filterInput() {
  return wrapper().get(".tree .tree-filter input");
}

describe("filtrer le référentiel (1.2-e, F1 à F5)", () => {
  it("filtre par nom et adresse, dossiers dépliés, compte affiché ; Échap efface", async () => {
    ({ app } = await openProject(commandes));
    // Un dossier replié reste replié dans l'état enregistré, mais se déplie pendant le filtre.
    await click(wrapper(), '.tree [role="treeitem"]', "commandes");
    expect(treeNames()).not.toContain("Changer le statut");

    await filterInput().setValue("stat");
    await flushPromises();
    expect(wrapper().get(".tree .tree-filter__count").text()).toBe("2 sur 5");
    expect(treeNames().sort()).toEqual(
      ["Changer le statut", "Lister les commandes", "commandes", "v2"].sort(),
    );
    expect(wrapper().get(".tree .row__hit").text()).toBe("stat");
    // « Lister les commandes » n'est trouvée que par son adresse.
    expect(
      wrapper()
        .findAll(".tree .row__via")
        .map((v) => v.text()),
    ).toEqual(["adresse"]);
    expect(wrapper().get(".tree__help").text()).toContain("#étiquette");

    await filterInput().trigger("keydown", { key: "Escape" });
    await flushPromises();
    expect((filterInput().element as HTMLInputElement).value).toBe("");
    expect(wrapper().find(".tree .tree-filter__count").exists()).toBe(false);
    expect(treeNames()).not.toContain("Changer le statut");
  });

  it("#étiquette et méthode ; aucun résultat le dit ; × efface", async () => {
    ({ app } = await openProject(commandes));
    await filterInput().setValue("#crit");
    await flushPromises();
    expect(treeNames()).toEqual(["commandes", "v2", "Changer le statut"]);

    await filterInput().setValue("POST");
    await flushPromises();
    expect(treeNames()).toEqual(["Clients", "Créer"]);

    await filterInput().setValue("introuvable");
    await flushPromises();
    expect(wrapper().get(".tree__none").text()).toBe("Aucune requête ne correspond.");
    expect(wrapper().get(".tree .tree-filter__count").text()).toBe("0 sur 5");

    await click(wrapper(), ".tree .tree-filter__clear");
    expect((filterInput().element as HTMLInputElement).value).toBe("");
    expect(document.activeElement).toBe(filterInput().element);
  });

  it("un clic ouvre la requête et garde le filtre", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject(commandes));
    await filterInput().setValue("stat");
    await flushPromises();
    await click(wrapper(), '.tree [role="treeitem"]', "Changer le statut");
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual({
      content: { kind: "request", projectId: expect.any(String), path: "commandes/v2/changer.yml" },
      preview: true,
    });
    expect((filterInput().element as HTMLInputElement).value).toBe("stat");
  });

  it("Ctrl+F dans la barre latérale met le focus dans le filtre", async () => {
    ({ app } = await openProject());
    // Hors de la barre latérale, Ctrl+F reste au navigateur.
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "f", ctrlKey: true }));
    await flushPromises();
    expect(document.activeElement).not.toBe(filterInput().element);

    (wrapper().get(".project__main").element as HTMLElement).focus();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "f", ctrlKey: true }));
    await flushPromises();
    expect(document.activeElement).toBe(filterInput().element);
  });
});

describe("projet créé (1.2-d) et menu « + » (1.2-a ter)", () => {
  async function emptyProject(): Promise<FakeCore> {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.workbench.folders.clear();
      c.workbench.requests.clear();
    }));
    return core;
  }

  it("deux sections repliées, sans ligne d'ajout, et quatre points de départ (D1, D2)", async () => {
    await emptyProject();
    const toggles = wrapper().findAll(".section-head__toggle");
    expect(toggles.map((t) => [t.text(), t.attributes("aria-expanded")])).toEqual([
      ["▸Contrats de service", "false"],
      ["▸Référentiel de tests", "false"],
    ]);
    expect(wrapper().find(".tree__ghost").exists()).toBe(false);
    expect(wrapper().find(".contracts__ghost").exists()).toBe(false);
    expect(
      wrapper()
        .findAll("main .start__action")
        .map((b) => b.text()),
    ).toEqual(["Importer…", "+ Contrat", "+ Requête", "+ Environnement"]);

    // Au clavier : Entrée sur l'en-tête déplie la section et montre sa ligne d'ajout.
    await toggles[1]?.trigger("click");
    expect(wrapper().get(".tree__ghost").text()).toBe("+ Nouvelle requête");
  });

  it("les points de départ créent un contrat, une requête, un environnement", async () => {
    const core = await emptyProject();
    await click(wrapper(), "main .start__action", "+ Contrat");
    expect(wrapper().get("dialog[open] h2").text()).toBe("Nouveau contrat");
    await click(wrapper(), "dialog[open] button", "Annuler");

    await click(wrapper(), "main .start__action", "+ Environnement");
    expect(wrapper().get("dialog[open]").text()).toContain("Environnements");
    await click(wrapper(), "dialog[open] button", "Fermer");

    await click(wrapper(), "main .start__action", "+ Requête");
    expect(core.calls.find((c) => c.cmd === "tree_create_request")?.args).toMatchObject({
      folder: "",
    });
  });

  it("Ctrl+Maj+N crée un dossier et déplie le référentiel (M2)", async () => {
    await emptyProject();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "N", ctrlKey: true, shiftKey: true }));
    await flushPromises();
    expect(wrapper().get("dialog[open] h2").text()).toBe("Nouveau dossier");
    expect(wrapper().findAll(".section-head__toggle")[1]?.attributes("aria-expanded")).toBe("true");
  });
});

describe("réponses attendues dans l'arborescence (3.2-a3, E1 à E3)", () => {
  async function petstore(): Promise<FakeCore> {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.contracts.files.set(PETSTORE, {
        name: "Petstore",
        version: "1.0.0",
        source: PETSTORE_SOURCE,
        operations: structuredClone(PETSTORE_OPERATIONS),
      });
      c.workbench.folders.add("Petstore");
      c.workbench.requests.set(
        "Petstore/creer.yml",
        newRequest("Créer un animal", {
          method: "POST",
          contract: { file: PETSTORE, operation: "POST /pets" },
          examples: [example(201, "Animal créé"), example(400, "Erreur de validation")],
        }),
      );
    }));
    await click(wrapper(), '[aria-label="Déplier le contrat Petstore"]');
    await click(wrapper(), ".crow__resource", "/pets");
    return core;
  }

  it("le nombre sur l'endpoint ; dépliée, les réponses par code (E1, E2)", async () => {
    await petstore();
    const post = wrapper().get('[aria-label="Ouvrir POST /pets"]');
    expect(post.get(".crow__examples").text()).toBe("2");
    await click(wrapper(), '[aria-label="Déplier POST /pets"]');
    const rows = wrapper().findAll(".crow__example");
    expect(rows.map((r) => [r.get(".crow__code").text(), r.get(".crow__name").text()])).toEqual([
      ["201", "Animal créé"],
      ["400", "Erreur de validation"],
    ]);
    expect(rows[0]?.get(".crow__code").classes()).toContain("crow__code--2xx");
    expect(rows[1]?.get(".crow__code").classes()).toContain("crow__code--4xx");
  });

  it("un clic ouvre l'endpoint sur la réponse choisie (E3)", async () => {
    const core = await petstore();
    await click(wrapper(), '[aria-label="Déplier POST /pets"]');
    await click(wrapper(), '[aria-label="Ouvrir la réponse attendue 400 Erreur de validation"]');
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual({
      content: { kind: "request", projectId: expect.any(String), path: "Petstore/creer.yml" },
    });
    await flushPromises();
    const tab = wrapper()
      .findAll('[role="tab"]')
      .find((t) => t.text().startsWith("Réponses attendues"));
    expect(tab?.attributes("aria-selected")).toBe("true");
    const fields = wrapper().findAll(".expected input[type='text']");
    expect(fields.map((f) => (f.element as HTMLInputElement).value)).toContain(
      "Erreur de validation",
    );
  });

  it("le ⋯ de l'opération agit sur l'endpoint (RC5)", async () => {
    await petstore();
    const trigger = wrapper().get('[aria-label="Actions pour Créer un animal"]');
    await trigger.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(
      wrapper()
        .findAll('[role="menuitem"]')
        .map((i) => i.text()),
    ).toEqual(["Renommer", "Dupliquer", "Copier ou déplacer…", "Mettre à la corbeille"]);
  });
});

describe("cartes de projet (écart 1.1-a, W1)", () => {
  it("disent le nombre de contrats de service", async () => {
    const { installFakeCore } = await import("../fakeCore");
    const { boutiqueSnapshot } = await import("../fixtures");
    const { mountApp } = await import("../mountApp");
    const core = installFakeCore(boutiqueSnapshot());
    core.contracts.files.set(PETSTORE, {
      name: "Petstore",
      version: "1.0.0",
      source: PETSTORE_SOURCE,
      operations: structuredClone(PETSTORE_OPERATIONS),
    });
    app = await mountApp();
    await flushPromises();
    const stats = wrapper()
      .findAll("main article .card__stats")
      .map((s) => s.text().replace(/\s+/g, " "));
    // Le faux cœur partage ses contrats entre projets : chaque carte présente en compte un.
    expect(stats.length).toBeGreaterThan(0);
    expect(stats.every((s) => s === "1 contrat de service")).toBe(true);
    expect(core.calls.some((c) => c.cmd === "contract_count")).toBe(true);
  });
});
