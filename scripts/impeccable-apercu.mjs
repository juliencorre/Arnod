#!/usr/bin/env node
/**
 * Détecteur Impeccable sur l'aperçu rendu (application sur le faux cœur, tests/apercu) : les
 * règles qui demandent une page (contraste réel, titres sautés, texte recouvert…) que le passage
 * sur les sources (scripts/impeccable-detect.mjs) ne voit pas. Chaque scénario est ouvert dans
 * Chromium, puis scanné tel qu'il est montré, par le port DevTools. Outil de développement, hors
 * CI : Playwright est pris dans l'installation globale de Node, comme scripts/apercu.mjs.
 *
 * Usage :
 *   node scripts/impeccable-apercu.mjs [scénario…] [--sombre] [--json <fichier>]
 *   (sans scénario : tous ceux de tests/apercu/scenarios.ts)
 */
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const PORT = 9333;
const LAUNCHER = path.join(ROOT, ".claude", "skills", "impeccable", "scripts", "impeccable");

const argv = process.argv.slice(2);
const dark = argv.includes("--sombre");
const jsonAt = argv.indexOf("--json");
const out = jsonAt >= 0 ? argv[jsonAt + 1] : undefined;
let scenarios = argv.filter((a, i) => !a.startsWith("--") && (jsonAt < 0 || i !== jsonAt + 1));
if (scenarios.length === 0) {
  const src = readFileSync(path.join(ROOT, "tests", "apercu", "scenarios.ts"), "utf8");
  const key = /^ {2}(?:"([^"]+)"|([\w.-]+)): \{\n {4}description:/gm;
  scenarios = [...src.matchAll(key)].map(([, quoted, bare]) => quoted ?? bare);
}

const { chromium } = require("playwright");
const { createServer } = await import("vite");
const server = await createServer({
  configFile: path.join(ROOT, "vite.apercu.config.ts"),
  logLevel: "warn",
});
await server.listen();
const base = server.resolvedUrls.local[0];
const browser = await chromium.launch({
  args: [`--remote-debugging-port=${String(PORT)}`, ...(dark ? ["--force-dark-mode"] : [])],
});
const results = {};
let total = 0;
try {
  for (const scenario of scenarios) {
    const context = await browser.newContext({
      viewport: { width: 1280, height: 775 },
      colorScheme: dark ? "dark" : "light",
    });
    const page = await context.newPage();
    await page.goto(`${base}?scenario=${encodeURIComponent(scenario)}`);
    await page.waitForFunction(() => document.documentElement.dataset.apercu !== undefined, null, {
      timeout: 20000,
    });
    let json = "[]";
    try {
      json = execFileSync(
        "sh",
        [
          LAUNCHER,
          "detect",
          "--json",
          "--cdp",
          String(PORT),
          "--target",
          `scenario=${encodeURIComponent(scenario)}`,
        ],
        { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
      );
    } catch (error) {
      json = error.stdout || "[]";
      if (error.status === 1) console.error(`✕ ${scenario} : ${String(error.stderr)}`);
    }
    const findings = JSON.parse(json || "[]").filter((f) => !f.advisory);
    results[scenario] = findings;
    total += findings.length;
    for (const f of findings) {
      console.log(`${scenario} · ${f.antipattern} : ${String(f.snippet).replace(/\s+/g, " ")}`);
    }
    await context.close();
  }
} finally {
  await browser.close();
  await server.close();
}
if (out) writeFileSync(out, `${JSON.stringify(results, null, 2)}\n`);
console.log(
  `Impeccable sur l'aperçu (${dark ? "sombre" : "clair"}) : ${String(total)} constat(s), ` +
    `${String(scenarios.length)} scénario(s).`,
);
