import { flushPromises } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";

import type { AppWrapper } from "../mountApp";
import { click, openProject } from "./helpers";

/** Barre latérale du projet (docs/plan-barre-laterale-projet.md, B1 à B6). */
let app: AppWrapper | undefined;

afterEach(() => {
  app?.unmount();
  app = undefined;
});

const ADD = '[aria-label="Ajouter au projet boutique-tests"]';

function menuTexts(wrapper: AppWrapper, role = "menuitem"): string[] {
  return wrapper.findAll(`[role="${role}"]`).map((item) => item.text());
}

describe("barre latérale du projet", () => {
  it("une ligne par projet : ▶, + et ⋯ ; l'environnement sous le nom ou à droite s'il est replié (B1, B2)", async () => {
    ({ app } = await openProject());
    const row = app.findAll(".project__row")[0];
    const tools = row?.findAll(".project__tool").map((b) => b.attributes("aria-label"));
    expect(tools).toEqual([
      "Exécuter 3 requêtes sur Local",
      "Ajouter au projet boutique-tests",
      "Plus d'actions sur boutique-tests",
    ]);
    expect(app.get(".project__env").text()).toContain("Local");
    // auth-sso, replié, garde sa pastille à droite de son nom.
    const other = app.findAll(".project__row")[1];
    expect(other?.find(".project__tool").exists()).toBe(false);
    expect(other?.get(".chip--compact").attributes("aria-label")).toBe(
      "Environnement de auth-sso : Local",
    );
  });

  it("le menu « + » se parcourt au clavier et crée une requête (B3)", async () => {
    let core;
    ({ app, core } = await openProject());
    await app.get(ADD).trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(menuTexts(app)).toEqual([
      "Nouvelle requêteCtrl+N",
      "Nouveau dossierCtrl+Maj+N",
      "Ajouter un contrat▸",
      "Nouvel environnement…",
    ]);
    const first = app.findAll('[role="menuitem"]')[0];
    expect(document.activeElement).toBe(first?.element);
    await first?.trigger("keydown", { key: "Enter" });
    await flushPromises();
    await app.get("dialog[open] input").setValue("Supprimer");
    await app.get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_create_request")?.args).toMatchObject({
      folder: "",
      name: "Supprimer",
    });
  });

  it("« Ajouter un contrat » ouvre ses quatre façons dans un sous-menu (B3)", async () => {
    ({ app } = await openProject());
    await click(app, ADD);
    const contract = app.findAll('[role="menuitem"]').find((i) => i.text().includes("contrat"));
    await contract?.trigger("mouseenter");
    await flushPromises();
    const sub = app.findAll('[role="menu"] [role="menu"] [role="menuitem"]').map((i) => i.text());
    expect(sub.map((t) => t.split("OpenAPI")[0]?.trim())).toEqual([
      "Importer un fichier…",
      "Coller un contrat…",
      "Depuis une adresse…",
      "Nouveau contrat…",
    ]);
    await click(app, '[role="menu"] [role="menu"] [role="menuitem"]', "Coller un contrat…");
    expect(app.get("dialog[open] h2").text()).toBe("Coller un contrat");
  });

  it("un projet vide propose « Nouvelle requête » à la place d'un texte d'aide (B4)", async () => {
    ({ app } = await openProject((core) => {
      core.workbench.folders.clear();
      core.workbench.requests.clear();
    }));
    // Sections vides repliées (1.2-d, lot D) ; dépliée, la ligne d'ajout (1.2-a ter).
    expect(app.find(".tree__ghost").exists()).toBe(false);
    await click(app, ".tree .section-head__toggle");
    const ghost = app.get(".tree__ghost");
    expect(ghost.text()).toBe("+ Nouvelle requête");
    expect(app.get('[aria-label^="Exécuter"]').attributes("aria-label")).toBe(
      "Exécuter 0 requête sur Local",
    );
    await ghost.trigger("click");
    await flushPromises();
    expect(app.get("dialog[open] h2").text()).toBe("Nouvelle requête");
  });

  it("Ctrl+N crée une requête et Ctrl+Maj+E ouvre les environnements du projet (B6)", async () => {
    ({ app } = await openProject());
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "n", ctrlKey: true }));
    await flushPromises();
    expect(app.get("dialog[open] h2").text()).toBe("Nouvelle requête");
    await app.get("dialog[open] form").trigger("reset");
    await click(app, "dialog[open] button", "Annuler");

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "E", ctrlKey: true, shiftKey: true }));
    await flushPromises();
    expect(app.get("dialog[open]").text()).toContain("Environnements");
  });

  it("⋯ ouvre le projet ou ses environnements, et le retire du workspace (B5)", async () => {
    ({ app } = await openProject());
    await click(app, `[aria-label="Plus d'actions sur boutique-tests"]`);
    expect(menuTexts(app)).toEqual([
      "Ouvrir le projet",
      "Gérer les environnements…Ctrl+Maj+E",
      "Afficher dans « Tous les projets »",
      "Retirer de « Boutique »Le projet reste dans « Tous les projets ».",
    ]);
    await click(app, '[role="menuitem"]', "Gérer les environnements…");
    expect(app.get("dialog[open]").text()).toContain("Environnements");
  });

  it("menu ouvert, la pastille perd son infobulle ; la gestion s'ouvre en fenêtre large", async () => {
    ({ app } = await openProject());
    const chip = app.get('.project__env [aria-label="Environnement de boutique-tests : Local"]');
    expect(chip.attributes("title")).toBe("Environnement du projet : Local");

    await chip.trigger("click");
    await flushPromises();
    // L'infobulle recouvrirait l'en-tête du menu.
    expect(chip.attributes("title")).toBeUndefined();

    await click(app, '[role="menuitem"]', "Gérer les environnements…");
    expect(app.get("dialog[open]").classes()).toContain("dialog--wide");
    expect(chip.attributes("title")).toBe("Environnement du projet : Local");
  });
});

