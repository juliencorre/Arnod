#!/usr/bin/env node
// Vérifie que toutes les sources de version d'Arnod concordent et, si un tag est donné,
// qu'elles lui correspondent. Lancé par `npm run check`, par la CI à chaque build et avant
// chaque release.
//
// Usage : node scripts/check-version.mjs [vX.Y.Z]
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(path, "utf8");
const json = (path) => JSON.parse(read(path));

/** Version de la section `[package]` d'un Cargo.toml. */
function cargoPackageVersion(text) {
  let inPackage = false;
  for (const line of text.split(/\r?\n/)) {
    if (/^\s*\[/.test(line)) {
      inPackage = line.trim() === "[package]";
    } else if (inPackage) {
      const match = /^\s*version\s*=\s*"([^"]+)"/.exec(line);
      if (match) return match[1];
    }
  }
  return undefined;
}

/** Version du paquet `arnod` dans Cargo.lock. */
function cargoLockVersion(text) {
  return /^name = "arnod"\r?\nversion = "([^"]+)"/m.exec(text)?.[1];
}

const lock = json("package-lock.json");
const sources = [
  ["package.json", json("package.json").version],
  ["package-lock.json", lock.version],
  ['package-lock.json (packages[""])', lock.packages?.[""]?.version],
  ["src-tauri/Cargo.toml", cargoPackageVersion(read("src-tauri/Cargo.toml"))],
  ["src-tauri/Cargo.lock", cargoLockVersion(read("src-tauri/Cargo.lock"))],
  ["src-tauri/tauri.conf.json", json("src-tauri/tauri.conf.json").version],
];

const tag = process.argv[2];
if (tag !== undefined && !/^v\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(tag)) {
  console.error(`Le tag « ${tag} » n'a pas la forme vX.Y.Z (par exemple v0.0.2).`);
  process.exit(1);
}

const expected = tag === undefined ? sources[0][1] : tag.slice(1);
const wrong = sources.filter(([, version]) => version !== expected);

for (const [path, version] of sources) {
  const mark = version === expected ? "ok" : "DIFFÉRENT";
  console.log(`${mark.padEnd(9)} ${String(version ?? "introuvable").padEnd(12)} ${path}`);
}

if (wrong.length > 0) {
  const target = tag === undefined ? `à celle de package.json (${expected})` : `au tag ${tag}`;
  console.error(
    `\n${wrong.length} source(s) de version ne correspondent pas ${target}. ` +
      "Voir « Publier une version » dans le README.",
  );
  process.exit(1);
}
console.log(
  tag === undefined
    ? `\nVersion ${expected} partout.`
    : `\nVersion ${expected} partout, conforme au tag ${tag}.`,
);
