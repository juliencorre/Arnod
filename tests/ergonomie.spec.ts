/**
 * Lot J (docs/plan-qualite-mvp.md, T29 à T31) : ergonomie et clavier. Chaque test est la preuve
 * d'un constat de l'audit (docs/audit-2026-11-mvp.md).
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { onTablistKeydown } from "@/lib/tablist";
import { AUTOSAVE_MS } from "@/stores/workbench";

import { componentPaths, cssValue, rawSource } from "./css";

import { PETSTORE_OPERATIONS, PETSTORE_SOURCE } from "./fakeContracts";
import { newRequest, okOutcome } from "./fakeWorkbench";
import { installFakeCore } from "./fakeCore";
import { firstRunSnapshot } from "./fixtures";
import { type AppWrapper, mountApp } from "./mountApp";
import { click, openProject } from "./workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

/** Une liste d'onglets « à la Vue » : cliquer choisit, le choisi seul est dans l'ordre de tabulation. */
function tablist(
  labels: string[],
  vertical = false,
): { list: HTMLElement; selected: () => string } {
  const list = document.createElement("div");
  list.setAttribute("role", "tablist");
  if (vertical) list.setAttribute("aria-orientation", "vertical");
  let chosen = labels[0] ?? "";
  const render = (): void => {
    for (const tab of list.children) {
      const on = tab.textContent === chosen;
      tab.setAttribute("aria-selected", String(on));
      tab.setAttribute("tabindex", on ? "0" : "-1");
    }
  };
  for (const label of labels) {
    const tab = document.createElement("button");
    tab.setAttribute("role", "tab");
    tab.textContent = label;
    tab.addEventListener("click", () => {
      chosen = label;
      render();
    });
    list.append(tab);
  }
  render();
  list.addEventListener("keydown", onTablistKeydown);
  document.body.append(list);
  return { list, selected: () => chosen };
}

function press(list: HTMLElement, key: string): void {
  const focused = document.activeElement;
  const from =
    focused instanceof HTMLElement && list.contains(focused) ? focused : list.children[0];
  from?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

describe("onglets au clavier (AC3)", () => {
  it("flèches, Début et Fin choisissent l'onglet voisin et lui donnent le focus", () => {
    const { list, selected } = tablist(["Paramètres", "En-têtes", "Corps"]);
    (list.children[0] as HTMLElement).focus();
    press(list, "ArrowRight");
    expect(selected()).toBe("En-têtes");
    expect(document.activeElement?.textContent).toBe("En-têtes");
    press(list, "End");
    expect(selected()).toBe("Corps");
    press(list, "ArrowRight");
    expect(selected()).toBe("Paramètres");
    press(list, "ArrowLeft");
    expect(selected()).toBe("Corps");
    press(list, "Home");
    expect(selected()).toBe("Paramètres");
    list.remove();
  });

  it("une liste verticale suit haut et bas, pas gauche et droite", () => {
    const { list, selected } = tablist(["Un", "Deux"], true);
    (list.children[0] as HTMLElement).focus();
    press(list, "ArrowRight");
    expect(selected()).toBe("Un");
    press(list, "ArrowDown");
    expect(selected()).toBe("Deux");
    list.remove();
  });
});

describe("onglets de l'éditeur et de la réponse (AC3)", () => {
  it("un seul arrêt de tabulation par liste, et la flèche change d'onglet", async () => {
    ({ app } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    await click(app, "button", "Envoyer");
    await flushPromises();
    for (const label of ["Sections de la requête", "Contenu de la réponse"]) {
      const tabs = app.findAll(`[role="tablist"][aria-label="${label}"] [role="tab"]`);
      expect(tabs.length).toBeGreaterThan(2);
      expect(tabs.filter((t) => t.attributes("tabindex") === "0")).toHaveLength(1);
      const before = tabs.findIndex((t) => t.attributes("aria-selected") === "true");
      const selected = tabs[before];
      if (!selected) throw new Error(`aucun onglet choisi dans ${label}`);
      await selected.trigger("keydown", { key: "ArrowRight" });
      const after = app
        .findAll(`[role="tablist"][aria-label="${label}"] [role="tab"]`)
        .findIndex((t) => t.attributes("aria-selected") === "true");
      expect(after).toBe(before + 1);
    }
  });
});

describe("« Annuler » d'un copier ou déplacer (IN7)", () => {
  it("un refus du cœur s'affiche en notification d'erreur", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.folders.add("commandes");
      c.workbench.requests.set("sans-coupon.yml", newRequest("Sans coupon", { method: "POST" }));
    }));
    const item = app
      .findAll('li[role="presentation"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("Sans coupon"));
    if (!item) throw new Error("ligne introuvable");
    await item.get(".row__more").trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Copier ou déplacer…");
    await app.get(".copy-move__search").setValue("commandes");
    await flushPromises();
    await click(app, "dialog[open] button", "Déplacer");
    core.failNext("transfer_undo", {
      kind: "undoExpired",
      message: "Cette copie ou ce déplacement ne peut plus être annulé.",
    });
    await click(app, ".toast button", "Annuler");
    await flushPromises();
    expect(app.get(".toast").text()).toContain("ne peut plus être annulé");
    expect(app.get(".toast").classes()).toContain("toast--error");
  });
});

