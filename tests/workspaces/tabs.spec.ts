import { clearMocks } from "@tauri-apps/api/mocks";
import { type DOMWrapper, flushPromises } from "@vue/test-utils";
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

let wrapper: AppWrapper | undefined;

afterEach(() => {
  wrapper?.unmount();
  wrapper = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function tabs(app: AppWrapper): DOMWrapper<Element>[] {
  return app.findAll('[role="tablist"] [role="tab"]');
}

function tab(app: AppWrapper, name: string): DOMWrapper<Element> {
  const found = tabs(app).find((t) => t.text() === name);
  if (!found) throw new Error(`onglet « ${name} » introuvable`);
  return found;
}

function selectedTab(app: AppWrapper): string | undefined {
  return tabs(app)
    .find((t) => t.attributes("aria-selected") === "true")
    ?.text();
}

async function press(key: string, options: KeyboardEventInit = {}): Promise<void> {
  document.activeElement?.dispatchEvent(
    new KeyboardEvent("keydown", { key, bubbles: true, ...options }),
  );
  await flushPromises();
}

function lastArgs(core: FakeCore, cmd: string) {
  return core.calls.filter((c) => c.cmd === cmd).at(-1)?.args;
}

/** Ouvre le menu contextuel d'un onglet par un clic droit. */
async function rightClick(app: AppWrapper, name: string): Promise<void> {
  await tab(app, name).trigger("contextmenu");
  await flushPromises();
}

function menuItems(app: AppWrapper): string[] {
  return app.findAll('.tabs__context [data-level="0"]').map((li) => li.text());
}

async function chooseMenuItem(app: AppWrapper, label: string): Promise<void> {
  const item = app
    .findAll('.tabs__context [role^="menuitem"]')
    .find((li) => li.text().includes(label));
  if (!item) throw new Error(`entrée « ${label} » introuvable`);
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

describe("onglets des workspaces (2b, O2)", () => {
  it("« Tous les projets » épinglé en tête, puis les workspaces dans l'ordre de création", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();

    expect(tabs(wrapper).map((t) => t.text())).toEqual([
      "Tous les projets",
      "Mes projets",
      "Boutique",
      "Recette sprint 42",
    ]);
    expect(selectedTab(wrapper)).toBe("Boutique");
    // Un seul onglet dans l'ordre de tabulation : l'onglet actif.
    expect(tabs(wrapper).map((t) => t.attributes("tabindex"))).toEqual(["-1", "-1", "0", "-1"]);
  });

  it("un clic change de workspace et le mémorise (R7)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await tab(wrapper, "Recette sprint 42").trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_open")).toEqual({ workspaceId: "ws-recette" });
    expect(selectedTab(wrapper)).toBe("Recette sprint 42");
    expect(wrapper.get("main h1").text()).toBe("Recette sprint 42");
    expect(useNavigationStore().titleSubject).toBe("Recette sprint 42");
  });

  it("« Tous les projets » se mémorise avec un onglet vide (null)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await tab(wrapper, "Tous les projets").trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_open")).toEqual({ workspaceId: null });
    expect(core.snapshot().lastWorkspaceId).toBeNull();
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 3");
    expect(wrapper.find("aside").exists()).toBe(false);
  });

  it("les flèches, Début et Fin déplacent le focus ; Entrée (clic) ouvre", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    (tab(wrapper, "Boutique").element as HTMLElement).focus();

    await press("ArrowRight");
    expect(document.activeElement?.textContent.trim()).toBe("Recette sprint 42");
    await press("ArrowRight");
    expect(document.activeElement?.textContent.trim()).toBe("Tous les projets");
    await press("End");
    expect(document.activeElement?.textContent.trim()).toBe("Recette sprint 42");
    await press("Home");
    expect(document.activeElement?.textContent.trim()).toBe("Tous les projets");
    await press("ArrowLeft");
    expect(document.activeElement?.textContent.trim()).toBe("Recette sprint 42");
    // Activation manuelle : le focus seul ne change pas de workspace.
    expect(selectedTab(wrapper)).toBe("Boutique");

    (document.activeElement as HTMLElement).click();
    await flushPromises();
    expect(selectedTab(wrapper)).toBe("Recette sprint 42");
  });
});

