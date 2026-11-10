/**
 * Sélecteur de workspace (docs/plan-selecteur-workspace.md, S1 à S8) : changer, créer,
 * renommer, colorer, supprimer, gérer et glisser un projet, à la souris et au clavier.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { useNavigationStore } from "@/stores/navigation";

import { type FakeCore, installFakeCore } from "../fakeCore";
import {
  BOUTIQUE,
  BOUTIQUE_TESTS,
  MES_PROJETS,
  boutiqueSnapshot,
  project,
  workspace,
} from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import {
  chooseSwitcherAction,
  chooseWorkspace,
  currentWorkspace,
  isSwitcherOpen,
  openSwitcher,
  switcherItem,
  switcherItems,
  switcherNames,
} from "../switcher";

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function press(key: string, options: KeyboardEventInit = {}): Promise<void> {
  (document.activeElement ?? document.body).dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, ...options }),
  );
  await flushPromises();
}

function lastArgs(core: FakeCore, cmd: string) {
  return core.calls.filter((c) => c.cmd === cmd).at(-1)?.args;
}

function trigger(app: AppWrapper): HTMLElement {
  return app.get(".switcher__trigger").element as HTMLElement;
}

function focused(): string {
  return document.activeElement?.querySelector(".switcher__name")?.textContent.trim() ?? "";
}

function actionLabels(app: AppWrapper): string[] {
  return app.findAll('.switcher__context [data-level="0"]').map((li) => li.text());
}

async function chooseAction(app: AppWrapper, label: string): Promise<void> {
  const item = app
    .findAll('.switcher__context [role^="menuitem"]')
    .find((li) => li.text().includes(label));
  if (!item) throw new Error(`action « ${label} » introuvable`);
  await item.trigger("click");
  await flushPromises();
}

/** Faux `DataTransfer` : happy-dom ne fournit pas de glisser-déposer complet. */
function dataTransfer() {
  const data = new Map<string, string>();
  return {
    get types() {
      return [...data.keys()];
    },
    setData: (type: string, value: string) => data.set(type, value),
    getData: (type: string) => data.get(type) ?? "",
    effectAllowed: "none",
    dropEffect: "none",
  };
}

function dispatchDrag(element: Element, type: string, transfer: ReturnType<typeof dataTransfer>) {
  const event = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(event, "dataTransfer", { value: transfer });
  element.dispatchEvent(event);
  return event;
}

function richSnapshot() {
  const snapshot = boutiqueSnapshot();
  snapshot.projects.push(project("p-notifications", "notifications"));
  snapshot.workspaces.push(workspace("ws-recette", "Recette sprint 42", { color: "green" }));
  return snapshot;
}

