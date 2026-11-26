#!/usr/bin/env node
// Détecteur de la skill Impeccable (.claude/skills/impeccable/) sur les sources de l'interface :
// règles écrites (barres de sélection sur le côté, tailles hors de l'échelle de DESIGN.md…), sans
// IA ni clé. Le lanceur de la skill télécharge une fois son moteur, vérifié par empreinte
// (engine-checksums.txt). Une exception se déclare en commentaire à côté de la valeur
// (`impeccable-disable-line <règle>: raison`) ; une exception qui ne sert plus fait échouer.
// En CI (GITHUB_ACTIONS), les constats deviennent des annotations de la pull request.
//
// Usage : node scripts/impeccable-detect.mjs [cible…]   (par défaut : src/)
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const scripts = path.join(root, ".claude", "skills", "impeccable", "scripts");
const windows = process.platform === "win32";
const launcher = path.join(scripts, windows ? "impeccable.cmd" : "impeccable");
const targets = process.argv.slice(2);
const args = [
  "detect",
  "--report-unused-ignores",
  ...(process.env.GITHUB_ACTIONS === "true" ? ["--github"] : []),
  ...(targets.length > 0 ? targets : ["src/"]),
];

// Interpréteur par chemin absolu, jamais cherché dans PATH (comme startup::open_external du cœur).
const shell = windows
  ? path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "cmd.exe")
  : "/bin/sh";
const shellArgs = windows ? ["/d", "/s", "/c", launcher, ...args] : [launcher, ...args];
const result = spawnSync(shell, shellArgs, { cwd: root, stdio: "inherit" });
if (result.error) {
  console.error(`Détecteur Impeccable indisponible : ${result.error.message}`);
  process.exit(1);
}
if (result.status === 2) {
  console.error(
    "Détecteur Impeccable : constats ci-dessus. Corriger, ou déclarer l'exception à côté de la " +
      "valeur (impeccable-disable-line <règle>: raison).",
  );
}
process.exit(result.status ?? 1);
