import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { EVENTS } from "@/lib/ipc";

import type { FakeCore } from "../fakeCore";
import { PETSTORE_OPERATIONS, PETSTORE_SOURCE } from "../fakeContracts";
import { BOUTIQUE_TESTS } from "../fixtures";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

/** Projet ouvert avec, s'il le faut, le contrat Petstore déjà ajouté. */
async function setup(withContract = false): Promise<{ app: AppWrapper; core: FakeCore }> {
  const opened = await openProject((core) => {
    if (withContract) {
      core.contracts.files.set("contracts/petstore.yaml", {
        name: "Petstore",
        version: "1.0.0",
        source: PETSTORE_SOURCE,
        operations: structuredClone(PETSTORE_OPERATIONS),
      });
    }
  });
  app = opened.app;
  return opened;
}

async function addMenu(wrapper: AppWrapper, item: string): Promise<void> {
  await wrapper.get('[aria-label="Ajouter un contrat à boutique-tests"]').trigger("click");
  await flushPromises();
  await click(wrapper, '[role="menuitem"]', item);
}

async function openContract(wrapper: AppWrapper, name = "Petstore"): Promise<void> {
  await click(wrapper, ".contract-row__main", name);
}

function operationRows(wrapper: AppWrapper): string[] {
  return wrapper
    .findAll(".op")
    .map((row) =>
      [".op__method", ".op__path", ".op__summary"].map((part) => row.get(part).text()).join(" "),
    );
}

describe("ajouter un contrat (C1 à C4)", () => {
  it("colle un contrat : refusé avec la raison, puis ajouté et ouvert", async () => {
    const { core } = await setup();
    expect(app?.get(".contracts__empty").text()).toBe(
      "Aucun contrat : « + Contrat » en ajoute un.",
    );

    await addMenu(app as AppWrapper, "Coller un contrat…");
    const dialog = app?.get("dialog[open]");
    expect(dialog?.get("h2").text()).toBe("Coller un contrat");
    await dialog?.get("textarea").setValue("titre: pas un contrat");
    await dialog?.get("form").trigger("submit");
    await flushPromises();
    expect(app?.get("dialog[open] [role='alert']").text()).toContain(
      "Ce texte n'est pas un contrat lisible",
    );
    expect(core.contracts.files.size).toBe(0);

    await app?.get("dialog[open] textarea").setValue(PETSTORE_SOURCE);
    await app?.get("dialog[open] form").trigger("submit");
    await flushPromises();

    expect(app?.find("dialog[open]").exists()).toBe(false);
    expect(app?.get(".contract-row").text()).toContain("Petstore");
    expect(app?.get(".contract-row__version").text()).toBe("1.0.0");
    expect(app?.get(".contract__name").text()).toBe("Petstore");
    expect(app?.get(".zone__tab-button[aria-selected='true']").text()).toContain("Petstore");
    expect(app?.get(".toast").text()).toContain("Contrat « Petstore » ajouté.");
  });

  it("une adresse injoignable est signalée dans le dialogue", async () => {
    await setup();
    await addMenu(app as AppWrapper, "Depuis une adresse…");
    await app?.get("dialog[open] input").setValue("https://exemple.invalid/openapi.json");
    await app?.get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(app?.get("dialog[open] [role='alert']").text()).toContain("Impossible de joindre");
  });

  it("un nouveau contrat vide s'ouvre sur sa source", async () => {
    await setup();
    await addMenu(app as AppWrapper, "Nouveau contrat…");
    await app?.get("dialog[open] input").setValue("Paiements");
    await app?.get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(app?.get(".contract__name").text()).toBe("Paiements");
    expect(app?.get('.contract__tab[aria-selected="true"]').text()).toContain("Source");
    expect((app?.get(".contract__source").element as HTMLTextAreaElement).value).toContain(
      'title: "Paiements"',
    );
  });

  it("importe un fichier choisi ; rien si le choix est annulé", async () => {
    const { core } = await setup();
    await addMenu(app as AppWrapper, "Importer un fichier…");
    expect(core.contracts.files.size).toBe(0);

    core.contracts.pickedFile = PETSTORE_SOURCE;
    await addMenu(app as AppWrapper, "Importer un fichier…");
    expect(core.calls.filter((c) => c.cmd === "contract_import_file").at(-1)?.args).toMatchObject({
      replace: null,
    });
    expect(app?.get(".contract__name").text()).toBe("Petstore");
  });
});

