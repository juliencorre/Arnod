/**
 * Lot L allégé - Mises à jour (écran 1.6-g) : section des préférences, vérification à la
 * demande, canal, vérification au démarrage annoncée par une notification. Rien n'est installé.
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { Release } from "@/types/ipc";

import { installFakeCore } from "../fakeCore";
import { boutiqueSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";
import { click, openProject } from "../workbench/helpers";

let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  clearMocks();
  document.body.innerHTML = "";
});

const RELEASE: Release = {
  version: "0.10.0",
  prerelease: false,
  highlights: ["Copier et déplacer une requête", "Filtre de l'arborescence"],
  url: "https://github.com/juliencorre/Arnod/releases/tag/v0.10.0",
  published: "2026-11-14",
};

async function openUpdates(wrapper: AppWrapper): Promise<void> {
  window.dispatchEvent(new KeyboardEvent("keydown", { key: ",", ctrlKey: true }));
  await flushPromises();
  await click(wrapper, ".preferences__section", "Mises à jour");
}

describe("lot L - mises à jour (1.6-g)", () => {
  it("« Vérifier maintenant » trouve la version, ses points et ouvre sa page", async () => {
    let core;
    ({ app, core } = await openProject());
    core.updates.available = RELEASE;
    await openUpdates(app);
    const before = app.get(".updates").text();
    await click(app, ".updates__link", "Vérifier maintenant");
    const card = app.get(".updates__card");
    await click(app, ".updates__card button", "Voir la version");
    expect({
      current: before.includes("Arnod 0.9.0, installée sur ce poste."),
      never: before.includes("Dernière vérification : jamais"),
      title: card.get(".updates__card-title").text(),
      points: card.findAll("li").map((li) => li.text()),
      opened: core.updates.opened,
    }).toEqual({
      current: true,
      never: true,
      title: "● Arnod 0.10.0 est disponible",
      points: RELEASE.highlights,
      opened: [RELEASE.url],
    });
  });

  it("à jour : le dit ; le canal et la vérification automatique s'enregistrent", async () => {
    let core;
    ({ app, core } = await openProject());
    await openUpdates(app);
    await click(app, ".updates__link", "Vérifier maintenant");
    const uptodate = app.get(".updates__uptodate").text();
    const radios = app.findAll('.updates input[type="radio"]');
    await radios[1]?.setValue(true);
    await radios[1]?.trigger("change");
    const box = app.get('.updates input[type="checkbox"]');
    await box.setValue(false);
    await box.trigger("change");
    await flushPromises();
    expect({
      uptodate,
      prefs: core.updates.view.prefs.channel,
      auto: core.updates.view.prefs.automatic,
    }).toEqual({
      uptodate: "✓ Arnod est à jour sur le canal stable.",
      prefs: "prerelease",
      auto: false,
    });
  });

  it("au démarrage, une version plus récente s'annonce ; « Voir » ouvre la section", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.updates.due = true;
    core.updates.available = RELEASE;
    app = await mountApp();
    await flushPromises();
    const toast = app.get(".toast").text();
    await click(app, ".toast button", "Voir");
    await flushPromises();
    expect({
      toast: toast.includes("Arnod 0.10.0 est disponible."),
      section: app.get('.preferences__section[aria-current="page"]').text(),
    }).toEqual({ toast: true, section: "Mises à jour" });
  });
});