describe("garde des actions asynchrones (IN8)", () => {
  it("« Ignorer pour ce résultat » et « Proposer des assertions » disent le refus du cœur", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.nextSend = okOutcome({
        passed: false,
        assertions: [
          {
            id: "statut",
            target: "status",
            passed: false,
            message: "attendu 200, reçu 500",
            actual: "500",
            ignored: false,
          },
        ],
      });
    }));
    await click(app, '[role="treeitem"]', "Lister");
    await click(app, "button", "Envoyer");
    await flushPromises();
    core.failNext("result_ignore_check", {
      kind: "notFound",
      message: "Ce résultat n'existe plus.",
    });
    await click(app, ".failure__link", "Ignorer pour ce résultat");
    await flushPromises();
    expect(app.get(".toast").text()).toContain("Ce résultat n'existe plus.");
    await click(app, ".response__tab", "Assertions");
    core.failNext("request_propose_assertions", {
      kind: "invalidRequest",
      message: "Corps illisible.",
    });
    await click(app, "button", "Proposer des assertions");
    await flushPromises();
    expect(app.get(".toast").text()).toContain("Corps illisible.");
  });
});

describe("erreur d'envoi avec son action (ER6)", () => {
  it("une variable inconnue propose « Ajouter » et « Environnements… »", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.requests.set(
        "Clients/lister.yml",
        newRequest("Lister", { url: "{{apiUrl}}/clients" }),
      );
    }));
    await click(app, '[role="treeitem"]', "Lister");
    await new Promise((resolve) => setTimeout(resolve, 300));
    await flushPromises();
    core.failNext("request_send", {
      kind: "unknownVariable",
      message: "Variable « apiUrl » absente de l'environnement Local.",
    });
    await click(app, "button", "Envoyer");
    await flushPromises();
    const error = app.get(".response__error");
    expect(error.get(".response__error-title").text()).toContain("Requête non envoyée");
    const actions = error.findAll(".response__error-actions button").map((b) => b.text());
    expect(actions).toEqual([expect.stringMatching(/^Ajouter/) as unknown, "Environnements…"]);
    await click(app, ".response__error-actions button", "Ajouter");
    await flushPromises();
    const saved = core.calls.filter((c) => c.cmd === "environment_save").at(-1);
    expect(JSON.stringify(saved?.args)).toContain("apiUrl");
    await click(app, ".response__error-actions button", "Environnements…");
    expect(app.find("dialog[open]").exists()).toBe(true);
  });

  it("une autre erreur propose « Réessayer », qui renvoie la requête", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[role="treeitem"]', "Lister");
    core.failNext("request_send", { kind: "io", message: "Disque plein." });
    await click(app, "button", "Envoyer");
    await flushPromises();
    const sends = (): number => core.calls.filter((c) => c.cmd === "request_send").length;
    const before = sends();
    await click(app, ".response__error-actions button", "Réessayer");
    await flushPromises();
    expect(sends()).toBe(before + 1);
    expect(app.find(".response__error").exists()).toBe(false);
  });
});

