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
import { chooseSwitcherAction, chooseWorkspace, currentWorkspace } from "../switcher";

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

/** Ouvre « Gérer les workspaces » par le sélecteur de workspace. */
async function openManage(app: AppWrapper): Promise<void> {
  await chooseSwitcherAction(app, "Gérer les workspaces…");
}

/** Ouvre « Tous les projets » par le sélecteur. */
async function openAll(app: AppWrapper): Promise<void> {
  await chooseWorkspace(app, "Tous les projets");
}

function lastArgs(core: FakeCore, cmd: string) {
  return core.calls.filter((c) => c.cmd === cmd).at(-1)?.args;
}

describe("gérer les workspaces (2d, O11)", () => {
  it("s'ouvre depuis le sélecteur sur le workspace courant, sans barre latérale", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();

    await openManage(wrapper);

    expect(wrapper.find("aside").exists()).toBe(false);
    expect(wrapper.get("main h1").text()).toBe("Workspaces");
    // Le workspace d'où l'on vient reste affiché.
    expect(currentWorkspace(wrapper)).toBe("Boutique");
    const list = wrapper.get("main nav");
    expect(list.get("h2").text()).toBe("3 workspaces");
    expect(list.get('[aria-current="true"]').text()).toContain("Boutique");
    expect(wrapper.get<HTMLInputElement>("main form input").element.value).toBe("Boutique");
    expect(useNavigationStore().titleSubject).toBe("Workspaces");
  });

  it("renomme et change la couleur ; plus de rôle « par défaut »", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper);

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

    expect(wrapper.find('[role="switch"]').exists()).toBe(false);
    expect(wrapper.get("main").text()).not.toContain("par défaut");
  });

  it("tout workspace peut être supprimé, le dernier compris", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper);

    for (const name of ["Mes projets", "Boutique", "Recette sprint 42"]) {
      await button(
        wrapper.get("main nav"),
        `${name}${name === "Recette sprint 42" ? "0 projet" : "2 projets"}`,
      ).trigger("click");
      await button(wrapper.get("main"), "Supprimer ce workspace").trigger("click");
      await flushPromises();
    }

    expect(core.snapshot().workspaces).toEqual([]);
    expect(core.snapshot().projects).toHaveLength(3);
    expect(wrapper.get("main").text()).toContain("Aucun workspace.");
  });

  it("supprime sans confirmation, puis « Annuler » le rétablit (R5, R6)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper);

    await button(wrapper.get("main"), "Supprimer ce workspace").trigger("click");
    await flushPromises();

    expect(lastArgs(core, "workspace_delete")).toEqual({ workspaceId: BOUTIQUE });
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "Workspace « Boutique » supprimé · 2 projets conservés dans « Tous les projets ».",
    );
    expect(wrapper.get("main nav h2").text()).toBe("2 workspaces");
    expect(wrapper.get("main nav [aria-current='true']").text()).toContain("Mes projets");
    expect(core.snapshot().projects).toHaveLength(3);

    await button(toast, "Annuler").trigger("click");
    await flushPromises();
    expect(wrapper.get("main nav h2").text()).toBe("3 workspaces");
    expect(wrapper.get("main nav [aria-current='true']").text()).toContain("Boutique");
  });

  it("retire un projet du workspace détaillé, avec « Annuler »", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper);

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
    await openManage(wrapper);

    await button(wrapper.get("main nav"), "+ Nouveau workspace").trigger("click");
    await flushPromises();
    const popover = wrapper.get('[role="dialog"]');
    await popover.get("input").setValue("Migration paiements");
    await popover.get("form").trigger("submit");
    await flushPromises();

    expect(wrapper.get("main h1").text()).toBe("Workspaces");
    expect(wrapper.get("main nav [aria-current='true']").text()).toContain("Migration paiements");
  });

  it("« Retour au workspace » rouvre le dernier workspace", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openManage(wrapper);

    await button(wrapper.get("main"), "Retour au workspace").trigger("click");
    await flushPromises();

    expect(wrapper.find("aside").exists()).toBe(true);
    expect(wrapper.get("main h1").text()).toBe("Boutique");
  });
});

