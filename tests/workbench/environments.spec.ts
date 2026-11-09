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
    const options = app.findAll(".envbar__select option").map((o) => o.text());
    expect(options).toEqual(["Local", "Recette 🔒"]);
    await app.get(".envbar__select").setValue("Recette");
    await flushPromises();
    expect(core.workbench.active).toBe("Recette");
    expect(core.calls.find((c) => c.cmd === "environment_select")?.args).toMatchObject({
      name: "Recette",
    });
  });

  it("crée un environnement depuis la fenêtre de gestion", async () => {
    let core;
    ({ app, core } = await openProject());
    await click(app, ".envbar button", "Gérer");
    expect(app.find("dialog[open]").exists()).toBe(true);
    await click(app, "dialog[open] button", "+ Nouvel environnement");
    expect(core.workbench.envs.map((e) => e.name)).toEqual(["Local", "Nouvel environnement"]);
  });

  it("enregistre une valeur locale comme secret sans l'afficher", async () => {
    let core;
    ({ app, core } = await openProject((c) => {
      const local = c.workbench.envs[0];
      if (local) local.locals = ["token"];
    }));
    await click(app, ".envbar button", "Gérer");
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
