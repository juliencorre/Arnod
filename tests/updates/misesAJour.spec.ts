/**
 * Mises à jour (écran 1.6-g) : section des préférences, vérification à la demande, canal,
 * vérification au démarrage annoncée par une notification (lot L allégé) ; téléchargement,
 * progression, annulation, échec et installation d'une version installable (lot L,
 * `docs/plan-mvp-l-mises-a-jour.md`).
 */
import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import { useUpdatesStore } from "@/stores/updates";
import { requestKey, useWorkbenchStore } from "@/stores/workbench";
import type { Release } from "@/types/ipc";

import { type FakeCore, installFakeCore } from "../fakeCore";
import { BOUTIQUE_TESTS, boutiqueSnapshot } from "../fixtures";
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

describe("lot L - installation depuis Arnod (1.6-g)", () => {
  async function installable(): Promise<{ app: AppWrapper; core: FakeCore }> {
    const opened = await openProject((core) => {
      core.updates.available = RELEASE;
      core.updates.installable = true;
    });
    await openUpdates(opened.app);
    await click(opened.app, ".updates__link", "Vérifier maintenant");
    return opened;
  }

  function buttons(wrapper: AppWrapper): string[] {
    return wrapper.findAll(".updates__card button").map((b) => b.text());
  }

  it("« Installer » télécharge et vérifie ; « Au prochain lancement » puis « Redémarrer maintenant »", async () => {
    let core;
    ({ app, core } = await installable());
    const before = { text: app.get(".updates__card").text(), buttons: buttons(app) };
    await click(app, ".updates__card button", "Installer");
    const ready = {
      title: app.get(".updates__card-title").text(),
      buttons: buttons(app),
      toast: app.get(".toast").text(),
    };
    await click(app, ".updates__card button", "Au prochain lancement");
    const onExit = { text: app.get(".updates__card").text(), buttons: buttons(app) };
    await click(app, ".updates__card button", "Redémarrer maintenant");
    expect({
      explained: before.text.includes("vérifie sa signature"),
      before: before.buttons,
      ready,
      onExit: onExit.text.includes("Elle s'installera à la fermeture d'Arnod."),
      onExitButtons: onExit.buttons,
      installs: core.updates.installs,
    }).toEqual({
      explained: true,
      before: ["Installer", "Notes de version"],
      ready: {
        title: "● Arnod 0.10.0 est prête",
        buttons: ["Redémarrer maintenant", "Au prochain lancement", "Notes de version"],
        toast: expect.stringContaining("Arnod 0.10.0 est prête.") as unknown,
      },
      onExit: true,
      onExitButtons: ["Redémarrer maintenant", "Notes de version"],
      installs: [false, true],
    });
  });

  it("téléchargement en cours : progression nommée, « Annuler » au clavier", async () => {
    let core;
    ({ app, core } = await installable());
    core.updates.downloadOutcome = "pending";
    await click(app, ".updates__card button", "Installer");
    const progress = app.get("progress");
    const during = {
      label: progress.attributes("aria-label"),
      value: progress.attributes("value"),
      text: app.get(".updates__card").text().includes("Téléchargement… 3,1 Mo sur 10,0 Mo"),
      live: app.get('.updates [aria-live="polite"]').text(),
      buttons: buttons(app),
    };
    const cancel = app.findAll(".updates__card button").find((b) => b.text() === "Annuler");
    (cancel?.element as HTMLButtonElement | undefined)?.focus();
    expect(document.activeElement?.textContent.trim()).toBe("Annuler");
    await cancel?.trigger("keydown", { key: "Enter" });
    await cancel?.trigger("click");
    await flushPromises();
    expect({
      during,
      after: buttons(app),
      calls: core.commands().includes("updates_cancel"),
    }).toEqual({
      during: {
        label: "Téléchargement d'Arnod 0.10.0",
        value: "3250000",
        text: true,
        live: "Téléchargement d'Arnod 0.10.0…",
        buttons: ["Annuler"],
      },
      after: ["Installer", "Notes de version"],
      calls: true,
    });
  });

  it("signature refusée : le message reste, « Réessayer » et « Voir la version »", async () => {
    let core;
    ({ app, core } = await installable());
    core.updates.downloadOutcome = {
      failed:
        "La signature d'Arnod 0.10.0 ne correspond pas à la clé d'Arnod : le fichier téléchargé est refusé, rien n'a été installé.",
    };
    await click(app, ".updates__card button", "Installer");
    await click(app, ".updates__card button", "Voir la version");
    expect({
      error: app.get(".updates__error--card").text(),
      buttons: buttons(app),
      opened: core.updates.opened,
    }).toEqual({
      error: expect.stringContaining("rien n'a été installé") as unknown,
      buttons: ["Réessayer", "Voir la version"],
      opened: [RELEASE.url],
    });
  });

  it("version non installable : « Voir la version » seulement", async () => {
    ({ app } = await openProject((core) => {
      core.updates.available = RELEASE;
    }));
    await openUpdates(app);
    await click(app, ".updates__link", "Vérifier maintenant");
    expect(buttons(app)).toEqual(["Voir la version"]);
  });

  it("au démarrage, « Installer » dans la notification ; la version prête s'annonce", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.updates.due = true;
    core.updates.available = RELEASE;
    core.updates.installable = true;
    app = await mountApp();
    await flushPromises();
    const toast = app.get(".toast").text();
    await click(app, ".toast button", "Installer");
    await flushPromises();
    expect({
      toast: toast.includes("Arnod 0.10.0 est disponible."),
      ready: app.get(".toast").text(),
      action: app.get(".toast button").text(),
    }).toEqual({
      toast: true,
      ready: expect.stringContaining("Arnod 0.10.0 est prête.") as unknown,
      action: "Redémarrer maintenant",
    });
  });

  it("« Redémarrer maintenant » attend que les brouillons soient enregistrés (R6)", async () => {
    let core;
    ({ app, core } = await installable());
    await click(app, ".updates__card button", "Installer");
    const workbench = useWorkbenchStore();
    await workbench.openRequest(BOUTIQUE_TESTS, "sante.yml");
    await flushPromises();
    const draft = workbench.editors[requestKey(BOUTIQUE_TESTS, "sante.yml")]?.draft;
    if (!draft) throw new Error("brouillon attendu");
    draft.url = "{{baseUrl}}/modifiee";
    core.failNext("request_save", { kind: "io", message: "Disque plein." });
    await expect(useUpdatesStore().installNow()).rejects.toThrow("ne sont pas enregistrées");
    expect(core.updates.installs).toEqual([]);
  });
});
