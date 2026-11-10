# CLAUDE.md - Règles pour Claude Code

Arnod est une application de bureau de test d'API (remplaçant de SoapUI et Bruno).
Ce dépôt part du **squelette technique** décrit dans `PRD-squelette.md` : Tauri 2,
frontend Vue 3 + TypeScript + Vite + Pinia, cœur natif Rust. Toute nouvelle
fonctionnalité part de cette base et respecte les règles ci-dessous.

Travail en cours : premier usage (arborescence, requêtes, environnements, vérifications, run),
selon `docs/plan-premier-usage.md` (cœur Rust et interface livrés ; restent les tests e2e, la mesure de
l'installeur et les écarts listés en fin de plan). Livré : projets et
workspaces, selon `docs/plan-projets-workspaces.md` (maquettes dans `docs/maquettes/`), puis
workspaces en onglets selon `docs/plan-workspaces-onglets.md` ; voir la section « Projets et
workspaces ». Livré ensuite : fenêtres et zones selon `docs/plan-fenetres-zones.md` ; voir la
section « Fenêtres et zones ». Puis contrats de services selon `docs/plan-contrats-services.md`
(section « Contrats de services ») et format OpenCollection strict selon
`docs/plan-opencollection.md` (section « Format OpenCollection »), puis configuration d'un
endpoint selon `docs/plan-configuration-endpoint.md` (section « Configuration d'un endpoint ») et
barre latérale du projet selon `docs/plan-barre-laterale-projet.md` (section « Barre latérale du
projet »). Un nouveau lot commence par un plan rédigé à partir de `docs/modele-plan.md`.

## Architecture

- **Le frontend ne contient aucune logique métier.** Il affiche l'état et appelle des
  commandes Rust typées. Le travail (HTTP, SOAP, mocks, scripts, assertions) se fait
  dans le cœur Rust.
- Chaîne d'appel : composant Vue → store Pinia (`src/stores/`) → `ipc` (`src/lib/ipc.ts`)
  → `invoke` → commande Rust (`src-tauri/src/commands/`).
- Les composants et les stores n'importent **jamais** `@tauri-apps/api/core` ni
  `@tauri-apps/api/event` : seul `src/lib/ipc.ts` appelle `invoke` et écoute les événements du
  cœur (`events`, `EVENTS`).
- Les commandes Rust vivent dans `src-tauri/src/commands/`, un fichier par domaine
  (`hello.rs`, plus tard `http.rs`, `mocks.rs`...).
- Les types échangés sont écrits à la main dans `src/types/ipc.ts` et doivent refléter
  exactement la sérialisation serde (`#[serde(rename_all = "camelCase")]`).
- Une commande qui peut échouer renvoie `Result<T, CommandError>` (`src-tauri/src/error.rs`) :
  `kind` donne la nature de l'erreur, `message` un texte en français prêt à afficher. Côté
  interface, `IpcError` reprend `kind` (ou `null` si la cause n'est pas une `CommandError`) et
  `message`. Une nouvelle nature s'ajoute à `CommandErrorKind` en Rust, dans `src/types/ipc.ts`
  et dans `COMMAND_ERROR_KINDS` (`src/lib/ipc.ts`) ; un test Rust vérifie la concordance.

### Ajouter une commande Rust

1. Écrire la fonction `#[tauri::command]` dans `src-tauri/src/commands/<domaine>.rs`
   (déclarer le module dans `commands/mod.rs`). Garder la logique dans des fonctions
   pures testables, la commande n'étant qu'une façade. Si elle peut échouer, elle renvoie
   `Result<T, CommandError>`.
2. L'enregistrer dans `tauri::generate_handler![...]` (`src-tauri/src/lib.rs`).
3. Ajouter son nom à `CommandName` et ses types dans `src/types/ipc.ts` (un test Rust vérifie
   que `CommandName` concorde avec `generate_handler!`, et les unions miroirs avec le Rust).
4. L'exposer dans l'objet `ipc` de `src/lib/ipc.ts`.
5. Tester : `#[cfg(test)]` côté Rust (dont la forme JSON sérialisée, erreurs comprises),
   Vitest avec `mockIPC` côté frontend (`tests/`).
6. Si la commande a besoin d'un plugin ou d'une permission, l'ajouter dans
   `src-tauri/capabilities/default.json` avec le strict minimum.

## Arborescence

```
src/                    Frontend Vue
  main.ts               Création de l'app + Pinia
  App.vue               Démarrage : catalogue et fenêtres, puis fenêtre principale, de contenu ou
                        barre latérale détachée ; ou erreur bloquante
  styles.css            Thème clair/sombre (variables CSS), police système
  lib/ipc.ts            Wrapper typé autour d'invoke et des événements (seul point d'accès IPC)
  lib/format.ts         Formatage d'affichage
  lib/url.ts            Paramètres de chemin d'une adresse, pour l'affichage
  stores/               catalog (instantané), navigation (onglet, vue, titre), toasts (notifications),
                        windows (fenêtres, zones, onglets de contenu), workbench (par projet :
                        arborescence, environnements, run ; par onglet : brouillon, réponse),
                        contracts (contrats par projet, contrats ouverts)
  composables/          Actions partagées : projets (useProjectActions), onglets (useTabActions)
  components/ui/        Composants de base : bouton, dialogue, menu, interrupteur, notification
  components/layout/    Coquille : onglets des workspaces, barre latérale (un SidebarProject par
                        projet), zone principale
  components/zones/     Zones : arbre, onglets, séparations (SplitHandle), onglet de requête
  components/windows/   Fenêtres détachées : contenu, barre latérale
  components/contracts/ Contrats : section de la barre latérale, dialogue d'ajout, onglet
  components/projects/  Vues des projets (premier lancement...)
  components/workspaces/ Vues des workspaces
  components/workbench/ Arborescence, éditeur, réponse, environnements, run
  types/ipc.ts          Types IPC documentés à la main
tests/                  Tests Vitest (happy-dom, mockIPC, faux cœur fakeCore.ts), dont contrastes
e2e/                    Tests e2e sur l'application réelle (tauri-driver, WebdriverIO)
src-tauri/              Cœur Rust
  tauri.conf.json       Fenêtre, CSP, bundles (NSIS currentUser, AppImage)
  capabilities/         Permissions de la WebView
  src/lib.rs            Builder Tauri, plugin dialog, état, enregistrement des commandes
  src/error.rs          CommandError, erreur commune des commandes
  src/catalog/          Règles des projets et workspaces (pur : ni Tauri, ni disque)
  src/storage.rs        Lecture / écriture atomique d'arnod.json
  src/request/          Requête : modèle, fichier, résolution des variables, vérifications,
                        héritage des dossiers (inherit.rs), réponse attendue (example.rs)
  src/tree/             Arborescence d'un projet sur disque (dossiers, requêtes, corbeille)
  src/environment.rs    Environnements d'un projet (fichiers, chaîne, protection)
  src/local.rs          Données locales d'un projet (hors dépôt : local.json)
  src/secrets.rs        Coffre des secrets (trait SecretStore, keyring du système)
  src/http.rs           Envoi HTTP (reqwest), délai, annulation, troncature à 5 Mo
  src/workbench.rs      Établi : envoi, variables, environnements actifs, runs
  src/layout/           Zones, onglets et fenêtres (pur : ni Tauri, ni disque)
  src/contract/         Contrats OpenAPI : lecture, exemples, requêtes créées, conformité ;
                        file.rs range les contrats dans contracts/ du projet
  src/windows.rs        windows.json, fenêtres natives, focus, fermetures, événements arnod://
  src/opencollection/   Format OpenCollection 1.0.0 : schéma embarqué, YAML, requêtes, racine
                        (opencollection.yml, extensions.arnod), conversion de l'ancien format
  src/manifest.rs       Dossier d'un projet et sa racine opencollection.yml
  src/state.rs          AppState : catalogue + stockage + annulation (app.manage)
  src/menu.rs           Menu natif (Fichier > Quitter, Fenêtre > Dispositions, détacher, rattacher)
  src/commands/         Commandes, un fichier par domaine
docs/                   Plans, modèle de plan, maquettes, captures du README, schéma
                        OpenCollection 1.0.0 (docs/opencollection/)
scripts/                Mesure de taille, concordance des versions, tests de fumée (CI)
.github/                Modèles d'issue et de pull request, Dependabot
  workflows/            CI de build Windows et Linux, contrôle des titres de pull request
```

## Commandes

```sh
npm ci                      # dépendances frontend (Node 22+)
npm run tauri dev           # application en développement (Vite sur le port 1420)
npm run tauri build         # build release : NSIS sous Windows, AppImage sous Linux

npm run check               # avant chaque commit : tout ce que vérifie la CI hors build
npm test                    # Vitest
npm run build:e2e           # binaire debug pour les tests e2e (Linux)
xvfb-run -a npm run test:e2e # e2e sur l'application réelle (voir e2e/README.md)
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
- Interface : réutiliser les composants de `src/components/ui/` (`BaseButton`, `BaseDialog` sur
  `<dialog>` natif, `BaseMenu` accessible au clavier, `ToggleSwitch`, `SearchInput`,
  `ColorDot`) ; navigation par le store `navigation` (pas de routeur) ; notifications par le
  store `toasts`. Chaque composant interactif est testé au clavier.
- ESLint (`vueTsConfigs.strictTypeChecked`) et Prettier sans erreur. Imports de types
  avec `import type`. Alias `@/` pour `src/`.
- Rust : `cargo fmt` et `cargo clippy -D warnings` sans avertissement. Pas de `unwrap()`
  hors tests ; erreurs de commande renvoyées en `Result<T, CommandError>`.
- Commentaires, messages et textes d'interface en français.
- Commits en [Conventional Commits](https://www.conventionalcommits.org/fr/) :
  `feat:`, `fix:`, `chore:`, `docs:`, `test:`, `ci:`, `refactor:`... Le titre d'une pull request
  suit la même forme (vérifié par `.github/workflows/pr-title.yml`) et sa description reprend les
  sections de `.github/pull_request_template.md`.
- Chaque tâche est relue avant commit ; `npm run check` passe avant chaque commit.
- Une version se publie par un tag `vX.Y.Z` sur `main` (README, « Publier une version ») : la CI
  crée la release. Jamais de release créée à la main ; `scripts/check-version.mjs` garde les six
  sources de version concordantes.
- Tests : règles dans `cargo test` (vrais dossiers temporaires pour le stockage), parcours
  d'interface dans Vitest avec le faux cœur `tests/fakeCore.ts`, parcours complets sur
  l'application réelle dans `e2e/` (à étendre à chaque nouvelle fonctionnalité visible).

## Contraintes à ne pas casser

- Installation **sans droits administrateur** : `bundle.windows.nsis.installMode` reste
  `currentUser`. Ne jamais passer à `perMachine` ou `both`.
- `webviewInstallMode` vaut `downloadBootstrapper`. Pour un poste hors ligne, passer à
  `embedBootstrapper` (+1,8 Mo).
- Installeur Windows < 15 Mo (vérifié par la CI, `scripts/bundle-size.mjs`).
- CSP stricte dans `tauri.conf.json` : pas de ressource distante, pas de `unsafe-eval`.
- Pas de nouveau plugin Tauri ni de nouvelle permission sans besoin explicite. Seul ajout
  validé : `tauri-plugin-dialog`, appelé uniquement depuis Rust (ni paquet npm, ni
  permission WebView). `reqwest`, `keyring`, `trash` et `tokio` sont des crates Rust, sans permission WebView.
- Thème clair/sombre suivant le système via les variables de `src/styles.css` (palette des
  maquettes, accent orange) : utiliser `var(--color-*)`, jamais de couleur codée en dur dans
  les composants. Tout nouveau jeton est défini pour les deux thèmes, en `#rrggbb` ;
  `tests/theme.spec.ts` vérifie les contrastes.
- Police système uniquement : la CSP interdit les polices distantes des maquettes.
- Hors périmètre actuel : SOAP (et donc WSDL), mocks, scripts, CLI, signature de code, updater. Les
  requêtes HTTP sont livrées (premier usage), les contrats OpenAPI aussi ; les secrets vont dans le
  coffre du système, jamais dans le projet.
- Une tâche de fond lancée depuis une commande passe par `tauri::async_runtime::spawn` : une
  commande synchrone ne s'exécute pas dans un contexte tokio (`tokio::spawn` y panique).

## Projets et workspaces

Référence : `docs/plan-projets-workspaces.md` (cas d'usage P1 à P12 et W1 à W11, règles R1 à
R11), révisé par `docs/plan-workspaces-onglets.md` (cas O1 à O12, règles R3, R4, R7, R8, R11 et
R12). Cocher une tâche dans le plan quand elle est livrée.

- « Tous les projets » est le référentiel, affiché dans l'onglet épinglé ; ce n'est pas un
  workspace. Il n'y a plus de workspace par défaut : un projet est dans zéro, un ou plusieurs
  workspaces. Les onglets suivent l'ordre de création ; `lastWorkspaceId` vaut `null` pour
  « Tous les projets ».
- Les règles vivent dans `src-tauri/src/catalog/`, module pur sans dépendance à Tauri ni au
  disque, testé règle par règle (`catalog/tests.rs`). L'interface affiche l'état et ne fait que
  des filtres et comptages d'affichage.
- Une commande qui modifie le catalogue passe par `AppState::mutate` (ou `mutate_undoable`) :
  copie, règle, écriture sur disque, remplacement. Elle renvoie l'instantané complet
  (`CatalogSnapshot`) ; le store remplace son état. Les actions annulables renvoient aussi un
  jeton pour `undo`. Les types de réponse sont dans `catalog/snapshot.rs`.
- État de l'application : `arnod.json` (format 2) dans `app_local_data_dir()`, écrit de façon
  atomique (fichier temporaire puis renommage). Un format 1 est migré à la lecture (R12). Un
  fichier illisible ou écrit par une version plus récente n'est jamais écrasé.
- Glisser-déposer HTML : `dragDropEnabled: false` sur la fenêtre (sinon WebView2 l'intercepte
  sous Windows). Toute action glissée a son équivalent au clavier (« Ajouter à… »).
- Un projet est un dossier contenant `opencollection.yml` (voir « Format OpenCollection »).
  Arnod n'initialise pas Git.
- Noms de projet et de workspace uniques, sans tenir compte de la casse. « Changer » choisit
  l'emplacement parent : le projet est créé dans `<emplacement>/<nom>`.
- Pas de confirmation quand rien n'est perdu : une notification avec « Annuler ».

## Fenêtres et zones

Référence : `docs/plan-fenetres-zones.md` (cas F1 à F14, règles Z1 à Z8).

- La disposition vit dans le cœur : `src-tauri/src/layout/` (pur, testé dans `layout/tests.rs`)
  tient les onglets, les zones, l'arbre des séparations et les fenêtres ; `windows.rs` l'enregistre
  dans `windows.json` (même dossier qu'`arnod.json`, écriture atomique) et crée ou ferme les
  fenêtres natives. Chaque commande `layout_*` ou `window_*` renvoie l'état de toutes les fenêtres
  et le diffuse par `arnod://fenetres` ; le store `windows` le remplace. Seul l'agrandissement d'une
  zone (⤢) reste local à la fenêtre.
- Fenêtres : `main` (principale) et `fenetre-<8 hex>` (contenu ou barre latérale détachée). La
  capacité `default` couvre `["main", "fenetre-*"]`, sans permission ajoutée. Les fenêtres sont
  créées par Rust, hors du fil de la commande (interblocage WebView2 sous Windows).
- Interconnexion : toute modification du catalogue émet `arnod://catalogue` ; l'arborescence, une
  requête enregistrée ou les environnements émettent `arnod://projet` (avec la fenêtre d'origine,
  qui l'ignore). Une nouvelle commande qui modifie un projet émet ce même événement.
- Fermer une fenêtre détachée la rattache (Z6) : le cœur émet `arnod://fermer`, la fenêtre
  enregistre ses requêtes puis appelle `window_reattach`. Fermer la principale quitte (Z7).
- Toute séparation passe par `SplitHandle` (motif WAI-ARIA « window splitter », minimums en pixels) ;
  toute action glissée (onglet, zone) a son équivalent au clavier ou dans le menu de l'onglet.

## Contrats de services

Référence : `docs/plan-contrats-services.md` (cas C1 à C12, règles R1 à R12).

- Un contrat est un fichier OpenAPI 3.0 ou 3.1, ou Swagger 2.0 (YAML ou JSON), rangé tel quel dans
  `contracts/` à la racine du projet. Ce dossier est réservé comme `environments/` : absent de
  l'arborescence, refusé comme nom de dossier racine. Un texte qui n'est pas un contrat lisible
  n'est jamais écrit.
- Les règles vivent dans `src-tauri/src/contract/` : `parse`, `sample`, `generate` et
  `conformance` sont purs, `file` touche au disque ; tests règle par règle dans
  `contract/tests.rs`. Pas de dépendance ajoutée : la validation couvre un sous-ensemble de JSON
  Schema (pas `pattern` ni `format`).
- Une requête créée depuis une opération est liée au contrat par l'étiquette
  `contract:<fichier>#<opération>` ; l'établi vérifie toute requête liée contre le contrat (résultat
  `contrat`), sans vérification écrite. Une opération ne crée qu'une requête ; `baseUrl` vide
  prend l'adresse du premier serveur du contrat.
- Les commandes `contract_*` qui modifient émettent `arnod://projet` en portée `contracts` (et
  `tree` pour les requêtes créées) ; supprimer un contrat ferme ses onglets dans toutes les
  fenêtres.

## Format OpenCollection

Référence : `docs/plan-opencollection.md` (cas S1 à S8, règles R1 à R8). Schéma officiel 1.0.0
conservé dans `docs/opencollection/` et embarqué dans le cœur.

- Strict : tout fichier écrit (racine, requête, `folder.yml`, environnement) est validé contre le
  schéma avant écriture ; un contenu non conforme n'est jamais écrit, un fichier lu non conforme
  est listé illisible et jamais réécrit. Aucun champ inventé.
- Ce qui est propre à Arnod va seulement dans `extensions.arnod` de `opencollection.yml`
  (identifiant, ordre et protection des environnements) et dans les étiquettes `info.tags` d'une
  requête (lien au contrat). Identifiants et origines des vérifications ne sont pas écrits.
- `src-tauri/src/opencollection/` fait la correspondance avec le modèle de l'interface
  (`request/model.rs`, qui n'est plus écrit tel quel). Enregistrer fusionne les champs gérés dans
  le fichier existant : scripts, variables et actions (signalés par `preserved`), la partie
  `request` des exemples, les autres authentifications et corps sont conservés.
- Un projet au format précédent (`arnod.yml`) est converti une fois à l'ouverture
  (`opencollection::migrate`). Une version de spécification de majeure supérieure à 1 ou une
  collection regroupée (`bundled`) sont refusées sans écriture.

## Configuration d'un endpoint

Référence : `docs/plan-configuration-endpoint.md` (cas EP1 à EP10, règles R1 à R9), maquettes
`docs/maquettes/5a-endpoint.png` et `5b-endpoint-reponse-attendue.png`.

- Un endpoint est une requête du projet. Tout se modifie sur place et s'enregistre seul : le store
  `workbench` enregistre 600 ms après la dernière modification (`AUTOSAVE_MS`), l'éditeur affiche
  « ✓ Enregistré », « Non enregistré · Réessayer » sur un échec. Plus de bouton « Enregistrer ».
- Éditeur : `RequestEditor.vue` (en-tête, nom, description, étiquettes, paramètres, en-têtes, docs)
  délègue à `BodyEditor`, `AuthEditor`, `SettingsEditor` et `AssertionsEditor` (`defineModel`).
  Réponses attendues : `ExpectedResponses.vue`, onglet du panneau de résultat (`RequestTab.vue`).
- Tout s'écrit dans le fichier OpenCollection : `info.description`, `info.tags` (l'étiquette
  `contract:` n'est jamais montrée), descriptions des lignes, variantes de corps, `settings`
  (absent = hérité), `docs`, `examples` (la partie `request` d'un exemple est conservée).
- Héritage (`request/inherit.rs`) : `folder.yml` du dossier puis des parents, puis
  `opencollection.yml` (`request.auth`, `request.headers`, `request.settings.http`), puis les
  défauts. Calculé à la lecture (`RequestDoc.inherited`) et à l'envoi (`request_send` reçoit `path`).
- Paramètres de chemin : segments `:nom` ou `{nom}` de l'adresse ; le cœur les aligne sur
  l'adresse à l'enregistrement (`normalize_path_params`), `lib/url.ts` les liste pour l'affichage.

## Barre latérale du projet

Référence : `docs/plan-barre-laterale-projet.md` (cas B1 à B6), maquette « Arnod - Barre latérale
du projet » (option a), captures `docs/maquettes/6*.png`.

- Un projet = `SidebarProject.vue` : chevron, pastille, nom, puis ▶ (exécuter), + (ajouter) et ⋯
  (autres), visibles au survol, au focus et sur le projet ouvert ; sous le nom, la pastille
  d'environnement (`EnvironmentChip`) ; puis `TreePanel` et `ContractsSection`, dont les créations
  passent par leurs méthodes exposées. Un projet replié montre sa pastille compacte à droite.
- Toute ligne de la barre latérale suit la grille `--tree-pad`, `--tree-indent`, `--tree-lead`,
  `--tree-gap` (définie par `AppSidebar`) : une colonne de tête de largeur fixe, le libellé
  après ; une nouvelle ligne la reprend au lieu de marges propres.
- Menus par `BaseMenu` (entrées `radio`, `swatch`, `tag`, `detail` pour les environnements) ; pas
  de texte d'aide vide : une ligne d'ajout (« + Nouvelle requête », « + Ajouter un contrat »).
- Raccourcis : Ctrl+N (nouvelle requête) et Ctrl+Maj+E (environnements) sur le projet de l'onglet
  actif, sinon le projet ouvert, sinon le premier déplié.
