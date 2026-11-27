/**
 * Version 2, lot D (`docs/plan-v2.md`, LC6) : « Copier la commande » du menu ⋯ d'un test ou d'un
 * dossier (5.3-c). Le cœur écrit la commande `arnod run` avec l'environnement actif ; elle part
 * dans le presse-papiers et une notification le confirme.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function clipboard(fails = false): ReturnType<typeof vi.fn> {
  const writeText = vi.fn(() => (fails ? Promise.reject(new Error("refus")) : Promise.resolve()));
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  return writeText;
}

async function openMenu(wrapper: AppWrapper, name: string): Promise<void> {
  const row = wrapper
    .findAll('li[role="presentation"]')
    .find((li) => li.get('[role="treeitem"]').text().includes(name));
  await row?.get(".row__more").trigger("click");
  await flushPromises();
}

describe("Copier la commande (5.3-c)", () => {
  it("copie arnod run du test avec l'environnement actif", async () => {
    const writeText = clipboard();
    let core;
    ({ app, core } = await openProject());
    // « Clients » ne contient que des requêtes : c'est un test.
    await openMenu(app, "Clients");
    await click(app, '[role="menuitem"]', "Copier la commande");
    expect(core.calls.find((c) => c.cmd === "cli_command")?.args).toEqual({
      project: "p-boutique-tests",
      path: "Clients",
      environment: "Local",
      shell: null,
    });
    expect(writeText).toHaveBeenCalledWith('arnod run "Clients" --env "Local"');
    expect(app.get(".toast").text()).toContain(
      "✓ Commande copiée · elle utilise l'environnement « Local »",
    );
  });

  it("un presse-papiers refusé montre la commande à copier à la main", async () => {
    clipboard(true);
    ({ app } = await openProject((c) => {
      c.workbench.folders.add("Clients/archives");
    }));
    await openMenu(app, "Clients");
    await click(app, '[role="menuitem"]', "Copier la commande");
    expect(app.get(".toast").text()).toContain(
      'Copie impossible ; la commande : arnod run "Clients" --env "Local"',
    );
  });
});
