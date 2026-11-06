import { clearMocks } from "@tauri-apps/api/mocks";
import { type DOMWrapper, flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { useNavigationStore } from "@/stores/navigation";

import { type FakeCore, installFakeCore } from "../fakeCore";
import {
  AUTH_SSO,
  BOUTIQUE,
  BOUTIQUE_TESTS,
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

interface Searchable {
  findAll: (selector: string) => DOMWrapper<Element>[];
}

function button(within: Searchable, text: string) {
  const found = within.findAll("button").find((b) => b.text().trim() === text);
  if (!found) throw new Error(`bouton « ${text} » introuvable`);
  return found;
}

async function press(key: string): Promise<void> {
  document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  await flushPromises();
}

function richSnapshot() {
  const snapshot = boutiqueSnapshot();
  snapshot.projects.push(project("p-notifications", "notifications"));
  snapshot.workspaces.push(workspace("ws-recette", "Recette sprint 42", { color: "green" }));
  return snapshot;
}

/** Ouvre l'écran de gestion par le sélecteur de workspaces. */
async function openManage(app: AppWrapper, entry: string): Promise<void> {
  await app.get('aside button[aria-haspopup="dialog"]').trigger("click");
  await flushPromises();
  await button(app.get("aside"), entry).trigger("click");
  await flushPromises();
}

function lastArgs(core: FakeCore, cmd: string) {
  return core.calls.filter((c) => c.cmd === cmd).at(-1)?.args;
}

describe("gérer les workspaces (2d, W7 à W11)", () => {
  it("s'ouvre depuis le sélecteur sur le workspace courant, sans barre latérale", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await openManage(wrapper, "Gérer les workspaces…");

    expect(wrapper.find("aside").exists()).toBe(false);
    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe("Workspaces");
    const list = wrapper.get("nav");
    expect(list.get("h2").text()).toBe("3 workspaces");
    expect(list.get('[aria-current="true"]').text()).toContain("Boutique");
    expect(wrapper.get<HTMLInputElement>("main form input").element.value).toBe("Boutique");
    expect(useNavigationStore().titleSubject).toBe("Workspaces");
  });

  it("renomme, change la couleur et transfère le rôle par défaut", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");

    const name = wrapper.get("main form input");
    await name.setValue("mes projets");
    await wrapper.get("main form").trigger("submit");
    await flushPromises();
    expect(wrapper.get("main form [role='alert']").text()).toBe(
      "Un workspace « Mes projets » existe déjà.",
    );
    await name.setValue("Boutique v2");
    await wrapper.get("main form").trigger("submit");
    await flushPromises();
    expect(lastArgs(core, "workspace_rename")).toEqual({
      workspaceId: BOUTIQUE,
      name: "Boutique v2",
    });

    const color = button(wrapper.get("main"), "Couleur ▾");
    (color.element as HTMLElement).focus();
    await color.trigger("click");
    await flushPromises();
    await press("End");
    expect(document.activeElement?.textContent).toContain("Turquoise");
    await press("Enter");
    expect(lastArgs(core, "workspace_set_color")).toEqual({ workspaceId: BOUTIQUE, color: "teal" });

    const toggle = wrapper.get('[role="switch"]');
    expect(toggle.attributes("aria-checked")).toBe("false");
    expect(wrapper.get("main").text()).toContain("(actuellement : Mes projets)");
    await toggle.trigger("click");
    await flushPromises();
    expect(lastArgs(core, "workspace_set_default")).toEqual({ workspaceId: BOUTIQUE });
    expect(toggle.attributes("aria-checked")).toBe("true");
    expect(toggle.attributes("disabled")).toBeDefined();
    expect(wrapper.get("main").text()).toContain("(actuellement : Boutique v2)");
  });

  it("le workspace par défaut ne peut pas être supprimé", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");

    await button(wrapper.get("nav"), "Mes projetspar défaut2 projets").trigger("click");

    expect(
      button(wrapper.get("main"), "Supprimer ce workspace").attributes("disabled"),
    ).toBeDefined();
    expect(wrapper.get("main").text()).toContain(
      "Le workspace par défaut ne peut pas être supprimé",
    );
  });

  it("supprime sans confirmation, puis « Annuler » le rétablit (R5, R6)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");

    await button(wrapper.get("main"), "Supprimer ce workspace").trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_delete")).toEqual({ workspaceId: BOUTIQUE });
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "Workspace « Boutique » supprimé · 2 projets conservés dans le référentiel.",
    );
    expect(wrapper.get("nav h2").text()).toBe("2 workspaces");
    expect(wrapper.get("nav [aria-current='true']").text()).toContain("Mes projets");
    expect(core.snapshot().projects).toHaveLength(3);

    await button(toast, "Annuler").trigger("click");
    await flushPromises();
    expect(wrapper.get("nav h2").text()).toBe("3 workspaces");
    expect(wrapper.get("nav [aria-current='true']").text()).toContain("Boutique");
  });

  it("retire un projet du workspace détaillé, avec « Annuler »", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");

    await wrapper.get('button[aria-label="Retirer auth-sso de Boutique"]').trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_remove_project")).toEqual({
      workspaceId: BOUTIQUE,
      projectId: AUTH_SSO,
    });
    expect(wrapper.get("main").text()).toContain("Projets · 1");
  });

  it("crée un workspace et le sélectionne sans quitter l'écran", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");

    await button(wrapper.get("nav"), "+ Nouveau workspace").trigger("click");
    await flushPromises();
    const dialog = wrapper.get("dialog[open]");
    await dialog.get("input").setValue("Migration paiements");
    await dialog.get("form").trigger("submit");
    await flushPromises();

    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe("Workspaces");
    expect(wrapper.get("nav [aria-current='true']").text()).toContain("Migration paiements");
  });

  it("« Retour au workspace » rouvre le dernier workspace", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");

    await button(wrapper.get("main"), "Retour au workspace").trigger("click");
    await flushPromises();

    expect(wrapper.find("aside").exists()).toBe(true);
    expect(wrapper.get("main h1").text()).toBe("Boutique");
  });
});

