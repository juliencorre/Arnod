/**
 * Version 2, lots E et F (`docs/plan-v2.md`) : synchronisation avec l'équipe. Barre d'état
 * (SY4), relier le projet (7.1-a), panneau « Synchronisation » (7.1-b), conflit (7.1-c),
 * secrets manquants (7.1-e), pastilles et historique en clair (7.1-d). Le cœur décrit tout en
 * clair ; l'interface affiche et transmet les choix.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { initial, statusLabel } from "@/lib/sync";

import type { FakeCore } from "../fakeCore";
import { item, linked, unlinked } from "../fakeSync";
import type { AppWrapper } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

const PROJECT = "p-boutique-tests";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

function statusText(wrapper: AppWrapper): string {
  return wrapper.get(".statusbar [role='status']").text();
}

function dialog(title: string): HTMLDialogElement {
  const found = Array.from(document.querySelectorAll<HTMLDialogElement>("dialog[open]")).find((d) =>
    d.querySelector(".dialog__title")?.textContent.includes(title),
  );
  if (!found) throw new Error(`fenêtre « ${title} » non ouverte`);
  return found;
}

async function press(element: Element | null | undefined): Promise<void> {
  if (!(element instanceof HTMLElement)) throw new Error("élément introuvable");
  element.click();
  await flushPromises();
}

function button(root: Element, text: string): HTMLButtonElement | undefined {
  return Array.from(root.querySelectorAll<HTMLButtonElement>("button")).find((b) =>
    b.textContent.includes(text),
  );
}

describe("libellés de la synchronisation (pur)", () => {
  it("la barre d'état dit l'état en clair", () => {
    expect(statusLabel(null)).toBe("Pas encore partagé");
    expect(statusLabel(unlinked("p"))).toBe("Pas encore partagé");
    expect(statusLabel(linked("p"))).toBe("À jour avec l'équipe");
    expect(
      statusLabel(
        linked(
          "p",
          [item("a", "A", "x"), item("b", "B", "x"), item("c", "C", "x")],
          [item("d", "D", "x"), item("e", "E", "x")],
        ),
      ),
    ).toBe("3 à récupérer · 2 à envoyer");
    expect(statusLabel({ ...linked("p"), conflicts: 1 })).toBe("1 conflit à résoudre");
    expect(statusLabel({ ...linked("p"), error: "hors ligne" })).toBe("Vérification impossible");
    expect(initial("marc")).toBe("M");
    expect(initial("  ")).toBe("?");
  });
});

describe("synchronisation (7.1)", () => {
  it("SY4, 7.1-a : un projet non partagé se relie et s'envoie", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject());
    expect(statusText(app)).toBe("Pas encore partagé");
    await click(app, ".statusbar__sync");
    const link = dialog("Synchroniser boutique-tests");
    const token = link.querySelector<HTMLInputElement>('input[type="password"]');
    if (!token) throw new Error("jeton introuvable");
    token.value = "ghp_jeton";
    token.dispatchEvent(new Event("input"));
    await flushPromises();
    await press(button(link, "Vérifier le jeton"));
    expect(core.calls.find((c) => c.cmd === "sync_token")?.args).toMatchObject({
      forge: "github",
      host: "github.com",
      token: "ghp_jeton",
    });
    expect(link.querySelector(".link__connected")?.textContent).toContain("julien-corre");
    await press(button(link, "acme/boutique-tests"));
    await press(button(link, "Relier et envoyer"));
    expect(core.calls.find((c) => c.cmd === "sync_link")?.args).toMatchObject({
      project: PROJECT,
      url: "https://github.com/acme/boutique-tests.git",
      branch: "main",
      account: { login: "julien-corre" },
    });
    expect(link.querySelector(".link__error")).toBeNull();
    expect(document.querySelector("dialog[open]")).toBeNull();
    expect(statusText(app)).toBe("À jour avec l'équipe");
    expect(app.get(".toast").text()).toContain("Projet relié et envoyé");
  });

  it("7.1-b : ce qui arrive, mes modifications à cocher, un message, puis récupérer et envoyer", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.sync.views.set(
        PROJECT,
        linked(
          PROJECT,
          [item("Clients/lister.yml", "Lister", "assertion body.page ajoutée", "Marc")],
          [
            item("Clients/creer.yml", "Créer", "2 assertions modifiées"),
            item("sante.yml", "Santé", "adresse modifiée"),
          ],
        ),
      );
    }));
    expect(statusText(app)).toBe("1 à récupérer · 2 à envoyer");
    await click(app, ".statusbar__sync");
    const panel = dialog("Synchronisation");
    expect(panel.textContent).toContain("acme / boutique-tests");
    expect(panel.textContent).toContain("branche main");
    expect(panel.textContent).toContain("Lister");
    expect(panel.textContent).toContain("assertion body.page ajoutée");
    expect(panel.textContent).toContain("Marc · il y a 2 h");
    expect(panel.textContent).toContain("2 sélectionnées");
    // Pas de fichier ni de commit à l'écran (R14).
    expect(panel.textContent).not.toContain(".yml");
    const boxes = panel.querySelectorAll<HTMLInputElement>('input[type="checkbox"]');
    await press(boxes[1]);
    expect(panel.textContent).toContain("1 sélectionnée");
    // « Envoyer seulement » n'est offert que quand rien n'arrive.
    expect(button(panel, "Envoyer seulement")).toBeUndefined();
    const message = panel.querySelector<HTMLInputElement>(".panel__input");
    if (!message) throw new Error("message introuvable");
    message.value = "Assertions de création";
    message.dispatchEvent(new Event("input"));
    await flushPromises();
    await press(button(panel, "Récupérer puis envoyer"));
    expect(core.calls.find((c) => c.cmd === "sync_pull_push")?.args).toEqual({
      project: PROJECT,
      items: ["Clients/creer.yml"],
      message: "Assertions de création",
      pull: true,
      push: true,
    });
    expect(statusText(app)).toBe("1 à envoyer");
    expect(document.querySelector("dialog[open]")).toBeNull();
  });

  it("7.1-c : un conflit se règle élément par élément, rien n'est écrit avant de valider", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.sync.views.set(
        PROJECT,
        linked(PROJECT, [item("Clients/lister.yml", "Lister", "x", "Marc")]),
      );
      c.sync.conflicts.set(PROJECT, [
        {
          item: { ...item("Clients/lister.yml", "Lister", "x"), place: "Clients" },
          author: "Marc",
          at: Date.now() - 7_200_000,
          elements: [
            {
              key: "assertions:body.installments#0",
              label: "Assertion body.installments",
              ours: "égal à 4",
              theirs: "entre 2 et 4",
              both: true,
            },
            { key: "url", label: "Adresse", ours: "/v2", theirs: "/v3", both: false },
          ],
          merged: [{ label: "Corps", theirs: true }],
          ours: "info: {}",
          theirs: "info: {}",
        },
      ]);
    }));
    await click(app, ".statusbar__sync");
    await press(button(dialog("Synchronisation"), "Récupérer"));
    const conflict = dialog("1 conflit à résoudre");
    expect(conflict.textContent).toContain("Assertion body.installments");
    expect(conflict.textContent).toContain("Corps : fusionné automatiquement · seul Marc l'a");
    const validate = () => button(conflict, "Valider la résolution");
    expect(validate()?.disabled).toBe(true);
    const radios = conflict.querySelectorAll<HTMLInputElement>('input[type="radio"]');
    // Assertion : garder les deux ; adresse : celle de Marc.
    radios[2]?.click();
    radios[4]?.click();
    await flushPromises();
    expect(validate()?.disabled).toBe(false);
    await press(validate());
    expect(core.sync.resolved).toEqual({
      "Clients/lister.yml": { "assertions:body.installments#0": "both", url: "theirs" },
    });
    expect(statusText(app)).toBe("À jour avec l'équipe");
  });

  it("R13 : abandonner laisse le projet tel quel", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.sync.views.set(PROJECT, { ...linked(PROJECT), conflicts: 1 });
      c.sync.conflicts.set(PROJECT, [
        {
          item: item("sante.yml", "Santé", "x"),
          author: "Sophie",
          at: 0,
          elements: [
            { key: "*", label: "Fichier entier", ours: "modifié", theirs: "supprimé", both: false },
          ],
          merged: [],
          ours: null,
          theirs: null,
        },
      ]);
    }));
    expect(statusText(app)).toBe("1 conflit à résoudre");
    await click(app, ".statusbar__sync");
    await press(button(dialog("1 conflit à résoudre"), "Abandonner la récupération"));
    expect(core.commands()).toContain("sync_abandon");
    expect(core.commands()).not.toContain("sync_resolve");
    expect(app.get(".toast").text()).toContain("le projet est resté tel quel");
  });

  it("7.1-e : les secrets manquants se renseignent ou attendent « Plus tard »", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.sync.views.set(PROJECT, linked(PROJECT, [item("sante.yml", "Santé", "x", "Marc")]));
      c.sync.missing = [{ environment: "Recette fonctionnelle", names: ["token", "cardKey"] }];
    }));
    await click(app, ".statusbar__sync");
    await press(button(dialog("Synchronisation"), "Récupérer"));
    const banner = app.get(".missing");
    expect(banner.text()).toContain(
      "2 secrets à renseigner pour Recette fonctionnelle : token, cardKey",
    );
    await banner
      .findAll("button")
      .find((b) => b.text() === "Plus tard")
      ?.trigger("click");
    await flushPromises();
    expect(core.sync.later).toEqual([
      { environment: "Recette fonctionnelle", names: ["token", "cardKey"] },
    ]);
    expect(app.find(".missing").exists()).toBe(false);
  });

  it("7.1-d : pastille du collègue et historique en clair dans l'onglet « Équipe »", async () => {
    let core: FakeCore;
    ({ app, core } = await openProject((c) => {
      c.sync.views.set(PROJECT, {
        ...linked(PROJECT),
        received: { "Clients/lister.yml": "Marc" },
      });
      c.sync.journal = [
        {
          author: "Marc",
          at: Date.now() - 7_200_000,
          detail: "assertion body.page ajoutée",
          message: "Pagination",
          before: "avant: 1",
          after: "après: 2",
        },
      ];
    }));
    const row = app.get('[role="treeitem"][data-path="Clients/lister.yml"]');
    expect(row.get(".row__author").text()).toContain("modifiée par Marc");
    await row.trigger("click");
    await flushPromises();
    await click(app, '[role="tab"]', "Équipe");
    expect(core.calls.find((c) => c.cmd === "sync_journal")?.args).toEqual({
      project: PROJECT,
      path: "Clients/lister.yml",
    });
    const team = app.get(".team");
    expect(team.text()).toContain("Marc a modifié cette requête — modification récupérée.");
    expect(team.text()).toContain("Marc · assertion body.page ajoutée « Pagination »");
    await click(app, ".team button", "Voir ce qui a changé");
    expect(app.get(".team__diff").text()).toContain("après: 2");
  });
});