describe("Tous les projets (2e, O1, O12, P6, P8, P10)", () => {
  it("liste chaque projet une fois avec ses workspaces en pastilles", async () => {
    const snapshot = richSnapshot();
    const missing = snapshot.projects[1];
    if (missing) missing.status = "missing";
    installFakeCore(snapshot).missing.add(AUTH_SSO);
    wrapper = await mountApp();

    await openAll(wrapper);

    expect(currentWorkspace(wrapper)).toBe("Tous les projets");
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 3");
    const rows = wrapper.findAll("tbody tr");
    expect(rows.map((row) => row.get(".all__project").text())).toEqual([
      "boutique-tests",
      "auth-sso",
      "notifications",
    ]);
    expect(rows[0]?.findAll(".chip").map((chip) => chip.text())).toEqual([
      "Mes projets",
      "Boutique",
    ]);
    // Le dossier passe sous le nom du projet, dans la même cellule.
    expect(rows[0]?.get("th .all__path").text()).toBe(snapshot.projects[0]?.path);
    expect(rows[1]?.get("th").text()).toContain("Dossier introuvable");
    expect(rows[2]?.text()).toContain("Aucun workspace");
    expect(wrapper.findAll("thead th").map((th) => th.text())).toEqual([
      "Projet",
      "Workspaces",
      "Exécution",
      "Actions",
    ]);
    expect(rows[0]?.findAll("td")[1]?.text()).toBe("—");
    expect(rows.every((row) => row.attributes("draggable") === "true")).toBe(true);
    expect(wrapper.get("main").text()).toContain("Un projet sans workspace reste ici");
  });

  it("filtre par nom ou dossier", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openAll(wrapper);

    await wrapper.get('main input[type="search"]').setValue("AUTH");

    expect(wrapper.findAll("tbody .all__project").map((name) => name.text())).toEqual(["auth-sso"]);
  });

  it("« Ajouter à… » ajoute le projet à un workspace", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openAll(wrapper);

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

  it("« Oublier » retire le projet de partout, avec « Annuler » (P11)", async () => {
    const core = installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openAll(wrapper);

    await wrapper.get('button[aria-label="Oublier boutique-tests"]').trigger("click");
    await flushPromises();

    expect(lastArgs(core, "project_forget")).toEqual({ projectId: BOUTIQUE_TESTS });
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 2");
    expect(core.snapshot().workspaces.some((w) => w.projectIds.includes(BOUTIQUE_TESTS))).toBe(
      false,
    );
    const toast = wrapper.get(".toast");
    expect(toast.text()).toContain(
      "« boutique-tests » oublié : retiré de « Tous les projets » et de ses workspaces. Ses fichiers restent sur le disque.",
    );

    await button(toast, "Annuler").trigger("click");
    await flushPromises();
    expect(wrapper.get("main h1").text()).toBe("Tous les projets · 3");
    expect(wrapper.findAll("tbody .all__project").map((name) => name.text())[0]).toBe(
      "boutique-tests",
    );
  });

  it("« Afficher dans « Tous les projets » » met le projet en évidence (P8)", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    const trigger = wrapper.get('button[aria-label="Actions du projet boutique-tests"]');
    (trigger.element as HTMLElement).focus();
    await trigger.trigger("click");
    await flushPromises();

    const reveal = wrapper
      .findAll('[role="menuitem"]')
      .find((item) => item.text() === "Afficher dans « Tous les projets »");
    await reveal?.trigger("click");
    await flushPromises();

    const row = wrapper.get(`tr[data-project-id="${BOUTIQUE_TESTS}"]`);
    expect(row.classes()).toContain("all__row--highlight");
    expect(currentWorkspace(wrapper)).toBe("Tous les projets");
    expect(document.activeElement).toBe(row.element);
  });

  it("« Nouveau projet » ouvre la fenêtre sans workspace", async () => {
    installFakeCore(richSnapshot());
    wrapper = await mountApp();
    await openAll(wrapper);

    await button(wrapper.get("main"), "Nouveau projet").trigger("click");
    await flushPromises();

    const dialog = wrapper.get("dialog[open]");
    expect(dialog.text()).toContain("Il sera visible dans « Tous les projets ».");
    expect(dialog.findAll(".storage__chip").map((c) => c.text().replace(/\s+/g, " "))).toEqual([
      "Tous les projets par défaut",
    ]);
  });

  it("affiche la dernière exécution de chaque projet (O12)", async () => {
    const core = installFakeCore(richSnapshot());
    core.workbench.projectLastRuns = [
      { projectId: BOUTIQUE_TESTS, startedAt: Date.UTC(2026, 10, 9, 13, 5), passed: 8, total: 10 },
      { projectId: AUTH_SSO, startedAt: Date.UTC(2026, 10, 9, 13, 5), passed: 3, total: 3 },
    ];
    wrapper = await mountApp();
    await openAll(wrapper);

    const cells = wrapper.findAll("tbody .all__run");
    // Le ratio, puis la date dessous.
    expect(cells[0]?.get(".all__run--ko").text()).toBe("8/10");
    expect(cells[0]?.get(".all__when").text()).toMatch(/^9 nov\./);
    expect(cells[1]?.get(".all__run--ok").text()).toBe("3/3");
    expect(cells[2]?.text()).toBe("—");
  });
});