describe("nouveau workspace depuis « + » (O3)", () => {
  it("crée un workspace dont l'onglet s'ajoute à la fin et s'ouvre", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await wrapper.get('button[aria-label="Nouveau workspace"]').trigger("click");
    await flushPromises();
    const popover = wrapper.get('[role="dialog"]');
    expect(popover.get("h2").text()).toBe("Nouveau workspace");
    expect(popover.text()).toContain(
      "Un nouvel onglet s'ajoute à la fin. Il démarre vide : glissez-y des projets depuis « Tous les projets ».",
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
    expect(tabs(wrapper).at(-1)?.text()).toBe("Données de référence");
    expect(selectedTab(wrapper)).toBe("Données de référence");
    expect(wrapper.get("main").text()).toContain("Aucun projet pour l'instant");
    expect(wrapper.get(".toast").text()).toBe("Workspace « Données de référence » créé.");
  });

  it("Échap ferme sans rien créer et rend le focus au « + »", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    const plus = wrapper.get('button[aria-label="Nouveau workspace"]');
    (plus.element as HTMLElement).focus();
    await plus.trigger("click");
    await flushPromises();

    await press("Escape");

    expect(wrapper.find('[role="dialog"]').exists()).toBe(false);
    expect(document.activeElement).toBe(plus.element);
    expect(core.commands()).not.toContain("workspace_create");
  });
});

describe("menu contextuel d'un onglet (O4, O5, O6)", () => {
  it("clic droit : renommer, couleur, gérer, supprimer", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await rightClick(wrapper, "Boutique");

    expect(menuItems(wrapper)).toEqual([
      "RenommerF2",
      "Changer la couleur▸",
      "Gérer les workspaces…",
      "Supprimer « Boutique »Les projets restent dans « Tous les projets ».Suppr",
    ]);
    // Le focus est dans le menu : Échap le ferme et le rend à l'onglet.
    await press("Escape");
    expect(wrapper.findAll(".tabs__context [role='menu']")).toHaveLength(0);
    expect(document.activeElement).toBe(tab(wrapper, "Boutique").element);
  });

  it("« Tous les projets » n'a pas de menu", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();

    await rightClick(wrapper, "Tous les projets");

    expect(wrapper.findAll(".tabs__context [role='menu']")).toHaveLength(0);
  });

  it("renomme dans l'onglet : Entrée enregistre, l'erreur du cœur est notifiée", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await rightClick(wrapper, "Boutique");
    await chooseMenuItem(wrapper, "Renommer");

    const input = wrapper.get<HTMLInputElement>(".tab--editing input");
    expect(input.element.value).toBe("Boutique");
    await input.setValue("mes projets");
    await wrapper.get(".tab--editing").trigger("submit");
    await flushPromises();
    expect(wrapper.get(".toast").text()).toBe("Un workspace « Mes projets » existe déjà.");
    expect(wrapper.find(".tab--editing").exists()).toBe(true);

    await input.setValue("E-commerce");
    await wrapper.get(".tab--editing").trigger("submit");
    await flushPromises();
    expect(lastArgs(core, "workspace_rename")).toEqual({
      workspaceId: BOUTIQUE,
      name: "E-commerce",
    });
    expect(wrapper.find(".tab--editing").exists()).toBe(false);
    expect(selectedTab(wrapper)).toBe("E-commerce");
  });

  it("F2 renomme au clavier, Échap annule", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    (tab(wrapper, "Boutique").element as HTMLElement).focus();

    await press("F2");
    expect(document.activeElement).toBe(wrapper.get(".tab--editing input").element);
    await wrapper.get(".tab--editing input").trigger("keydown", { key: "Escape" });
    await flushPromises();

    expect(wrapper.find(".tab--editing").exists()).toBe(false);
    expect(core.commands()).not.toContain("workspace_rename");
    expect(document.activeElement).toBe(tab(wrapper, "Boutique").element);
  });

  it("Maj+F10 ouvre le menu ; le sous-menu change la couleur", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    (tab(wrapper, "Boutique").element as HTMLElement).focus();

    await press("F10", { shiftKey: true });
    expect(document.activeElement?.textContent).toContain("Renommer");
    await press("ArrowDown");
    await press("ArrowRight");
    const colors = wrapper.findAll('.tabs__context [data-level="1"]');
    expect(colors.map((c) => c.text())).toContain("✓Orange");
    await colors.find((c) => c.text().includes("Turquoise"))?.trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_set_color")).toEqual({ workspaceId: BOUTIQUE, color: "teal" });
  });

  it("supprime sans confirmation, revient sur « Tous les projets » ; « Annuler » rouvre l'onglet (R8)", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await rightClick(wrapper, "Boutique");
    await chooseMenuItem(wrapper, "Supprimer « Boutique »");

    expect(core.commands()).toContain("workspace_delete");
    expect(tabs(wrapper).map((t) => t.text())).toEqual(["Tous les projets", "Mes projets"]);
    expect(selectedTab(wrapper)).toBe("Tous les projets");
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "Workspace « Boutique » supprimé · 2 projets conservés dans « Tous les projets ».",
    );

    await toast
      .findAll("button")
      .find((b) => b.text() === "Annuler")
      ?.trigger("click");
    await flushPromises();
    expect(tabs(wrapper).map((t) => t.text())).toEqual([
      "Tous les projets",
      "Mes projets",
      "Boutique",
    ]);
    expect(selectedTab(wrapper)).toBe("Boutique");
  });

  it("Suppr au clavier supprime l'onglet qui a le focus", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    (tab(wrapper, "Mes projets").element as HTMLElement).focus();

    await press("Delete");

    expect(lastArgs(core, "workspace_delete")).toEqual({ workspaceId: MES_PROJETS });
    expect(selectedTab(wrapper)).toBe("Boutique");
  });

  it("« Gérer les workspaces… » ouvre la gestion sur ce workspace", async () => {
    installFakeCore(boutiqueSnapshot());
    wrapper = await mountApp();
    await rightClick(wrapper, "Mes projets");
    await chooseMenuItem(wrapper, "Gérer les workspaces…");

    expect(wrapper.get("main h1").text()).toBe("Workspaces");
    expect(wrapper.get<HTMLInputElement>("main form input").element.value).toBe("Mes projets");
  });
});