describe("sélecteur de workspace (S1, S2, S8)", () => {
  it("montre le workspace ouvert ; plus de rangée d'onglets de workspaces", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();

    expect(currentWorkspace(wrapper)).toBe("Boutique");
    expect(wrapper.find('[role="tablist"][aria-label="Workspaces"]').exists()).toBe(false);
    // En tête de la colonne de la barre latérale, au-dessus de l'« aside ».
    expect(wrapper.find(".shell__sidebar > .shell__switcher .switcher").exists()).toBe(true);
    expect(trigger(wrapper).getAttribute("aria-haspopup")).toBe("menu");
    expect(trigger(wrapper).getAttribute("aria-expanded")).toBe("false");
    expect(trigger(wrapper).textContent).toContain("Workspace : Boutique");
  });

  it("le menu liste « Tous les projets », puis les workspaces avec projets et raccourcis", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await openSwitcher(wrapper);

    expect(trigger(wrapper).getAttribute("aria-expanded")).toBe("true");
    expect(
      switcherItems(wrapper).map((item) =>
        [".switcher__name", ".switcher__count", ".switcher__key"].map((part) =>
          item.get(part).text(),
        ),
      ),
    ).toEqual([
      ["Tous les projets", "3 projets", "Ctrl+0"],
      ["Mes projets", "2 projets", "Ctrl+1"],
      ["Boutique", "2 projets", "Ctrl+2"],
      ["Recette sprint 42", "0 projet", "Ctrl+3"],
    ]);
    // Le workspace ouvert est coché (et marqué ✓, décoratif).
    expect(
      switcherItem(wrapper, "Boutique").get(".switcher__check").attributes("aria-hidden"),
    ).toBe("true");
    expect(switcherItems(wrapper).map((item) => item.attributes("aria-checked"))).toEqual([
      "false",
      "false",
      "true",
      "false",
    ]);
    expect(wrapper.get(".switcher__heading").text()).toBe("Workspaces · 3");
    // Le focus est sur le workspace ouvert.
    expect(focused()).toBe("Boutique");
  });

  it("un clic change de workspace, le mémorise (R7) et referme le menu", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await chooseWorkspace(wrapper, "Recette sprint 42");

    expect(lastArgs(core, "workspace_open")).toEqual({ workspaceId: "ws-recette" });
    expect(currentWorkspace(wrapper)).toBe("Recette sprint 42");
    expect(isSwitcherOpen(wrapper)).toBe(false);
    expect(document.activeElement).toBe(trigger(wrapper));
    expect(wrapper.get("main h1").text()).toBe("Recette sprint 42");
    expect(useNavigationStore().titleSubject).toBe("Recette sprint 42");
  });

  it("« Tous les projets » : sélecteur dans une rangée en haut, sans barre latérale", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await chooseWorkspace(wrapper, "Tous les projets");

    expect(lastArgs(core, "workspace_open")).toEqual({ workspaceId: null });
    expect(core.snapshot().lastWorkspaceId).toBeNull();
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 3");
    expect(wrapper.find("aside").exists()).toBe(false);
    expect(wrapper.find(".shell__switcher--top .switcher").exists()).toBe(true);
    expect(currentWorkspace(wrapper)).toBe("Tous les projets");
  });

  it("clavier : ↓ ouvre, flèches, Début, Fin, Entrée ; Échap rend le focus", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    trigger(wrapper).focus();

    await press("ArrowDown");
    expect(focused()).toBe("Boutique");
    await press("ArrowDown");
    expect(focused()).toBe("Recette sprint 42");
    await press("Home");
    expect(focused()).toBe("Tous les projets");
    await press("End");
    expect(document.activeElement?.textContent).toContain("Gérer les workspaces…");
    await press("ArrowDown");
    expect(focused()).toBe("Tous les projets");
    await press("Escape");
    expect(isSwitcherOpen(wrapper)).toBe(false);
    expect(document.activeElement).toBe(trigger(wrapper));

    await press("ArrowUp");
    expect(document.activeElement?.textContent).toContain("Gérer les workspaces…");
    await press("ArrowUp");
    await press("ArrowUp");
    expect(focused()).toBe("Recette sprint 42");
    await press("Enter");
    expect(currentWorkspace(wrapper)).toBe("Recette sprint 42");
    expect(document.activeElement).toBe(trigger(wrapper));
  });

  it("Ctrl+1…9 ouvre un workspace, Ctrl+0 « Tous les projets » (touche physique)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    // Sur un clavier AZERTY, Ctrl+1 produit « & » : c'est le code de la touche qui compte.
    await press("&", { code: "Digit1", ctrlKey: true });
    expect(currentWorkspace(wrapper)).toBe("Mes projets");
    await press("0", { code: "Numpad0", ctrlKey: true });
    expect(currentWorkspace(wrapper)).toBe("Tous les projets");
    await press("9", { code: "Digit9", ctrlKey: true });
    expect(currentWorkspace(wrapper)).toBe("Tous les projets");
    expect(lastArgs(core, "workspace_open")).toEqual({ workspaceId: null });
  });
});

