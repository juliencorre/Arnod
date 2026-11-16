import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

describe("environnements du projet", () => {
  it("liste les environnements et change l'environnement actif", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      c.workbench.envs.push({
        name: "Recette",
        color: "blue",
        protected: true,
        variables: [],
        locals: [],
      });
    }));
    // Pastille sous le nom du projet ; son menu s'ouvre et se parcourt au clavier (B2).
    const chip = app.get('[aria-label="Environnement de boutique-tests : Local"]');
    await chip.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    const radios = app.findAll('[role="menuitemradio"]');
    expect(radios.map((r) => r.text())).toEqual(["✓Local", "Recetteprotégé"]);
    expect(radios[0]?.attributes("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(radios[0]?.element);
    await radios[0]?.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(document.activeElement).toBe(radios[1]?.element);
    await radios[1]?.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(core.workbench.active).toBe("Recette");
    expect(core.calls.find((c) => c.cmd === "environment_select")?.args).toMatchObject({
      name: "Recette",
    });
  });

  it("crée un environnement depuis la fenêtre de gestion", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[aria-label="Environnement de boutique-tests : Local"]');
    await click(app, '[role="menuitem"]', "Gérer les environnements…");
    expect(app.find("dialog[open]").exists()).toBe(true);
    await click(app, "dialog[open] button", "+ Nouvel environnement");
    expect(core.workbench.envs.map((e) => e.name)).toEqual(["Local", "Nouvel environnement"]);
  });

  it("« + Nouvel environnement » du menu ouvre la gestion sur un nouvel environnement", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, '[aria-label="Environnement de boutique-tests : Local"]');
    await click(app, '[role="menuitem"]', "+ Nouvel environnement");
    await flushPromises();
    expect(app.find("dialog[open]").exists()).toBe(true);
    expect(core.workbench.envs.map((e) => e.name)).toEqual(["Local", "Nouvel environnement"]);
  });

  it("enregistre une valeur locale comme secret sans l'afficher", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      const local = c.workbench.envs[0];
      if (local) local.locals = ["token"];
    }));
    await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
    await click(app, '[role="menuitem"]', "Gérer les environnements…");
    const input = app.get('dialog[open] input[aria-label="Valeur de token"]');
    expect((input.element as HTMLInputElement).type).toBe("password");
    await input.setValue("abc123");
    await click(app, "dialog[open] button", "Enregistrer comme secret");
    expect(core.calls.find((c) => c.cmd === "local_set")?.args).toMatchObject({
      environment: "Local",
      name: "token",
      value: "abc123",
      secret: true,
    });
    expect(app.html()).not.toContain("abc123");
  });
});