describe("« Annuler » des suppressions (ER1)", () => {
  async function menu(label: string, entry: string): Promise<void> {
    if (!app) throw new Error("application absente");
    await app.get(`[aria-label="${label}"]`).trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', entry);
    await flushPromises();
  }

  it("un dossier mis à la corbeille revient par « Annuler »", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.folders.add("commandes");
      c.workbench.requests.set("commandes/creer.yml", newRequest("Créer", { method: "POST" }));
    }));
    const item = app
      .findAll('li[role="presentation"]')
      .find((li) => li.get('[role="treeitem"]').text().includes("commandes"));
    if (!item) throw new Error("dossier introuvable");
    await item.get(".row__more").trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Mettre à la corbeille");
    expect(core.workbench.requests.has("commandes/creer.yml")).toBe(false);
    expect(app.get(".toast").text()).toContain("« commandes » mis à la corbeille.");
    await click(app, ".toast button", "Annuler");
    await flushPromises();
    expect(core.workbench.requests.has("commandes/creer.yml")).toBe(true);
    expect(app.text()).toContain("commandes");
  });

  it("un contrat mis à la corbeille revient par « Annuler »", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.contracts.files.set("contracts/petstore.yaml", {
        name: "Petstore",
        version: "1.0.0",
        source: PETSTORE_SOURCE,
        operations: structuredClone(PETSTORE_OPERATIONS),
      });
    }));
    await menu("Actions pour le contrat Petstore", "Mettre à la corbeille");
    expect(app.find(".contract-row").exists()).toBe(false);
    await click(app, ".toast button", "Annuler");
    await flushPromises();
    expect(core.contracts.files.has("contracts/petstore.yaml")).toBe(true);
    expect(app.find(".contract-row").exists()).toBe(true);
  });

  it("un environnement supprimé revient par « Annuler »", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.envs.push({
        name: "Recette",
        color: "blue",
        protected: false,
        variables: [{ name: "baseUrl", value: "https://recette" }],
        locals: [],
      });
    }));
    await menu("Plus d'actions sur boutique-tests", "Gérer les environnements…");
    await click(app, "dialog[open] .envs__item", "Recette");
    expect(app.get("dialog[open] .envs__item[aria-current='true'] .envs__item-name").text()).toBe(
      "Recette",
    );
    await click(app, "dialog[open] button", "Supprimer");
    expect(core.workbench.envs.map((e) => e.name)).toEqual(["Local"]);
    expect(app.get(".toast").text()).toContain("Environnement « Recette » supprimé.");
    await click(app, ".toast button", "Annuler");
    await flushPromises();
    expect(core.workbench.envs.map((e) => e.name)).toEqual(["Local", "Recette"]);
    expect(app.findAll("dialog[open] .envs__item-name").map((b) => b.text())).toContain("Recette");
  });
});

describe("mouvement réduit (AC8)", () => {
  it("les transitions deviennent quasi immédiates quand le système le demande", () => {
    expect(cssValue("src/styles.css", "*", "transition-duration")).toBe("0.01ms !important");
    expect(cssValue("src/styles.css", "*", "animation-duration")).toBe("0.01ms !important");
  });
});

describe("cibles (AC4)", () => {
  it("« TEST », le retrait d'une étiquette et le × d'une ligne font 24 × 24 px au moins", () => {
    for (const [file, selector] of [
      ["src/components/workbench/StepBar.vue", ".steps__test"],
      ["src/components/workbench/TagsEditor.vue", ".tag__remove"],
      ["src/components/ui/RowRemoveButton.vue", ".row-remove"],
    ] as const) {
      expect(cssValue(file, selector, "min-width")).toBe("1.5rem");
      expect(cssValue(file, selector, "min-height")).toBe("1.5rem");
    }
  });
});

describe("une forme d'ajout et une de retrait de ligne (ER8)", () => {
  const EDITORS = [
    "src/components/workbench/PairsEditor.vue",
    "src/components/workbench/AssertionsEditor.vue",
    "src/components/workbench/VariablesEditor.vue",
    "src/components/workbench/EnvironmentsDialog.vue",
    "src/components/workbench/ExpectedResponses.vue",
  ];

  it("chaque éditeur de lignes retire par RowRemoveButton et ajoute par un BaseButton « + Nom »", () => {
    for (const file of EDITORS) {
      const source = rawSource(file);
      expect(source, file).toContain("<RowRemoveButton");
      expect(source, file).toMatch(/<BaseButton[^>]*>\s*(\{\{ addLabel[^}]*\}\}|\+ \S)/);
      // Plus de ✕ écrit à la main ni de lien « + Ajouter … » pour une ligne.
      expect(source, file).not.toMatch(/>\s*✕\s*</);
      expect(source, file).not.toContain("+ Ajouter une");
    }
  });

  it("aucun composant n'écrit plus de bouton de retrait ✕ seul", () => {
    const loose = componentPaths().filter((file) =>
      /<button[^>]*aria-label="`Supprimer[^>]*>\s*✕\s*<\/button>/.test(rawSource(file)),
    );
    expect(loose).toEqual([]);
  });
});

