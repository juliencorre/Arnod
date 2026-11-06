# CLAUDE.md - Règles pour Claude Code

Arnod est une application de bureau de test d'API (remplaçant de SoapUI et Bruno).
Ce dépôt est le **squelette technique** décrit dans `PRD-squelette.md` : Tauri 2,
frontend Vue 3 + TypeScript + Vite + Pinia, cœur natif Rust. Toute nouvelle
fonctionnalité part de cette base et respecte les règles ci-dessous.

## Architecture

- **Le frontend ne contient aucune logique métier.** Il affiche l'état et appelle des
  commandes Rust typées. Le travail (HTTP, SOAP, mocks, scripts, assertions) se fait
  dans le cœur Rust.
- Chaîne d'appel : composant Vue → store Pinia (`src/stores/`) → `ipc` (`src/lib/ipc.ts`)
  → `invoke` → commande Rust (`src-tauri/src/commands/`).
- Les composants et les stores n'importent **jamais** `@tauri-apps/api/core` : seul
  `src/lib/ipc.ts` appelle `invoke`.
- Les commandes Rust vivent dans `src-tauri/src/commands/`, un fichier par domaine
  (`hello.rs`, plus tard `http.rs`, `mocks.rs`...).
- Les types échangés sont écrits à la main dans `src/types/ipc.ts` et doivent refléter
  exactement la sérialisation serde (`#[serde(rename_all = "camelCase")]`).

### Ajouter une commande Rust

1. Écrire la fonction `#[tauri::command]` dans `src-tauri/src/commands/<domaine>.rs`
   (déclarer le module dans `commands/mod.rs`). Garder la logique dans des fonctions
   pures testables, la commande n'étant qu'une façade.
2. L'enregistrer dans `tauri::generate_handler![...]` (`src-tauri/src/lib.rs`).
3. Ajouter son nom à `CommandName` et ses types dans `src/types/ipc.ts`.
4. L'exposer dans l'objet `ipc` de `src/lib/ipc.ts`.
5. Tester : `#[cfg(test)]` côté Rust (dont la forme JSON sérialisée), Vitest avec
   `mockIPC` côté frontend (`tests/`).
6. Si la commande a besoin d'un plugin ou d'une permission, l'ajouter dans
   `src-tauri/capabilities/default.json` avec le strict minimum.

## Arborescence

```
src/                    Frontend Vue
  main.ts               Création de l'app + Pinia
  App.vue               Page principale
  styles.css            Thème clair/sombre (variables CSS), police système
  lib/ipc.ts            Wrapper typé autour d'invoke (seul point d'accès IPC)
  lib/format.ts         Formatage d'affichage
  stores/               Stores Pinia
  types/ipc.ts          Types IPC documentés à la main
tests/                  Tests Vitest (happy-dom, mockIPC)
src-tauri/              Cœur Rust
  tauri.conf.json       Fenêtre, CSP, bundles (NSIS currentUser, AppImage)
  capabilities/         Permissions de la WebView
  src/lib.rs            Builder Tauri, enregistrement des commandes
  src/menu.rs           Menu natif (Fichier > Quitter)
  src/commands/         Commandes, un fichier par domaine
scripts/                Mesure de taille et tests de fumée (CI)
.github/workflows/      CI de build Windows et Linux
```

## Commandes

```sh
npm ci                      # dépendances frontend (Node 22+)
npm run tauri dev           # application en développement (Vite sur le port 1420)
npm run tauri build         # build release : NSIS sous Windows, AppImage sous Linux

npm test                    # Vitest
npm run lint                # ESLint
npm run typecheck           # vue-tsc
npm run format              # Prettier (écriture) ; format:check pour vérifier

cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Linux : installer d'abord les dépendances système
(`libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev`).

## Conventions

- TypeScript strict (`strict`, `noUncheckedIndexedAccess`), pas de `any`.
- ESLint (`vueTsConfigs.strictTypeChecked`) et Prettier sans erreur. Imports de types
  avec `import type`. Alias `@/` pour `src/`.
- Rust : `cargo fmt` et `cargo clippy -D warnings` sans avertissement. Pas de `unwrap()`
  hors tests ; erreurs de commande renvoyées en `Result<T, E>` avec `E: Serialize`.
- Commentaires, messages et textes d'interface en français.
- Commits en [Conventional Commits](https://www.conventionalcommits.org/fr/) :
  `feat:`, `fix:`, `chore:`, `docs:`, `test:`, `ci:`, `refactor:`...
- Chaque tâche est relue avant commit ; lint, tests et clippy passent avant chaque commit.

## Contraintes à ne pas casser

- Installation **sans droits administrateur** : `bundle.windows.nsis.installMode` reste
  `currentUser`. Ne jamais passer à `perMachine` ou `both`.
- `webviewInstallMode` vaut `downloadBootstrapper`. Pour un poste hors ligne, passer à
  `embedBootstrapper` (+1,8 Mo).
- Installeur Windows < 15 Mo (vérifié par la CI, `scripts/bundle-size.mjs`).
- CSP stricte dans `tauri.conf.json` : pas de ressource distante, pas de `unsafe-eval`.
- Pas de nouveau plugin Tauri ni de nouvelle permission sans besoin explicite.
- Thème clair/sombre suivant le système via les variables de `src/styles.css` :
  utiliser `var(--color-*)`, jamais de couleur codée en dur dans les composants.
- Hors périmètre du squelette : requêtes HTTP/SOAP, mocks, scripts, CLI, signature de
  code, updater, persistance.
