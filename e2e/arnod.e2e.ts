/**
 * Parcours de bout en bout sur l'application réelle (binaire debug, cœur Rust,
 * vrai disque) : création et gestion des projets et des workspaces.
 *
 * Les étapes s'enchaînent et partagent un même bac à sable : elles suivent le
 * premier usage puis l'usage quotidien d'un utilisateur. Le sélecteur de
 * dossier natif (« Changer », P3) ne se pilote pas par WebDriver : il est
 * couvert par les tests Rust et Vitest.
 */
import type { ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

import {
  type Browser,
  type Sandbox,
  cardNames,
  createSandbox,
  heading,
  launchApp,
  removeSandbox,
  readState,
  screenshot,
  startDriver,
  stopDriver,
  themeEnv,
  toastText,
  waitForHeading,
} from "./app";

let sandbox: Sandbox;
let driver: ChildProcess | undefined;
let browser: Browser;

beforeAll(async () => {
  sandbox = createSandbox();
  driver = await startDriver(sandbox, themeEnv());
  browser = await launchApp();
});

afterAll(async () => {
  try {
    // `browser` reste indéfini si le lancement a échoué.
    await (browser as Browser | undefined)?.deleteSession().catch(() => undefined);
  } finally {
    await stopDriver(driver);
    removeSandbox(sandbox);
  }
});

/** Redémarre Arnod sur le même bac à sable (nouvelle session WebDriver). */
async function restart(): Promise<void> {
  await browser.deleteSession();
  browser = await launchApp();
}

async function openSwitcher(): Promise<void> {
  await browser.$('aside button[aria-haspopup="dialog"]').click();
  await browser.$('[aria-label="Changer de workspace"]').waitForDisplayed();
}

async function openProjectMenu(name: string): Promise<void> {
  await browser.$(`button[aria-label="Actions du projet ${name}"]`).click();
  await browser.$('[role="menu"]').waitForDisplayed();
}

async function dismissToast(): Promise<void> {
  await browser.execute(() => {
    document.querySelector(".toast")?.dispatchEvent(new MouseEvent("mouseleave"));
  });
}

describe("Arnod, de bout en bout", () => {
  it("premier lancement : « Mes projets » par défaut et invitation à créer (1a, W1)", async () => {
    expect(await heading(browser)).toBe("Créez votre premier projet");
    const sidebar = await browser.$("aside").getText();
    expect(sidebar).toContain("Mes projets");
    expect(sidebar).toContain("par défaut");
    expect(sidebar).toContain("Créé automatiquement. Votre premier projet y sera rangé.");

    const state = readState(sandbox);
    expect(state.workspaces.map((w) => w.name)).toEqual(["Mes projets"]);
    expect(state.projects).toEqual([]);
    await screenshot(browser, "01-premier-lancement");
  });

  it("crée le premier projet sur disque et l'ouvre (P1, P4, P5)", async () => {
    await browser.$("form input").setValue("boutique-tests");
    const folder = browser.$('[data-testid="project-folder"]');
    const expected = path.join(sandbox.home, "arnod", "boutique-tests");
    await browser.waitUntil(async () => (await folder.getText()) === expected, {
      timeoutMsg: `aperçu du dossier « ${expected} » attendu`,
    });

    await browser.$("button=Créer le projet").click();
    await waitForHeading(browser, "boutique-tests");

    expect(await toastText(browser)).toContain(
      "« boutique-tests » créé dans le workspace Mes projets (par défaut).",
    );
    const manifest = readFileSync(path.join(expected, "arnod.yml"), "utf8");
    expect(manifest).toContain("name: boutique-tests");
    const state = readState(sandbox);
    expect(state.projects.map((p) => p.name)).toEqual(["boutique-tests"]);
    expect(state.workspaces[0]?.projectIds).toEqual([state.projects[0]?.id]);
    await screenshot(browser, "02-projet-cree");
  });

  it("le menu du sélecteur s'affiche en entier, par-dessus la zone principale (2b, W4)", async () => {
    await openSwitcher();
    const layout = await browser.execute(() => {
      const sidebar = document.querySelector("aside");
      const popover = document.querySelector('[aria-label="Changer de workspace"]');
      if (!sidebar || !popover) return null;
      const box = popover.getBoundingClientRect();
      // Près de chaque coin, c'est bien le menu qui est affiché : il n'est pas rogné.
      const corners = [
        [box.left + 4, box.top + 4],
        [box.right - 4, box.top + 4],
        [box.left + 4, box.bottom - 4],
        [box.right - 4, box.bottom - 4],
      ] as const;
      return {
        sidebarScrollLeft: sidebar.scrollLeft,
        overflowsSidebar: box.right > sidebar.getBoundingClientRect().right,
        cornersVisible: corners.every(([x, y]) =>
          popover.contains(document.elementFromPoint(x, y)),
        ),
      };
    });
    expect(layout).toEqual({ sidebarScrollLeft: 0, overflowsSidebar: true, cornersVisible: true });
    await screenshot(browser, "02b-selecteur-workspaces");

    await browser.keys("Escape");
    await browser.$('[aria-label="Changer de workspace"]').waitForDisplayed({ reverse: true });
  });

  it("refuse un nom déjà pris, message du cœur sous le champ (R9)", async () => {
    await browser.$("button=← Mes projets").click();
    await waitForHeading(browser, "Mes projets");
    await browser.$("button=+ Nouveau projet").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();

    await dialog.$("input").setValue("BOUTIQUE-TESTS");
    await dialog.$("button=Créer le projet").click();

    const error = dialog.$('[role="alert"]');
    await error.waitForDisplayed();
    expect(await error.getText()).toBe(
      "Un projet « boutique-tests » existe déjà dans le référentiel.",
    );
    await dialog.$("button=Annuler").click();
    await dialog.waitForDisplayed({ reverse: true });
  });

  it("crée un workspace depuis le sélecteur et l'ouvre (2b, W5)", async () => {
    await openSwitcher();
    await browser.$("button=+ Nouveau workspace").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    await dialog.$("input").setValue("Boutique");
    await browser.keys("Enter");

    await waitForHeading(browser, "Boutique");
    expect(await browser.$("main").getText()).toContain("Aucun projet pour l'instant");
    expect(readState(sandbox).workspaces.map((w) => w.name)).toEqual(["Mes projets", "Boutique"]);
  });

  it("ajoute un projet du référentiel au workspace (2c, W6)", async () => {
    await browser.$("main .add-card").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    expect(await dialog.$("h2").getText()).toBe("Ajouter des projets à « Boutique »");

    await dialog.$("label*=boutique-tests").click();
    await dialog.$("button=Ajouter 1 projet").click();

    await browser.waitUntil(async () => (await cardNames(browser)).length === 1);
    expect(await cardNames(browser)).toEqual(["boutique-tests"]);
    expect(await toastText(browser)).toBe("« boutique-tests » ajouté à « Boutique ».");
  });

  it("crée un projet depuis le workspace : rangé ici et dans le défaut (2a bis, R4)", async () => {
    await browser.$("button=+ Nouveau projet").click();
    const dialog = browser.$("dialog[open]");
    await dialog.waitForDisplayed();
    expect(await dialog.getText()).toContain("Aussi ajouté à");

    await dialog.$("input").setValue("paiements-v3");
    await dialog.$("button=Créer le projet").click();
    await waitForHeading(browser, "paiements-v3");

    expect(existsSync(path.join(sandbox.home, "arnod", "paiements-v3", "arnod.yml"))).toBe(true);
    const state = readState(sandbox);
    const id = state.projects.find((p) => p.name === "paiements-v3")?.id;
    expect(state.workspaces.every((w) => id !== undefined && w.projectIds.includes(id))).toBe(true);

    await browser.$("button=← Boutique").click();
    await waitForHeading(browser, "Boutique");
    expect(await cardNames(browser)).toEqual(["boutique-tests", "paiements-v3"]);
    await screenshot(browser, "03-workspace-boutique");
  });

  it("retire un projet du workspace puis annule (P7, R5, R6)", async () => {
    await openProjectMenu("paiements-v3");
    await browser.$('[role="menuitem"]*=Retirer de « Boutique »').click();

    await browser.waitUntil(async () => (await cardNames(browser)).length === 1);
    expect(await toastText(browser)).toContain("Le projet reste dans le référentiel.");
    expect(readState(sandbox).projects).toHaveLength(2);

    await browser.$(".toast").$("button=Annuler").click();
    await browser.waitUntil(async () => (await cardNames(browser)).length === 2);
    expect(await cardNames(browser)).toEqual(["boutique-tests", "paiements-v3"]);
  });

  it("gère les workspaces : défaut transféré, suppression annulée (2d, W9, W11)", async () => {
    await openSwitcher();
    await browser.$("button=Gérer les workspaces…").click();
    await browser.$('[role="tab"][aria-selected="true"]').waitForDisplayed();
    expect(await browser.$("aside").isExisting()).toBe(false);

    await browser.$('[role="switch"]').click();
    await browser.waitUntil(async () =>
      (await browser.$("main").getText()).includes("(actuellement : Boutique)"),
    );
    expect(readState(sandbox).defaultWorkspaceId).toBe(
      readState(sandbox).workspaces.find((w) => w.name === "Boutique")?.id,
    );
    await screenshot(browser, "04-gerer-workspaces");

    await browser.$("nav").$("button*=Mes projets").click();
    await browser.$("button=Supprimer ce workspace").click();
    expect(await toastText(browser)).toContain(
      "Workspace « Mes projets » supprimé · 2 projets conservés dans le référentiel.",
    );
    expect(readState(sandbox).workspaces.map((w) => w.name)).toEqual(["Boutique"]);

    await browser.$(".toast").$("button=Annuler").click();
    await browser.waitUntil(() => readState(sandbox).workspaces.length === 2);
    await dismissToast();
  });

  it("le référentiel liste chaque projet une fois avec ses workspaces (2e, P9)", async () => {
    await browser.$('[role="tab"]=Référentiel des projets').click();
    await waitForHeading(browser, "2 projets");

    expect(await browser.$$("tbody th").map((row) => row.getText())).toEqual([
      "boutique-tests",
      "paiements-v3",
    ]);
    expect(await browser.$$("tbody tr:first-child .chip").map((chip) => chip.getText())).toEqual([
      "Boutique",
      "Mes projets",
    ]);
    await screenshot(browser, "05-referentiel");
    await browser.$("button=Retour au workspace").click();
    await waitForHeading(browser, "Boutique");
  });

  it("au redémarrage, rouvre le dernier workspace avec ses projets (W2, R7)", async () => {
    await restart();

    await waitForHeading(browser, "Boutique");
    expect(await cardNames(browser)).toEqual(["boutique-tests", "paiements-v3"]);
  });

  it("signale un dossier supprimé hors d'Arnod sans planter (P10)", async () => {
    rmSync(path.join(sandbox.home, "arnod", "paiements-v3"), { recursive: true });
    await restart();

    const card = browser.$("main").$("article*=paiements-v3");
    expect(await card.getText()).toContain("Dossier introuvable");
    await browser.$("aside nav").$("button*=paiements-v3").click();
    expect(await toastText(browser)).toContain(
      "Le dossier du projet « paiements-v3 » est introuvable",
    );
    expect(await heading(browser)).toBe("Boutique");
  });

  it("inscrit un dossier projet existant par son manifeste (P12)", async () => {
    const folder = path.join(sandbox.home, "dev", "catalogue-legacy");
    mkdirSync(folder, { recursive: true });
    writeFileSync(
      path.join(folder, "arnod.yml"),
      "version: 1\nid: 7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10\nname: catalogue-legacy\n",
    );
    const workspaceId = readState(sandbox).lastWorkspaceId;

    // Le sélecteur natif ne se pilote pas : on appelle la commande Rust réelle.
    await browser.execute(
      (target, workspace) =>
        (
          window as unknown as {
            __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
          }
        ).__TAURI_INTERNALS__.invoke("project_add_existing", {
          path: target,
          workspaceId: workspace,
        }),
      folder,
      workspaceId,
    );
    await restart();

    await waitForHeading(browser, "Boutique");
    expect(await cardNames(browser)).toContain("catalogue-legacy");
    const added = readState(sandbox).projects.find((p) => p.name === "catalogue-legacy");
    expect(added?.id).toBe("7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10");
  });

  it("oublie un projet sans toucher à ses fichiers, puis annule (P11, R6)", async () => {
    await openSwitcher();
    await browser.$("button*=Référentiel des projets").click();
    await waitForHeading(browser, "3 projets");

    await browser.$('button[aria-label="Oublier catalogue-legacy"]').click();
    await waitForHeading(browser, "2 projets");

    expect(await toastText(browser)).toContain("Ses fichiers restent sur le disque.");
    const state = readState(sandbox);
    expect(state.projects.map((p) => p.name)).not.toContain("catalogue-legacy");
    expect(state.workspaces.flatMap((w) => w.projectIds)).not.toContain(
      "7f3c2a10-5b1e-4c8d-9a2f-0e6b4d3c2a10",
    );
    expect(existsSync(path.join(sandbox.home, "dev", "catalogue-legacy", "arnod.yml"))).toBe(true);

    await browser.$(".toast").$("button=Annuler").click();
    await waitForHeading(browser, "3 projets");
    expect(readState(sandbox).projects.map((p) => p.name)).toContain("catalogue-legacy");
  });
});