describe("glisser un projet sur un onglet (O7)", () => {
  async function dragRowToTab(app: AppWrapper, projectName: string, tabName: string) {
    const row = app.findAll("main tbody tr").find((r) => r.find("th").text() === projectName);
    if (!row) throw new Error(`ligne « ${projectName} » introuvable`);
    const transfer = dataTransfer();
    dispatchDrag(row.element, "dragstart", transfer);
    const target = tab(app, tabName).element;
    const over = dispatchDrag(target, "dragover", transfer);
    await flushPromises();
    return { transfer, target, over };
  }

  it("le dépôt ajoute le projet au workspace et le notifie", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await tab(wrapper, "Tous les projets").trigger("click");
    await flushPromises();

    const { transfer, target, over } = await dragRowToTab(wrapper, "notifications", "Boutique");
    expect(over.defaultPrevented).toBe(true);
    expect(tab(wrapper, "Boutique").classes()).toContain("tab--drop");
    expect(wrapper.get(".tabs__drop-hint").text()).toBe("Ajouter à « Boutique »");

    dispatchDrag(target, "drop", transfer);
    await flushPromises();

    expect(lastArgs(core, "workspace_add_projects")).toEqual({
      workspaceId: BOUTIQUE,
      projectIds: ["p-notifications"],
    });
    expect(wrapper.find(".tabs__drop-hint").exists()).toBe(false);
    expect(wrapper.get(".toast").text()).toBe("« notifications » ajouté à « Boutique ».");
  });

  it("un projet déjà présent n'est pas ajouté deux fois", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await tab(wrapper, "Tous les projets").trigger("click");
    await flushPromises();

    const { transfer, target } = await dragRowToTab(wrapper, "boutique-tests", "Boutique");
    dispatchDrag(target, "drop", transfer);
    await flushPromises();

    expect(core.commands()).not.toContain("workspace_add_projects");
    expect(wrapper.get(".toast").text()).toBe("« boutique-tests » est déjà dans « Boutique ».");
  });

  it("un glisser étranger (texte, fichier) est ignoré", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const transfer = dataTransfer();
    transfer.setData("text/plain", BOUTIQUE_TESTS);

    const over = dispatchDrag(tab(wrapper, "Boutique").element, "dragover", transfer);

    expect(over.defaultPrevented).toBe(false);
    expect(wrapper.find(".tabs__drop-hint").exists()).toBe(false);
  });
});