describe("référentiel des projets (2e, P6, P8, P9, P10)", () => {
  it("liste chaque projet une fois avec ses workspaces en pastilles", async () => {
    const snapshot = richSnapshot();
    const missing = snapshot.projects[1];
    if (missing) missing.status = "missing";
    installFakeCore(snapshot).missing.add(AUTH_SSO);
    wrapper = await mountApp();

    await openManage(wrapper, "Référentiel des projets3 projets");

    expect(wrapper.get('[role="tab"][aria-selected="true"]').text()).toBe(
      "Référentiel des projets",
    );
    expect(wrapper.get("main h1").text()).toBe("3 projets");
    const rows = wrapper.findAll("tbody tr");
    expect(rows.map((row) => row.get("th").text())).toEqual([
      "boutique-tests",
      "auth-sso",
      "notifications",
    ]);
    expect(rows[0]?.findAll(".chip").map((chip) => chip.text())).toEqual([
      "Mes projets",
      "Boutique",
    ]);
    expect(rows[1]?.text()).toContain("Dossier introuvable");
    expect(rows[2]?.text()).toContain("Aucun workspace");
    expect(rows[0]?.findAll("td")[2]?.text()).toBe("—");
    expect(wrapper.get("main").text()).toContain("Un projet sans workspace reste ici");
  });

  it("filtre par nom ou dossier", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Référentiel des projets3 projets");

    await wrapper.get('main input[type="search"]').setValue("AUTH");

    expect(wrapper.findAll("tbody th").map((th) => th.text())).toEqual(["auth-sso"]);
  });

  it("« Ajouter à… » ajoute le projet à un workspace", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Référentiel des projets3 projets");

    const trigger = wrapper.get('button[aria-label="Ajouter notifications à un workspace"]');
    (trigger.element as HTMLElement).focus();
    await trigger.trigger("click");
    await flushPromises();
    const items = wrapper.findAll('[role="menuitemcheckbox"]');
    expect(items.map((item) => item.text())).toEqual([
      "Mes projets",
      "Boutique",
      "Recette sprint 42",
    ]);
    await press("End");
    await press("Enter");

    expect(lastArgs(core, "workspace_add_projects")).toEqual({
      workspaceId: "ws-recette",
      projectIds: ["p-notifications"],
    });
    expect(
      wrapper
        .findAll("tbody tr")[2]
        ?.findAll(".chip")
        .map((c) => c.text()),
    ).toEqual(["Recette sprint 42"]);
  });

  it("« Oublier » retire le projet du référentiel et de ses workspaces, avec « Annuler » (P11)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Référentiel des projets3 projets");

    await wrapper.get('button[aria-label="Oublier boutique-tests"]').trigger("click");
    await flushPromises();

    expect(lastArgs(core, "project_forget")).toEqual({ projectId: BOUTIQUE_TESTS });
    expect(wrapper.get("main h1").text()).toBe("2 projets");
    expect(core.snapshot().workspaces.some((w) => w.projectIds.includes(BOUTIQUE_TESTS))).toBe(
      false,
    );
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "« boutique-tests » oublié : retiré du référentiel et de ses workspaces. Ses fichiers restent sur le disque.",
    );

    await button(toast, "Annuler").trigger("click");
    await flushPromises();
    expect(wrapper.get("main h1").text()).toBe("3 projets");
    expect(wrapper.findAll("tbody th").map((th) => th.text())[0]).toBe("boutique-tests");
  });

  it("« Afficher dans le référentiel » met le projet en évidence (P8)", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const trigger = wrapper.get('button[aria-label="Actions du projet boutique-tests"]');
    (trigger.element as HTMLElement).focus();
    await trigger.trigger("click");
    await flushPromises();

    const reveal = wrapper
      .findAll('[role="menuitem"]')
      .find((item) => item.text() === "Afficher dans le référentiel");
    await reveal?.trigger("click");
    await flushPromises();

    const row = wrapper.get(`tr[data-project-id="${BOUTIQUE_TESTS}"]`);
    expect(row.classes()).toContain("registry__row--highlight");
    expect(document.activeElement).toBe(row.element);
  });

  it("« Nouveau projet » ouvre la fenêtre sur le dernier workspace ouvert", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Référentiel des projets3 projets");

    await button(wrapper.get("main"), "Nouveau projet").trigger("click");
    await flushPromises();

    expect(wrapper.get<HTMLSelectElement>("dialog[open] select").element.value).toBe(BOUTIQUE);
  });

  it("les onglets se parcourent aux flèches", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper, "Gérer les workspaces…");
    const tab = wrapper.get('[role="tab"][aria-selected="true"]');
    (tab.element as HTMLElement).focus();

    await press("ArrowRight");

    expect(document.activeElement?.textContent.trim()).toBe("Référentiel des projets");
    expect(wrapper.get("main h1").text()).toBe("3 projets");
    expect(useNavigationStore().view).toMatchObject({ kind: "manage", tab: "registry" });
  });
});
