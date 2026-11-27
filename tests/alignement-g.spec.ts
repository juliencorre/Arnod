/**
 * Alignement sur la maquette, lot G (docs/plan-alignement-maquette.md, T18 et T19) : page des
 * environnements et portée d'une extraction. Chaque test est la preuve d'un écart `aligner: code`
 * de docs/maquette-etat.yml.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { AUTOSAVE_MS } from "@/stores/workbench";

import { newRequest, runOf } from "./fakeWorkbench";
import { AUTH_SSO, BOUTIQUE_TESTS } from "./fixtures";
import type { AppWrapper } from "./mountApp";
import { click, openProject } from "./workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

async function waitAutosave(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, AUTOSAVE_MS + 50));
  await flushPromises();
}

describe("portée d'une extraction (AM44, 4.2-b bis)", () => {
  it("« le test ▾ » se change en « l'environnement », écrit dans la requête", async () => {
    const opened = await openProject((c) => {
      c.workbench.requests.set(
        "Clients/creer.yml",
        newRequest("Créer", {
          method: "POST",
          extractions: [{ name: "client.id", source: "body.id", enabled: true, scope: "runtime" }],
        }),
      );
    });
    app = opened.app;
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "Variables");

    const scope = app.get<HTMLSelectElement>("select[aria-label='Portée de client.id']");
    expect(scope.element.value).toBe("runtime");
    expect(scope.findAll("option").map((o) => o.text())).toEqual(["le test", "l'environnement"]);
    await scope.setValue("environment");
    await waitAutosave();
    expect(opened.core.workbench.requests.get("Clients/creer.yml")?.extractions).toEqual([
      { name: "client.id", source: "body.id", enabled: true, scope: "environment" },
    ]);
  });

  it("une portée écrite à la main (« collection ») reste montrée et gardée", async () => {
    const opened = await openProject((c) => {
      c.workbench.requests.set(
        "Clients/creer.yml",
        newRequest("Créer", {
          method: "POST",
          extractions: [
            { name: "lieu", source: "headers.Location", enabled: true, scope: "collection" },
          ],
        }),
      );
    });
    app = opened.app;
    await click(app, '[role="treeitem"]', "Créer");
    await click(app, ".editor__tab", "Variables");
    const scope = app.get<HTMLSelectElement>("select[aria-label='Portée de lieu']");
    expect(scope.element.value).toBe("collection");
    expect(scope.findAll("option").map((o) => o.text())).toEqual([
      "le test",
      "l'environnement",
      "le projet (comme le test)",
    ]);
  });
});

describe("page des environnements (AM43, 3.3-a)", () => {
  const RECETTE = {
    name: "Recette",
    color: "blue",
    protected: true,
    variables: [
      { name: "baseUrl", value: "https://recette.boutique.dev" },
      { name: "devise", value: "EUR" },
    ],
    locals: ["token"],
  };

  async function openPage() {
    const opened = await openProject((c) => {
      c.workbench.envs.push(structuredClone(RECETTE));
      c.workbench.lastRun = { ...runOf(c.workbench, ""), environment: "Recette", number: 3 };
    });
    app = opened.app;
    await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
    await click(app, '[role="menuitem"]', "Gérer les environnements…");
    return opened;
  }

  it("s'ouvre dans un onglet : fil d'Ariane, nombre, « Copier… », « + Nouvel environnement »", async () => {
    const opened = await openPage();
    expect(opened.core.calls.filter((c) => c.cmd === "layout_open_selection").at(-1)?.args).toEqual(
      { content: { kind: "environments", projectId: BOUTIQUE_TESTS } },
    );
    expect(app?.find("dialog[open]").exists()).toBe(false);
    const page = app?.get(".envs");
    expect(page?.findAll(".envs__crumbs span").map((c) => c.text())).toEqual([
      "boutique-tests",
      "/",
      "environnements",
    ]);
    expect(page?.get("h1").text()).toBe("Environnements");
    expect(page?.get(".envs__count").text()).toBe("2 · propres à ce projet");
    expect(page?.findAll(".envs__head-actions button").map((b) => b.text())).toEqual([
      "Copier depuis un autre projet…",
      "+ Nouvel environnement",
    ]);
    expect(page?.findAll(".envs__item-meta").map((m) => m.text())).toEqual(["protégé"]);
    expect(page?.get(".envs__side-note").text()).toBe("Glissez ⋮⋮ pour réordonner.");
  });

  it("un seul tableau Variable, Valeur, Portée ; « locale » range la valeur au coffre", async () => {
    const opened = await openPage();
    await click(app as AppWrapper, ".envs .envs__item", "Recette");
    const page = app?.get(".envs");
    expect(page?.get(".envs__title").text().replace(/\s+/g, " ")).toBe(
      "Variables · 3 utilisables partout avec {{ nom }}",
    );
    expect(page?.findAll(".envs__table th").map((h) => h.text())).toEqual([
      "Variable",
      "Valeur",
      "Portée",
      "Retirer",
    ]);
    const scopes = page?.findAll<HTMLSelectElement>(".envs__scope") ?? [];
    expect(scopes.map((s) => s.attributes("aria-label"))).toEqual([
      "Portée de baseUrl",
      "Portée de devise",
      "Portée de token",
    ]);
    expect(scopes.map((s) => s.element.value)).toEqual(["shared", "shared", "local"]);

    await scopes[1]?.setValue("local");
    await flushPromises();
    const recette = opened.core.workbench.envs.find((e) => e.name === "Recette");
    expect(recette?.variables.map((v) => v.name)).toEqual(["baseUrl"]);
    expect(recette?.locals).toEqual(["token", "devise"]);
    expect(opened.core.calls.filter((c) => c.cmd === "local_set").at(-1)?.args).toMatchObject({
      environment: "Recette",
      name: "devise",
      value: "EUR",
      secret: true,
    });
  });

  it("⋮⋮ réordonne la chaîne au clavier ; « Utilisation » dit le dernier run", async () => {
    const opened = await openPage();
    const handle = app?.get(".envs__handle[data-name='Local']");
    expect(handle?.attributes("aria-label")).toBe("Déplacer Local");
    await handle?.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(opened.core.calls.find((c) => c.cmd === "environments_reorder")?.args).toMatchObject({
      names: ["Recette", "Local"],
    });

    expect(app?.get(".envs__usage").text()).toBe("Aucun run sur cet environnement pour l'instant.");
    await click(app as AppWrapper, ".envs .envs__item", "Recette");
    expect(app?.get(".envs__usage").text()).toMatch(/^Dernier run n° 3 du projet, /);
  });

  it("« Copier depuis un autre projet… » ajoute les environnements choisis, sans valeurs locales", async () => {
    const opened = await openPage();
    await click(app as AppWrapper, ".envs button", "Copier depuis un autre projet…");
    const dialog = app?.get("dialog[open]");
    expect(dialog?.get("h2").text()).toBe("Copier depuis un autre projet");
    expect(dialog?.get("select").element.value).toBe(AUTH_SSO);
    const recette = dialog
      ?.findAll("label.copy__row")
      .find((row) => row.text().includes("Recette"));
    await recette?.get("input").setValue(true);
    await dialog?.get("form").trigger("submit");
    await flushPromises();
    expect(opened.core.calls.find((c) => c.cmd === "environment_copy")?.args).toEqual({
      project: BOUTIQUE_TESTS,
      fromProject: AUTH_SSO,
      names: ["Recette"],
    });
    expect(app?.find("dialog[open]").exists()).toBe(false);
    expect(app?.get(".envs .envs__item[aria-current='true'] .envs__item-name").text()).toBe(
      "Recette 2",
    );
    expect(app?.get(".toast").text()).toContain(
      "1 environnement copié de « auth-sso » : les valeurs locales restent à saisir sur ce poste.",
    );
  });
});