describe("créer et gérer depuis le sélecteur (S3, S5)", () => {
  it("« + Nouveau workspace » : la fenêtre s'ouvre sous le sélecteur, le workspace créé s'ouvre", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await chooseSwitcherAction(wrapper, "Nouveau workspace");
    const popover = wrapper.get('[role="dialog"]');
    expect(popover.get("h2").text()).toBe("Nouveau workspace");
    expect(popover.text()).toContain(
      "Il s'ajoute au sélecteur, vide : glissez-y des projets depuis « Tous les projets ».",
    );
    expect(document.activeElement).toBe(popover.get("input").element);

    await popover.get("input").setValue("boutique");
    await popover.get("form").trigger("submit");
    await flushPromises();
    expect(popover.get("[role='alert']").text()).toBe("Un workspace « Boutique » existe déjà.");

    await popover.get("input").setValue("Données de référence");
    await popover.get("form").trigger("submit");
    await flushPromises();

    expect(lastArgs(core, "workspace_create")).toEqual({ name: "Données de référence" });
    expect(wrapper.find('[role="dialog"]').exists()).toBe(false);
    expect((await switcherNames(wrapper)).at(-1)).toBe("Données de référence");
    expect(currentWorkspace(wrapper)).toBe("Données de référence");
    expect(wrapper.get("main").text()).toContain("Aucun projet pour l'instant");
    expect(wrapper.get(".toast").text()).toBe("Workspace « Données de référence » créé.");
  });

  it("Échap ferme « Nouveau workspace » sans rien créer et rend le focus au sélecteur", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await chooseSwitcherAction(wrapper, "Nouveau workspace");

    await press("Escape");

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false);
    expect(document.activeElement).toBe(trigger(wrapper));
    expect(core.commands()).not.toContain("workspace_create");
  });

  it("« Gérer les workspaces… » et Ctrl+Maj+W ouvrent la gestion sur le workspace ouvert", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await chooseSwitcherAction(wrapper, "Gérer les workspaces…");
    expect(wrapper.get("main h1").text()).toBe("Workspaces");
    expect(wrapper.get<HTMLInputElement>("main form input").element.value).toBe("Boutique");

    await wrapper
      .get("main")
      .findAll("button")
      .find((b) => b.text().includes("Retour"))
      ?.trigger("click");
    await flushPromises();
    await press("W", { code: "KeyW", ctrlKey: true, shiftKey: true });
    expect(wrapper.get("main h1").text()).toBe("Workspaces");
  });
});

describe("actions d'un workspace (S4)", () => {
  it("clic droit : renommer, couleur, gérer, supprimer ; « Tous les projets » n'en a pas", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);

    await switcherItem(wrapper, "Boutique").trigger("contextmenu");
    await flushPromises();

    expect(actionLabels(wrapper)).toEqual([
      "RenommerF2",
      "Changer la couleur▸",
      "Gérer les workspaces…",
      "Supprimer « Boutique »Les projets restent dans « Tous les projets ».Suppr",
    ]);
    // Échap ferme les actions et rend le focus à la ligne ; le sélecteur reste ouvert.
    await press("Escape");
    expect(wrapper.findAll(".switcher__context [role='menu']")).toHaveLength(0);
    expect(isSwitcherOpen(wrapper)).toBe(true);
    expect(focused()).toBe("Boutique");

    await switcherItem(wrapper, "Tous les projets").trigger("contextmenu");
    await flushPromises();
    expect(wrapper.findAll(".switcher__context [role='menu']")).toHaveLength(0);
  });

  it("« ⋯ » au survol ouvre les mêmes actions", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);

    await switcherItem(wrapper, "Mes projets").get(".switcher__more").trigger("click");
    await flushPromises();

    expect(actionLabels(wrapper)[3]).toContain("Supprimer « Mes projets »");
  });

  it("Renommer ouvre une fenêtre : nom pris refusé sous le champ, puis enregistré", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);
    await switcherItem(wrapper, "Boutique").trigger("contextmenu");
    await flushPromises();
    await chooseAction(wrapper, "Renommer");

    const dialog = wrapper.get("dialog");
    expect(dialog.get("h2").text()).toBe("Renommer le workspace");
    const input = dialog.get<HTMLInputElement>("input");
    expect(input.element.value).toBe("Boutique");
    await input.setValue("mes projets");
    await dialog.get("form").trigger("submit");
    await flushPromises();
    expect(dialog.get("[role='alert']").text()).toBe("Un workspace « Mes projets » existe déjà.");

    await input.setValue("E-commerce");
    await dialog.get("form").trigger("submit");
    await flushPromises();
    expect(lastArgs(core, "workspace_rename")).toEqual({
      workspaceId: BOUTIQUE,
      name: "E-commerce",
    });
    expect(currentWorkspace(wrapper)).toBe("E-commerce");
  });

  it("F2 sur un workspace du menu ouvre le renommage", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    trigger(wrapper).focus();
    await press("ArrowDown");

    await press("F2");

    expect(isSwitcherOpen(wrapper)).toBe(false);
    expect(wrapper.get<HTMLInputElement>("dialog input").element.value).toBe("Boutique");
  });

  it("Maj+F10 ouvre les actions ; le sous-menu change la couleur", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    trigger(wrapper).focus();
    await press("ArrowDown");

    await press("F10", { shiftKey: true });
    expect(document.activeElement?.textContent).toContain("Renommer");
    await press("ArrowDown");
    await press("ArrowRight");
    const colors = wrapper.findAll('.switcher__context [data-level="1"]');
    expect(colors.map((c) => c.text())).toContain("✓Orange");
    await colors.find((c) => c.text().includes("Turquoise"))?.trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_set_color")).toEqual({ workspaceId: BOUTIQUE, color: "teal" });
  });

  it("supprime sans confirmation, revient sur « Tous les projets » ; « Annuler » le rétablit (R8)", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await openSwitcher(wrapper);
    await switcherItem(wrapper, "Boutique").trigger("contextmenu");
    await flushPromises();
    await chooseAction(wrapper, "Supprimer « Boutique »");

    expect(core.commands()).toContain("workspace_delete");
    expect(await switcherNames(wrapper)).toEqual(["Tous les projets", "Mes projets"]);
    expect(currentWorkspace(wrapper)).toBe("Tous les projets");
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "Workspace « Boutique » supprimé · 2 projets conservés dans « Tous les projets ».",
    );

    await toast
      .findAll("button")
      .find((b) => b.text() === "Annuler")
      ?.trigger("click");
    await flushPromises();
    expect(await switcherNames(wrapper)).toEqual(["Tous les projets", "Mes projets", "Boutique"]);
    expect(currentWorkspace(wrapper)).toBe("Boutique");
  });

  it("Suppr sur un workspace du menu le supprime", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    trigger(wrapper).focus();
    await press("ArrowDown");
    await press("ArrowUp");
    expect(focused()).toBe("Mes projets");

    await press("Delete");

    expect(lastArgs(core, "workspace_delete")).toEqual({ workspaceId: MES_PROJETS });
    expect(currentWorkspace(wrapper)).toBe("Boutique");
  });
});

