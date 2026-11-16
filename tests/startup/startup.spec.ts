import { clearMocks } from "@tauri-apps/api/mocks";
import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { installFakeCore } from "../fakeCore";
import { fichierVerrouilleLotG, versionPlusRecenteLotG } from "../fakeStartup";
import { boutiqueSnapshot, firstRunSnapshot } from "../fixtures";
import { type AppWrapper, mountApp } from "../mountApp";

/** Lot G - Démarrage et erreurs (docs/plan-mvp-g-demarrage-erreurs.md, écrans 0.3). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
  vi.useRealTimers();
  vi.restoreAllMocks();
  clearMocks();
  document.body.innerHTML = "";
});

function optionButton(wrapper: AppWrapper, label: string) {
  const found = wrapper
    .findAll(".startup-problem__option-button")
    .find((button) => button.text() === label);
  if (!found) throw new Error(`bouton « ${label} » absent`);
  return found;
}

function linkButton(wrapper: AppWrapper, label: string) {
  const found = wrapper.findAll("button").find((button) => button.text() === label);
  if (!found) throw new Error(`bouton « ${label} » absent`);
  return found;
}

function stubClipboard() {
  const writeText = vi.fn(() => Promise.resolve());
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  return writeText;
}

describe("démarrage normal et lent (0.3-a)", () => {
  it("montre la fenêtre une fois prête, sans squelette ni erreur", async () => {
    const core = installFakeCore(boutiqueSnapshot());

    app = await mountApp();

    expect(app.find(".startup-skeleton").exists()).toBe(false);
    expect(app.find(".startup-problem").exists()).toBe(false);
    expect(app.get("main h1").text()).toBe("Boutique");
    const commands = core.commands();
    expect(commands.indexOf("startup_status")).toBeLessThan(commands.indexOf("catalog_load"));
    expect(core.startup.shown).toBe(1);
  });

  it("démarrage lent : squelette de l'écran attendu, sans « Chargement… »", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.startup.slow = true;

    app = await mountApp();

    const skeleton = app.get(".startup-skeleton");
    expect(skeleton.attributes("aria-busy")).toBe("true");
    expect(skeleton.find(".startup-skeleton__sidebar").exists()).toBe(true);
    expect(skeleton.findAll(".startup-skeleton__card")).toHaveLength(2);
    expect(app.text()).not.toContain("Chargement");
    expect(app.find("main").exists()).toBe(false);
    // L'interface ne signale « prête » qu'une fois rendue ; le cœur montre la fenêtre à 300 ms.
    expect(core.startup.shown).toBe(0);
  });
});

describe("erreur au démarrage (0.3-b)", () => {
  it("la cause, ce qui est intact, deux issues dont la recommandée en premier", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();

    app = await mountApp();

    expect(app.get("main h1").text()).toBe("Cet état vient d'une version plus récente d'Arnod");
    expect(app.get('[role="alert"]').text()).toContain("écrit par Arnod 1.0.2 (format 3)");
    expect(app.get(".startup-problem__intact").text()).toContain("Rien n'a été modifié.");
    const options = app.findAll(".startup-problem__option");
    expect(options.map((o) => o.get("h3").text())).toEqual([
      "Installer Arnod 1.0.2 ou plus récent Recommandé",
      "Repartir d'un état vide",
    ]);
    expect(options[0]?.classes()).toContain("startup-problem__option--recommended");
    // Un seul bouton plein par vue : celui de l'issue recommandée.
    expect(app.findAll(".button--primary").map((b) => b.text())).toEqual([
      "Ouvrir la page des versions",
    ]);
    expect(app.get(".startup-problem__path").text()).toContain("arnod.json");
    expect(core.commands()).not.toContain("catalog_load");
    expect(core.startup.shown).toBe(1);
  });

  it("« Repartir d'un état vide » reprend le démarrage et notifie le fichier mis de côté", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();
    app = await mountApp();

    await optionButton(app, "Repartir d'un état vide").trigger("click");
    await flushPromises();

    expect(core.commands()).toContain("startup_start_empty");
    expect(app.find(".startup-problem").exists()).toBe(false);
    expect(app.get("main h1").text()).toBe("Bienvenue dans Arnod");
  });

  it("ouvre la page des versions, au clavier", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();
    app = await mountApp();

    const button = optionButton(app, "Ouvrir la page des versions");
    (button.element as HTMLButtonElement).focus();
    expect(document.activeElement).toBe(button.element);
    await button.trigger("click");
    await flushPromises();

    const call = core.calls.find((c) => c.cmd === "startup_open");
    expect(call?.args).toEqual({ target: "releases" });
  });

  it("« Quitter » ferme Arnod sans rien écrire", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();
    app = await mountApp();

    await linkButton(app, "Quitter").trigger("click");
    await flushPromises();

    expect(core.startup.quit).toBe(true);
  });
});

describe("fichier verrouillé (0.3-b bis)", () => {
  it("réessaie seul après le délai annoncé, puis démarre", async () => {
    vi.useFakeTimers();
    const core = installFakeCore(boutiqueSnapshot());
    core.startup.problem = fichierVerrouilleLotG(2);
    app = await mountApp();

    expect(app.get(".startup-problem__option-note").text()).toBe(
      "Arnod réessaie seul : prochain essai dans 2 s.",
    );
    await vi.advanceTimersByTimeAsync(1000);
    expect(app.get(".startup-problem__option-note").text()).toContain("dans 1 s.");
    await vi.advanceTimersByTimeAsync(1000);
    await flushPromises();

    const retry = core.calls.find((c) => c.cmd === "startup_retry");
    expect(retry?.args).toEqual({ automatic: true });
    expect(app.find(".startup-problem").exists()).toBe(false);
    expect(app.get("main h1").text()).toBe("Boutique");
  });

  it("« Réessayer » à la main garde l'erreur si le verrou tient, avec l'essai suivant", async () => {
    const core = installFakeCore(boutiqueSnapshot());
    core.startup.problem = fichierVerrouilleLotG();
    core.startup.afterRetry = { ...fichierVerrouilleLotG(8), retry: null };
    const tenu = core.startup.afterRetry;
    tenu.options = tenu.options.map((o) =>
      o.action === "retry" ? { ...o, note: "Arnod a réessayé seul 3 fois, sans succès." } : o,
    );
    app = await mountApp();

    await optionButton(app, "Réessayer").trigger("click");
    await flushPromises();

    expect(core.calls.find((c) => c.cmd === "startup_retry")?.args).toEqual({ automatic: false });
    expect(app.get(".startup-problem__option-note").text()).toBe(
      "Arnod a réessayé seul 3 fois, sans succès.",
    );
    await optionButton(app, "Ouvrir le dossier").trigger("click");
    await flushPromises();
    expect(core.calls.at(-1)).toEqual({ cmd: "startup_open", args: { target: "folder" } });
  });
});

describe("détails pour le support (0.3-b ter)", () => {
  it("repliés par défaut, dépliés au clavier", async () => {
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();
    app = await mountApp();

    const toggle = app.get(".startup-problem__toggle");
    expect(toggle.attributes("aria-expanded")).toBe("false");
    expect(app.find("#startup-problem-details").exists()).toBe(false);

    (toggle.element as HTMLButtonElement).focus();
    await toggle.trigger("click");

    expect(toggle.attributes("aria-expanded")).toBe("true");
    const rows = app.findAll(".startup-problem__row-label").map((dt) => dt.text());
    expect(rows).toEqual([
      "Nature",
      "Fichier",
      "Format du fichier",
      "Formats lus ici",
      "Arnod",
      "Date",
    ]);
    expect(app.get(".startup-problem__privacy").text()).toContain("rien n'est envoyé sans vous");
  });

  it("« Copier le détail » et « Copier » copient le texte du cœur", async () => {
    const writeText = stubClipboard();
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();
    app = await mountApp();

    await linkButton(app, "Copier le détail").trigger("click");
    await flushPromises();
    await linkButton(app, "Copier").trigger("click");
    await flushPromises();

    expect(writeText).toHaveBeenNthCalledWith(1, versionPlusRecenteLotG().supportText);
    expect(writeText).toHaveBeenNthCalledWith(2, versionPlusRecenteLotG().path);
    expect(app.get(".toast").text()).toBe("Chemin copié.");
  });

  it("« Signaler le problème… » ouvre l'issue préremplie, sinon en copie l'adresse", async () => {
    const writeText = stubClipboard();
    const core = installFakeCore(firstRunSnapshot());
    core.startup.problem = versionPlusRecenteLotG();
    app = await mountApp();

    await linkButton(app, "Signaler le problème…").trigger("click");
    await flushPromises();
    expect(core.calls.at(-1)).toEqual({ cmd: "startup_open", args: { target: "issue" } });
    expect(writeText).not.toHaveBeenCalled();

    core.startup.openFails = true;
    await linkButton(app, "Signaler le problème…").trigger("click");
    await flushPromises();
    expect(writeText).toHaveBeenCalledWith(versionPlusRecenteLotG().issueUrl);
    expect(app.get(".toast").text()).toContain("l'adresse de l'issue préremplie est copiée");
  });
});