describe("onglet du contrat (C5, C6, C10)", () => {
  it("présente les opérations groupées par tag, toutes cochées", async () => {
    await setup(true);
    await openContract(app as AppWrapper);

    expect(app?.get(".contract__version").text()).toBe("version 1.0.0");
    expect(app?.get(".contract__kind").text()).toBe("OpenAPI 3.0.3");
    expect(app?.findAll(".group__title").map((t) => t.text())).toEqual(["animaux", "Sans tag"]);
    expect(operationRows(app as AppWrapper)).toEqual([
      "GET /pets Lister les animaux",
      "POST /pets Créer un animal",
      "GET /health Santé",
    ]);
    expect(app?.get(".contract__coverage").text()).toBe("3 opérations · 0 avec requête");
    expect(
      app
        ?.findAll("button")
        .find((b) => b.text().startsWith("Créer les requêtes"))
        ?.text(),
    ).toBe("Créer les requêtes (3)");
  });

  it("crée les requêtes cochées, renseigne baseUrl, puis ne propose plus que les autres", async () => {
    const { core } = await setup(true);
    await openContract(app as AppWrapper);

    await app?.get('[aria-label="Créer la requête de GET /health"]').trigger("change");
    await click(app as AppWrapper, ".contract__actions button", "Créer les requêtes (2)");

    expect(core.calls.find((c) => c.cmd === "contract_generate")?.args).toEqual({
      project: BOUTIQUE_TESTS,
      path: "contracts/petstore.yaml",
      operations: ["GET /pets", "POST /pets"],
      environment: "Local",
    });
    expect(app?.get(".toast").text()).toBe(
      "2 requêtes créées dans « Petstore ». baseUrl de « Local » : http://127.0.0.1:8080.",
    );
    expect(app?.get(".contract__coverage").text()).toBe("3 opérations · 2 avec requête");
    expect(app?.findAll(".op__open")).toHaveLength(2);
    expect(app?.get(".contract__actions").text()).toContain("Créer les requêtes (0)");
    // Les requêtes créées apparaissent dans l'arborescence.
    expect(app?.findAll('[role="treeitem"]').some((i) => i.text().includes("Petstore"))).toBe(true);

    // Cocher à nouveau l'opération restante : une seule requête de plus.
    await app?.get('[aria-label="Créer la requête de GET /health"]').trigger("change");
    expect(app?.get(".contract__actions").text()).toContain("Créer les requêtes (1)");
  });

  it("ouvre la requête liée, qui renvoie au contrat (C7)", async () => {
    const { core } = await setup(true);
    await openContract(app as AppWrapper);
    await click(app as AppWrapper, ".contract__actions button", "Créer les requêtes");

    await app?.get('[aria-label="Ouvrir la requête de GET /pets"]').trigger("click");
    await flushPromises();
    const chip = app?.get(".editor__contract");
    expect(chip?.text()).toBe("Contrat : Petstore · GET /pets");

    await click(app as AppWrapper, ".editor__tab", "Vérifications");
    const selects = app?.findAll(".assertion select") ?? [];
    expect((selects.at(-1)?.element as HTMLSelectElement).value).toBe("matchesContract");
    expect(app?.findAll(".assertion__whole").map((s) => s.text())).toEqual(["La réponse"]);

    await chip?.trigger("click");
    await flushPromises();
    expect(app?.get(".contract__name").text()).toBe("Petstore");
    expect(core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual({
      content: { kind: "contract", projectId: BOUTIQUE_TESTS, path: "contracts/petstore.yaml" },
    });
  });

  it("exécute les requêtes du contrat (C10)", async () => {
    const { core } = await setup(true);
    await openContract(app as AppWrapper);
    const run = app
      ?.findAll(".contract__actions button")
      .find((b) => b.text().includes("Exécuter"));
    expect(run?.attributes("disabled")).toBeDefined();

    await click(app as AppWrapper, ".contract__actions button", "Créer les requêtes");
    await click(app as AppWrapper, ".contract__actions button", "▶ Exécuter (3)");

    expect(core.calls.find((c) => c.cmd === "contract_run")?.args).toEqual({
      project: BOUTIQUE_TESTS,
      path: "contracts/petstore.yaml",
      environment: "Local",
      confirmed: false,
    });
    expect(app?.get(".run__title").text()).toContain("Run du contrat « Petstore »");
    expect(app?.findAll(".run li")).toHaveLength(3);
  });
});

describe("source et suppression (C9, C11)", () => {
  it("enregistre une source lisible, refuse une source illisible", async () => {
    const { core } = await setup(true);
    await openContract(app as AppWrapper);
    await click(app as AppWrapper, ".contract__tab", "Source");
    const source = app?.get(".contract__source");

    await source?.setValue("pas un contrat");
    expect(app?.get('.contract__tab[aria-selected="true"]').text()).toContain("●");
    await click(app as AppWrapper, ".contract__actions button", "Enregistrer");
    expect(app?.get(".contract__panel [role='alert']").text()).toContain("pas un contrat lisible");

    await source?.setValue(PETSTORE_SOURCE.replace("1.0.0", "1.1.0"));
    await source?.trigger("keydown", { key: "s", ctrlKey: true });
    await flushPromises();
    expect(core.calls.filter((c) => c.cmd === "contract_save")).toHaveLength(2);
    expect(app?.find(".contract__panel [role='alert']").exists()).toBe(false);
    expect(app?.get(".contract__version").text()).toBe("version 1.1.0");
    expect(app?.get(".contract-row__version").text()).toBe("1.1.0");
  });

  it("une source illisible n'est pas perdue à la fermeture de l'onglet", async () => {
    await setup(true);
    await openContract(app as AppWrapper);
    await click(app as AppWrapper, ".contract__tab", "Source");
    await app?.get(".contract__source").setValue("pas un contrat");

    await app?.get('[aria-label="Fermer Petstore"]').trigger("click");
    await flushPromises();
    expect(app?.get(".toast").text()).toContain("Source du contrat non enregistrée");
    expect(app?.find(".contract").exists()).toBe(true);
  });

  it("met un contrat à la corbeille et ferme son onglet", async () => {
    const { core } = await setup(true);
    await openContract(app as AppWrapper);

    await app?.get('[aria-label="Actions pour le contrat Petstore"]').trigger("click");
    await flushPromises();
    await click(app as AppWrapper, '[role="menuitem"]', "Mettre à la corbeille");

    expect(core.calls.find((c) => c.cmd === "contract_delete")?.args).toMatchObject({
      permanent: false,
    });
    expect(app?.find(".contract").exists()).toBe(false);
    expect(app?.find(".contract-row").exists()).toBe(false);
    expect(app?.get(".toast").text()).toContain("Contrat « Petstore » mis à la corbeille.");
  });
});

describe("au clavier et entre fenêtres", () => {
  it("le menu « + Contrat » s'ouvre et se parcourt au clavier", async () => {
    await setup();
    const trigger = app?.get('[aria-label="Ajouter un contrat à boutique-tests"]');
    await trigger?.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const items = app?.findAll('[role="menuitem"]').map((i) => i.text()) ?? [];
    expect(items.slice(0, 4).map((t) => t.split("OpenAPI")[0]?.trim())).toEqual([
      "Importer un fichier…",
      "Coller un contrat…",
      "Depuis une adresse…",
      "Nouveau contrat…",
    ]);
    expect(document.activeElement?.textContent).toContain("Importer un fichier…");
  });

  it("suit les contrats ajoutés dans une autre fenêtre (F11)", async () => {
    const { core } = await setup();
    core.contracts.files.set("contracts/petstore.yaml", {
      name: "Petstore",
      version: "1.0.0",
      source: PETSTORE_SOURCE,
      operations: structuredClone(PETSTORE_OPERATIONS),
    });
    await core.emit(EVENTS.project, {
      projectId: BOUTIQUE_TESTS,
      scope: "contracts",
      path: "contracts/petstore.yaml",
      origin: "fenetre-autre",
    });
    await flushPromises();
    expect(app?.get(".contract-row").text()).toContain("Petstore");
  });
});
