/**
 * Pilotage de l'application réelle pour les tests e2e.
 *
 * `tauri-driver` relaie WebDriver vers le pilote natif (WebKitWebDriver sous
 * Linux) et lance le binaire debug d'Arnod. Chaque bac à sable fournit un
 * dossier personnel et un dossier de données vierges : l'application crée ses
 * projets et son `arnod.json` sur un vrai disque, isolé de l'utilisateur.
 */
import { type ChildProcess, spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";

import { remote } from "webdriverio";

export const ROOT = path.resolve(import.meta.dirname, "..");
export const APPLICATION = path.join(
  ROOT,
  "src-tauri",
  "target",
  "debug",
  process.platform === "win32" ? "arnod.exe" : "arnod",
);
export const ARTIFACTS = path.join(ROOT, "e2e", "artifacts");
const IDENTIFIER = "io.github.juliencorre.arnod";
const DRIVER_PORT = 4444;

export type Browser = Awaited<ReturnType<typeof remote>>;

/** Dossiers d'un bac à sable : personnel (projets) et données (arnod.json). */
export interface Sandbox {
  root: string;
  home: string;
  dataDir: string;
}

export function createSandbox(): Sandbox {
  const root = mkdtempSync(path.join(tmpdir(), "arnod-e2e-"));
  const home = path.join(root, "home");
  mkdirSync(home, { recursive: true });
  return { root, home, dataDir: path.join(root, "data", IDENTIFIER) };
}

export function removeSandbox(sandbox: Sandbox): void {
  // L'application peut encore écrire son cache graphique en se fermant : on réessaie.
  rmSync(sandbox.root, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
}

/** État enregistré par l'application (`arnod.json`). */
export function readState(sandbox: Sandbox): {
  projects: { id: string; name: string; path: string }[];
  workspaces: { id: string; name: string; projectIds: string[] }[];
  defaultWorkspaceId: string;
  lastWorkspaceId: string;
} {
  return JSON.parse(readFileSync(path.join(sandbox.dataDir, "arnod.json"), "utf8")) as ReturnType<
    typeof readState
  >;
}

function waitForPort(port: number, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolve, reject) => {
    const attempt = (): void => {
      const socket = connect(port, "127.0.0.1");
      socket.once("connect", () => {
        socket.end();
        resolve();
      });
      socket.once("error", () => {
        socket.destroy();
        if (Date.now() > deadline) reject(new Error(`tauri-driver n'écoute pas sur ${port}`));
        else setTimeout(attempt, 200);
      });
    };
    attempt();
  });
}

/**
 * Lance `tauri-driver` avec l'environnement du bac à sable : l'application qu'il
 * démarre hérite de HOME (projets dans ~/arnod) et XDG_DATA_HOME (arnod.json).
 */
export async function startDriver(
  sandbox: Sandbox,
  extraEnv: Record<string, string> = {},
): Promise<ChildProcess> {
  const driver = spawn("tauri-driver", ["--port", String(DRIVER_PORT)], {
    env: {
      ...process.env,
      HOME: sandbox.home,
      XDG_DATA_HOME: path.dirname(sandbox.dataDir),
      XDG_CONFIG_HOME: path.join(sandbox.root, "config"),
      // Caches graphiques hors du bac à sable : WebKit peut encore y écrire en se
      // fermant, ce qui empêcherait de le supprimer.
      XDG_CACHE_HOME: path.join(tmpdir(), "arnod-e2e-cache"),
      MESA_SHADER_CACHE_DISABLE: "true",
      ...extraEnv,
    },
    stdio: ["ignore", "inherit", "inherit"],
  });
  await waitForPort(DRIVER_PORT, 30_000);
  return driver;
}

export async function stopDriver(driver: ChildProcess | undefined): Promise<void> {
  if (!driver || driver.exitCode !== null) return;
  const exited = new Promise((resolve) => driver.once("exit", resolve));
  driver.kill();
  await exited;
}

/** Démarre Arnod (nouvelle session WebDriver) et attend son premier écran. */
export async function launchApp(): Promise<Browser> {
  const browser = await remote({
    hostname: "127.0.0.1",
    port: DRIVER_PORT,
    logLevel: "error",
    capabilities: {
      // WebKitWebDriver ne parle pas WebDriver BiDi : WebDriver classique.
      "wdio:enforceWebDriverClassic": true,
      "tauri:options": { application: APPLICATION },
    } as WebdriverIO.Capabilities,
  });
  await browser.$("main h1").waitForDisplayed({ timeout: 30_000 });
  return browser;
}

/** Texte du titre principal de la vue courante. */
export async function heading(browser: Browser): Promise<string> {
  return browser.$("main h1").getText();
}

/** Attend que le titre principal vaille `text`. */
export async function waitForHeading(browser: Browser, text: string): Promise<void> {
  await browser.waitUntil(async () => (await heading(browser)) === text, {
    timeout: 10_000,
    timeoutMsg: `titre « ${text} » attendu, obtenu « ${await heading(browser)} »`,
  });
}

/** Noms des cartes de projets de la vue workspace. */
export async function cardNames(browser: Browser): Promise<string[]> {
  return browser.$$("main article h2").map((title) => title.getText());
}

/** Texte de la notification courante. */
export async function toastText(browser: Browser): Promise<string> {
  const toast = browser.$(".toast");
  await toast.waitForDisplayed({ timeout: 10_000 });
  return toast.getText();
}

/** Thème demandé à GTK, donc à WebKitGTK (`ARNOD_E2E_THEME=dark` pour le thème sombre). */
export const THEME = process.env.ARNOD_E2E_THEME === "dark" ? "dark" : "light";

/** Variables d'environnement propres au thème de la session. */
export function themeEnv(): Record<string, string> {
  return THEME === "dark" ? { GTK_THEME: "Adwaita:dark" } : {};
}

/**
 * Capture de la fenêtre dans `e2e/artifacts/`. WebKit renvoie la dernière image
 * peinte : on attend deux images d'animation pour capturer l'état courant.
 */
export async function screenshot(browser: Browser, name: string): Promise<void> {
  mkdirSync(ARTIFACTS, { recursive: true });
  await browser.execute(
    () =>
      new Promise<void>((resolve) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => {
            resolve();
          });
        });
      }),
  );
  await browser.pause(300);
  await browser.saveScreenshot(path.join(ARTIFACTS, `${name}-${THEME}.png`));
}