describe("focus rendu (AC2)", () => {
  /** Le focus est sur un vrai contrôle, pas retombé sur `body`. */
  function focused(): Element | null {
    const active = document.activeElement;
    return active === document.body ? null : active;
  }

  function blur(): void {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  }

  it("Échap dans Environnements rend le focus à la pastille qui a ouvert le menu", async () => {
    ({ app } = await openProject());
    const chip = app.get<HTMLButtonElement>(
      '[aria-label="Environnement de boutique-tests : Local"]',
    );
    chip.element.focus();
    await chip.trigger("click");
    await flushPromises();
    await click(app, '[role="menuitem"]', "Gérer les environnements…");
    await flushPromises();
    const dialog = app.get("dialog[open]");
    (dialog.element as HTMLElement).querySelector("input")?.focus();
    await dialog.trigger("cancel");
    await flushPromises();
    expect(app.find("dialog[open]").exists()).toBe(false);
    expect(focused()).toBe(chip.element);
  });

  it("après ▶, le focus va à l'onglet du run", async () => {
    ({ app } = await openProject());
    blur();
    await click(app, '[aria-label^="Exécuter 3 requêtes sur Local"]');
    await flushPromises();
    expect(focused()?.getAttribute("role")).toBe("tab");
    expect(focused()?.getAttribute("aria-selected")).toBe("true");
  });

  it("après un essai, le focus va à l'onglet de l'essai", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await app.get(".try__input").setValue("https://api.boutique.dev/products");
    await flushPromises();
    blur();
    await app.get(".try__form").trigger("submit");
    await flushPromises();
    expect(focused()?.getAttribute("role")).toBe("tab");
    expect(focused()?.textContent).toContain("Essai 1");
  });

  it("après la création d'un projet, le focus reste dans la zone principale", async () => {
    installFakeCore(firstRunSnapshot());
    app = await mountApp();
    await app.get(".first-run__card form input").setValue("boutique-tests");
    await flushPromises();
    blur();
    await app.get(".first-run__card form").trigger("submit");
    await flushPromises();
    expect(focused()?.closest("main")).not.toBeNull();
  });
});

describe("environnements et réglages enregistrés seuls (ER2)", () => {
  const settle = async (): Promise<void> => {
    await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
    await flushPromises();
  };

  async function openEnvironments(): Promise<void> {
    if (!app) throw new Error("application absente");
    await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
    await click(app, '[role="menuitem"]', "Gérer les environnements…");
  }

  it("une variable modifiée s'enregistre seule, sans bouton « Enregistrer »", async () => {
    let core;
    ({ app, core } = await openProject());
    await openEnvironments();
    expect(app.findAll("dialog[open] button").map((b) => b.text())).not.toContain("Enregistrer");
    await app.get('dialog[open] input[aria-label="Valeur de la variable 1"]').setValue("https://x");
    expect(app.get("dialog[open] .save-state").text()).toBe("Modifications en attente…");
    await settle();
    expect(core.workbench.envs[0]?.variables[0]?.value).toBe("https://x");
    expect(app.get("dialog[open] .save-state").text()).toBe("✓ Enregistré");
  });

  it("Échap ne jette rien : la modification en attente est enregistrée avant de fermer", async () => {
    let core;
    ({ app, core } = await openProject());
    await openEnvironments();
    await app.get('dialog[open] input[aria-label="Valeur de la variable 1"]').setValue("https://y");
    await app.get("dialog[open]").trigger("cancel");
    await flushPromises();
    expect(core.workbench.envs[0]?.variables[0]?.value).toBe("https://y");
    expect(app.find("dialog[open]").exists()).toBe(false);
  });

  it("un enregistrement refusé garde la fenêtre ouverte et le dit", async () => {
    let core;
    ({ app, core } = await openProject());
    await openEnvironments();
    await app.get('dialog[open] input[aria-label="Valeur de la variable 1"]').setValue("https://z");
    core.failNext("environment_save", { kind: "io", message: "Disque plein." });
    await app.get("dialog[open]").trigger("cancel");
    await flushPromises();
    expect(app.find("dialog[open]").exists()).toBe(true);
    expect(app.get("dialog[open] [role='alert']").text()).toContain("Disque plein.");
    expect(app.get("dialog[open] .save-state").text()).toContain("Non enregistré");
  });

  it("Échap dans les réglages du projet enregistre l'authentification modifiée", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
    await click(app, '[role="menuitem"]', "Réglages du projet…");
    await flushPromises();
    await app.get("dialog[open] select").setValue("bearer");
    await flushPromises();
    await app.get("dialog[open]").trigger("cancel");
    await flushPromises();
    const saved = core.calls.filter((c) => c.cmd === "project_auth_save").at(-1);
    expect((saved?.args.auth as { type: string } | undefined)?.type).toBe("bearer");
    expect(app.find("dialog[open]").exists()).toBe(false);
  });
});