describe("glisser un projet sur le sélecteur (S6)", () => {
  async function dragOverSwitcher(app: AppWrapper, projectName: string) {
    await chooseWorkspace(app, "Tous les projets");
    const row = app.findAll("main tbody tr").find((r) => r.find("th").text() === projectName);
    if (!row) throw new Error(`ligne « ${projectName} » introuvable`);
    const transfer = dataTransfer();
    dispatchDrag(row.element, "dragstart", transfer);
    dispatchDrag(app.get(".switcher__trigger").element, "dragenter", transfer);
    await flushPromises();
    return transfer;
  }

  it("le sélecteur s'ouvre ; le dépôt sur un workspace ajoute le projet et le notifie", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    const transfer = await dragOverSwitcher(wrapper, "notifications");
    expect(isSwitcherOpen(wrapper)).toBe(true);
    expect(wrapper.get(".switcher__hint").text()).toBe("Déposez le projet sur un workspace.");

    const target = switcherItem(wrapper, "Boutique").element;
    const over = dispatchDrag(target, "dragover", transfer);
    await flushPromises();
    expect(over.defaultPrevented).toBe(true);
    expect(switcherItem(wrapper, "Boutique").classes()).toContain("switcher__item--drop");
    expect(wrapper.get(".switcher__hint").text()).toBe("Ajouter à « Boutique »");

    dispatchDrag(target, "drop", transfer);
    await flushPromises();

    expect(lastArgs(core, "workspace_add_projects")).toEqual({
      workspaceId: BOUTIQUE,
      projectIds: ["p-notifications"],
    });
    expect(isSwitcherOpen(wrapper)).toBe(false);
    expect(wrapper.get(".toast").text()).toBe("« notifications » ajouté à « Boutique ».");
  });

  it("un projet déjà présent n'est pas ajouté deux fois ; la fin du glisser referme", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    const transfer = await dragOverSwitcher(wrapper, "boutique-tests");
    dispatchDrag(switcherItem(wrapper, "Boutique").element, "drop", transfer);
    await flushPromises();
    expect(core.commands()).not.toContain("workspace_add_projects");
    expect(wrapper.get(".toast").text()).toBe("« boutique-tests » est déjà dans « Boutique ».");

    await dragOverSwitcher(wrapper, "notifications");
    expect(isSwitcherOpen(wrapper)).toBe(true);
    document.dispatchEvent(new Event("dragend"));
    await flushPromises();
    expect(isSwitcherOpen(wrapper)).toBe(false);
  });

  it("un glisser étranger (texte, fichier) est ignoré", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const transfer = dataTransfer();
    transfer.setData("text/plain", BOUTIQUE_TESTS);

    const over = dispatchDrag(wrapper.get(".switcher__trigger").element, "dragover", transfer);
    await flushPromises();

    expect(over.defaultPrevented).toBe(false);
    expect(isSwitcherOpen(wrapper)).toBe(false);
  });
});