/** Icônes des sections d'un projet (docs/plan-selecteur-workspace.md, I1, I2). */
describe("sections d'un projet", () => {
  it("« Contrats de service » puis « Référentiel de tests » : icône décorative, nombre et « + » (I1, I2, RC6)", async () => {
    ({ app } = await openProject());
    const heads = app.findAll(".section-head");
    expect(heads.map((h) => h.get("h3").text())).toEqual([
      "Contrats de service",
      "Référentiel de tests",
    ]);
    expect(heads.map((h) => h.get("svg").attributes("data-icon"))).toEqual(["contracts", "tests"]);
    expect(heads.map((h) => h.get("svg").attributes("aria-hidden"))).toEqual(["true", "true"]);
    // Aucun contrat : pas de nombre, la ligne d'ajout suffit.
    expect(heads[0]?.find(".section-head__count").exists()).toBe(false);
    // Déplié, le référentiel montre ses requêtes : le nombre ne s'affiche que replié (lot D).
    expect(heads[1]?.find(".section-head__count").exists()).toBe(false);
    await heads[1]?.get(".section-head__toggle").trigger("click");
    await flushPromises();
    expect(app.findAll(".section-head")[1]?.get(".section-head__count").text()).toBe("3");
  });

  it("le « + » du référentiel de tests crée une requête ou un dossier, au clavier", async () => {
    let core;
    ({ app, core } = await openProject());
    const add = app.get('[aria-label="Ajouter au référentiel de tests de boutique-tests"]');
    await add.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    expect(menuTexts(app)).toEqual(["Nouvelle requêteCtrl+N", "Nouveau dossierCtrl+Maj+N"]);
    await app.findAll('[role="menuitem"]')[0]?.trigger("keydown", { key: "ArrowDown" });
    await flushPromises();
    await app.findAll('[role="menuitem"]')[1]?.trigger("keydown", { key: "Enter" });
    await flushPromises();
    expect(app.get("dialog[open] h2").text()).toBe("Nouveau dossier");
    await app.get("dialog[open] input").setValue("Paiements");
    await app.get("dialog[open] form").trigger("submit");
    await flushPromises();
    expect(core.calls.find((c) => c.cmd === "tree_create_folder")?.args).toMatchObject({
      parent: "",
      name: "Paiements",
    });
  });
});
