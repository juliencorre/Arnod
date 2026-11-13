# CLAUDE.md - Règles pour Claude Code

Arnod est une application de bureau de test d'API (remplaçant de SoapUI et Bruno).
Ce dépôt part du **squelette technique** décrit dans `PRD-squelette.md` : Tauri 2,
frontend Vue 3 + TypeScript + Vite + Pinia, cœur natif Rust. Toute nouvelle
fonctionnalité part de cette base et respecte les règles ci-dessous.

Premier usage (arborescence, requêtes, environnements, vérifications, run) selon
`docs/plan-premier-usage.md`, terminé par `docs/plan-fin-premier-usage.md` (écran du run de la
maquette 1f, statuts de la barre latérale, échecs ignorables, e2e) ; section « Run et statuts ».
Restent la mesure de l'installeur et les écarts listés en fin de plan du premier usage.
Livré : projets et workspaces, selon `docs/plan-projets-workspaces.md` (maquettes dans
`docs/maquettes/`), puis workspaces en onglets selon `docs/plan-workspaces-onglets.md` ; voir la section « Projets et
workspaces ». Livré ensuite : fenêtres et zones selon `docs/plan-fenetres-zones.md` ; voir la
section « Fenêtres et zones ». Puis contrats de services selon `docs/plan-contrats-services.md`
(section « Contrats de services ») et format OpenCollection strict selon
`docs/plan-opencollection.md` (section « Format OpenCollection »), puis configuration d'un
endpoint selon `docs/plan-configuration-endpoint.md` (section « Configuration d'un endpoint ») et
barre latérale du projet selon `docs/plan-barre-laterale-projet.md` (section « Barre latérale du
projet »), puis résultat détachable selon `docs/plan-resultat-detachable.md` (section « Fenêtres et
zones »), puis accueil, catalogue des étapes et Non classés selon `docs/plan-accueil-non-classes.md`
et l'étude `docs/etude-catalogue-etapes.md` (section « Accueil, catalogue des étapes et Non
classés »), puis sélecteur de workspace et icônes des sections selon
`docs/plan-selecteur-workspace.md` (section « Projets et workspaces »), puis requêtes rangées dans
leur contrat selon `docs/plan-requetes-dans-contrats.md` (sections « Contrats de services » et
« Barre latérale du projet »), puis fin du premier usage selon `docs/plan-fin-premier-usage.md`
(section « Run et statuts »), puis suivi de la maquette selon `docs/plan-suivi-maquette.md`
(section « Maquette et état des écrans »), puis qualité visuelle de l'interface selon
`docs/plan-qualite-interface.md` (échelles du design system appliquées au code et à la maquette).
Un nouveau lot commence par un plan rédigé à partir de `docs/modele-plan.md`.

**Travail en cours** : finalisation du MVP selon `docs/plan-finalisation-mvp.md` (lots A à L en
trois vagues, règles communes R1 à R10). Un lot = une branche `claude/mvp-<lot>` et une pull request ;
il respecte les règles communes, notamment les ajouts en fin de liste dans les fichiers partagés (R5)
et la comparaison avec la maquette par l'aperçu (R6).

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
  lib/steps.ts          Libellés des natures d'étape (familles, moteurs, règles), pour l'affichage
  lib/status.ts         Comptage d'affichage des statuts (✓ / ✕, « réussies / exécutées »)
  stores/               catalog (instantané), navigation (onglet, vue, titre), toasts (notifications),
                        windows (fenêtres, zones, onglets de contenu), workbench (par projet :
                        arborescence, environnements, run, statuts ; par onglet : brouillon,
                        réponse),
                        contracts (contrats par projet, contrats ouverts), steps (catalogue des
                        étapes, reconnaissance, essai)
  composables/          Actions partagées : projets (useProjectActions), onglets (useTabActions)
  components/ui/        Composants de base : bouton, dialogue, menu, interrupteur, notification,
                        en-tête et icônes des sections (SectionHeading, SectionIcon)
  components/layout/    Coquille : sélecteur de workspace (WorkspaceSwitcher), barre latérale (un
                        SidebarProject par projet), zone principale
  components/zones/     Zones : arbre, onglets, séparations (SplitHandle), onglet de requête,
                        onglet de résultat (ResultTab, ResultSide)
  components/windows/   Fenêtres détachées : contenu, barre latérale
  components/contracts/ Contrats : section de la barre latérale, dialogue d'ajout, onglet
  components/projects/  Vues des projets (premier lancement...)
  components/home/      « Essayer une étape » (TryStep) et formulaire déclaratif d'une nature
  components/steps/     Catalogue des étapes, pastille de nature (couleur de sa famille)
  components/workspaces/ Vues des workspaces
  components/workbench/ Arborescence, éditeur, réponse, environnements, run, statuts (StatusMark,
                        StatusCount)
  types/ipc.ts          Types IPC documentés à la main
tests/                  Tests Vitest (happy-dom, mockIPC, faux cœur fakeCore.ts), dont contrastes
                        ; apercu/ : l'application sur le faux cœur dans un navigateur
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
  src/workbench.rs      Établi : envoi, variables, environnements actifs, runs (numéro, groupes,
                        révision), statuts, notificateur des événements
  src/results.rs        Derniers résultats des envois, partagés entre fenêtres (pur)
  src/statuses.rs       Statut de la dernière exécution de chaque requête (statuses.json)
  src/layout/           Zones, onglets et fenêtres (pur : ni Tauri, ni disque)
  src/contract/         Contrats OpenAPI : lecture, exemples, requêtes créées, conformité ;
                        file.rs range les contrats dans contracts/ du projet ; deduce.rs déduit
                        un contrat des essais
  src/steps/            Catalogue des étapes (pur) : définitions (REST intégrée dans steps/rest.json),
                        validation, reconnaissance, valeurs du formulaire, requête HTTP d'un essai
  src/trial.rs          Essai sans projet : contrat déduit et « Essai N » dans « Non classés »
  src/windows.rs        windows.json, fenêtres natives, focus, fermetures, événements arnod://
  src/opencollection/   Format OpenCollection 1.0.0 : schéma embarqué, YAML, requêtes, racine
                        (opencollection.yml, extensions.arnod), conversion de l'ancien format
  src/manifest.rs       Dossier d'un projet et sa racine opencollection.yml
  src/state.rs          AppState : catalogue + stockage + annulation (app.manage)
  src/menu.rs           Menu natif (Fichier > Quitter, Fenêtre > Dispositions, détacher, rattacher)
  src/commands/         Commandes, un fichier par domaine
docs/                   Plans, modèle de plan, maquettes, captures du README, schéma
                        OpenCollection 1.0.0 (docs/opencollection/), design system
                        (docs/design-system/)
scripts/                Mesure de taille, concordance des versions, tests de fumée (CI), état de
                        la maquette (check-maquette.mjs, maquette-sync.py, maquette-couleurs.py),
                        design system (design-system.mjs), aperçu Playwright (apercu.mjs)
.github/                Modèles d'issue et de pull request, Dependabot
  workflows/            CI de build Windows et Linux, contrôle des titres de pull request
```

## Commandes

```sh
npm ci                      # dépendances frontend (Node 22.23.4+, 24.22.1+ ou 26.12.1+)
npm run tauri dev           # application en développement (Vite sur le port 1420)
npm run tauri build         # build release : NSIS sous Windows, AppImage sous Linux

npm run check               # avant chaque commit : tout ce que vérifie la CI hors build
npm test                    # Vitest
npm run build:e2e           # binaire debug pour les tests e2e (Linux)
xvfb-run -a npm run test:e2e # e2e sur l'application réelle (voir e2e/README.md)
node scripts/apercu.mjs requete --maquette 5.1-b  # aperçu Playwright sur le faux cœur, comparé à la maquette
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
  `ColorDot`, `SectionHeading` et `SectionIcon` pour les sections de la barre latérale) ;
  navigation par le store `navigation` (pas de routeur) ; notifications par le store `toasts`.
  Chaque composant interactif est testé au clavier.
- Échelles (`docs/plan-qualite-interface.md`, R1 à R8) : tailles de texte, rayons et hauteurs de
  contrôle viennent des variables `--text-*`, `--radius-*` et `--control-*` de `src/styles.css`,
  jamais d'une valeur écrite dans un composant (sauf les glyphes ▾ ● ×). Texte à 13 px par
  défaut, graisses 400 et 600, un seul bouton plein par vue (`BaseButton` `primary`). Une liste
  déroulante ou une case à cocher garde l'habillage de `styles.css` ; un composant qui change le
  fond d'une liste écrit `background-color`, pas `background`.
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
  les composants. Tout nouveau jeton est défini pour les deux thèmes, en `#rrggbb`, et reçoit sa
  note d'usage dans `scripts/design-system.mjs` ; `tests/theme.spec.ts` vérifie les contrastes.
- Police système uniquement : la CSP interdit les polices distantes des maquettes.
- Hors périmètre actuel : SOAP (et donc WSDL), mocks, scripts, CLI, signature de code, updater. Les
  requêtes HTTP sont livrées (premier usage), les contrats OpenAPI aussi ; les secrets vont dans le
  coffre du système, jamais dans le projet.
- Une tâche de fond lancée depuis une commande passe par `tauri::async_runtime::spawn` : une
  commande synchrone ne s'exécute pas dans un contexte tokio (`tokio::spawn` y panique).
- Une annulation (envoi, run) passe par `http::Cancel`, qui la mémorise : un `Notify` seul ne
  réveille que les envois déjà en attente et laisse partir celui qui commence juste après.
- Le cœur ne garde en mémoire que les `MAX_FINISHED_RUNS` derniers runs terminés ; relancer les
  échecs d'un run oublié repart du dernier run du projet, gardé sur disque.
- L'avancement d'un run arrive par `arnod://run` (révision croissante), jamais par interrogation :
  le store n'applique pas une révision plus ancienne que celle qu'il montre.
- Côté interface, l'état « modifié » d'une requête se lit par `workbench.isDirty` (brouillon
  sérialisé une fois par modification) : ne pas sérialiser un brouillon dans un composant.

## Run et statuts

Référence : `docs/plan-fin-premier-usage.md` (cas RU1 à RU6, ST1 à ST4, VT1 à VT4, règles R1 à
R8), maquettes `docs/maquettes/1e-requete-assertions-reponse.png` et `1f-run.png`.

- Le cœur calcule tout le run : numéro par projet, concurrence, fin, révision et résultat par
  dossier (`run_groups`, premier segment sous le dossier exécuté). `RunPanel.vue` ne fait que
  filtrer (Échecs, Tous, Lents, dossier choisi) et compter.
- `Workbench` prévient un `Notifier` (trait, branché par `lib.rs` sur `windows::EventNotifier`,
  remplacé par un enregistreur dans les tests) de chaque changement de run et de statuts.
- Statut d'une requête : dernière exécution, envoi seul ou ligne de run, réussie ou en échec
  (erreur comprise, annulation ignorée), gardé dans `projects/<id>/statuses.json` (poste, jamais le
  projet) et diffusé par `arnod://statuts`. Il suit renommages, déplacements et suppressions.
  Barre latérale : `StatusMark` (✓ / ✕, jamais la couleur seule) et `StatusCount`
  (« réussies / exécutées », sinon le nombre de requêtes).
- Une vérification ignorée (`AssertionResult.ignored`) ne vaut que pour le résultat affiché
  (`result_ignore_check`, `run_ignore_check`) ; elle reste dans la requête. « Retirer la
  vérification » la retire du brouillon (enregistrement automatique). La conformité au contrat
  s'ignore mais ne se retire pas.
- Origine d'une vérification : de session, jamais écrite ; relue d'un fichier, elle est
  « manuelle », sauf la durée par défaut, reconnue « par défaut ».

## Maquette et état des écrans

Référence : `docs/plan-suivi-maquette.md` (règles R1 à R9).

- La maquette (Claude Design) range ses pages par code (0 Cadre, 1 Espace de travail, 2 Démarrer,
  3 Services, 4 Tests, 5 Exécution, 6 Recette, 7 Équipe) ; un écran a un repère `3.2-a2` (page,
  lettre). Plans, commits et pull requests citent les écrans par repère.
- `docs/maquette-etat.yml` tient l'état de chaque écran (livré, en partie, à faire…), sa pull
  request ou son plan, ses captures e2e et ses écarts avec le code (`aligner` : maquette, code ou
  à décider). Une pull request qui livre, modifie ou écarte un écran le met à jour ;
  `scripts/check-maquette.mjs` le vérifie (`npm run check`, CI).
- Une capture citée existe dans `e2e/arnod.e2e.ts` (`screenshot(browser, "…")`) : un écran livré
  visible gagne une capture e2e. La maquette est régénérée depuis le fichier après fusion
  (`scripts/maquette-sync.py`, toutes les pages ou celles dont on donne le code).
- Design system « Arnod » (`docs/design-system/`, publié sur claude.ai) : référence des couleurs,
  typographie, espacements, hauteurs, rayons et composants pour la maquette et le code. Les
  couleurs et les échelles (texte, rayons, hauteurs) sont lues dans `src/styles.css`.
  `scripts/maquette-couleurs.py` aligne le bouton principal et les teintes proches des écrans.
  `scripts/design-system.mjs` en génère les jetons, les styles des composants et les icônes
  depuis le code ; `npm run check` échoue s'il n'est pas à jour. README, guides et aperçus des
  composants s'écrivent à la main. La page 0.5 de la maquette est générée depuis ce dossier.

## Projets et workspaces

Référence : `docs/plan-projets-workspaces.md` (cas d'usage P1 à P12 et W1 à W11, règles R1 à
R11), révisé par `docs/plan-workspaces-onglets.md` (cas O1 à O12, règles R3, R4, R7, R8, R11 et
R12) puis par `docs/plan-selecteur-workspace.md` (cas S1 à S9, I1, I2). Cocher une tâche dans le
plan quand elle est livrée.

- « Tous les projets » est le référentiel, proposé en tête du sélecteur de workspace ; ce n'est pas
  un workspace. Il n'y a plus de workspace par défaut : un projet est dans zéro, un ou plusieurs
  workspaces. Le sélecteur suit l'ordre de création ; `lastWorkspaceId` vaut `null` pour
  « Tous les projets ».
- Sélecteur (`WorkspaceSwitcher.vue`) : en tête de la colonne de la barre latérale, à la hauteur de
  `.zone__bar` ; sans barre latérale (gestion, catalogue, barre détachée), dans une rangée en haut.
  « Tous les projets » garde la barre latérale (S9) : on en sort en ouvrant un projet ou par le
  sélecteur ; hors workspace, sa ligne d'ajout est « + Ouvrir un projet » (dossier existant). Plus
  de rangée d'onglets de workspaces. Son menu (`role="menu"`, `menuitemradio`) porte
  la création, les actions d'un workspace (« ⋯ », clic droit, Maj+F10 : renommer F2, couleur,
  gérer, supprimer Suppr) et le dépôt d'un projet glissé ; Ctrl+0…9 et Ctrl+Maj+W partout (code de
  la touche, pour l'AZERTY). Tests : aides `tests/switcher.ts`.
- Sections d'un projet dans la barre latérale : `SectionHeading` (icône `SectionIcon`, nom, nombre,
  « + »). Les icônes des sections à venir sont déjà dans `components/ui/sectionIcons.ts`.
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
- Résultat détachable (`docs/plan-resultat-detachable.md`, D1 à D7, R1 à R7) : le contenu `result`
  (un seul dans l'application) montre le résultat de la **requête active**, calculée par le cœur
  (`Windows::active_request` : ordre du focus, zones) et diffusée par `arnod://requete-active`. Sa
  fenêtre ne reçoit pas la sélection ; tant qu'il existe, l'onglet d'une requête ne montre que
  l'éditeur ; le rattacher le ferme. Le cœur garde le dernier envoi de chaque requête
  (`results.rs`, 50 au plus) et le diffuse par `arnod://resultat` ; un run démarré est diffusé par
  `arnod://run`. Une fenêtre ignore l'écho de ses propres événements (`origin`).

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
- Barre latérale (`docs/plan-requetes-dans-contrats.md`, RC1 à RC9, R1 à R7) : une requête liée est
  montrée **sous son contrat** (ressource = premier segment du chemin, puis opération, puis
  requêtes ; la première est l'endpoint), jamais dans le référentiel de tests ; « Hors contrat »
  pour une opération absente ou un contrat illisible. Le cœur calcule tout : `ContractSummary`
  (`resources`, `unplaced`) et `TreeView.tests` (`tree::tests_view`). Sur disque, rien ne change
  (`<contrat>/<tag>/`). Côté interface, relire l'arborescence relit les contrats du projet (et
  inversement, `contracts.sync`) ; l'état déplié est en mémoire (store `contracts`).

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
  d'environnement (`EnvironmentChip`) ; puis `ContractsSection` (« Contrats de service », avec les
  requêtes liées) et `TreePanel` (« Référentiel de tests » : les requêtes sans contrat), dont les
  créations passent par leurs méthodes exposées. Plus de section « Requêtes ». Un projet replié
  montre sa pastille compacte à droite.
- Toute ligne de la barre latérale suit la grille `--tree-pad`, `--tree-indent`, `--tree-lead`,
  `--tree-gap` (définie par `AppSidebar`) : une colonne de tête de largeur fixe, le libellé
  après ; une nouvelle ligne la reprend au lieu de marges propres.
- Menus par `BaseMenu` (entrées `radio`, `swatch`, `tag`, `detail` pour les environnements) ; pas
  de texte d'aide vide : une ligne d'ajout (« + Nouvelle requête », « + Ajouter un contrat »).
- Raccourcis : Ctrl+N (nouvelle requête) et Ctrl+Maj+E (environnements) sur le projet de l'onglet
  actif, sinon le projet ouvert, sinon le premier déplié.

## Accueil, catalogue des étapes et Non classés

Référence : `docs/plan-accueil-non-classes.md` (cas A1 à A5, K1, K2, N1 à N5, règles R1 à R12) et
l'étude `docs/etude-catalogue-etapes.md` (catalogue synchronisé, non implémenté).

- Une nature d'étape est une **définition** (données : famille, moteur, champ principal, règles de
  reconnaissance, formulaire déclaratif) exécutée par un **moteur** du cœur. Seule REST existe
  (`src-tauri/steps/rest.json`, moteur intégré `http`, version d'interface 1). Une définition est
  validée (`steps::definition::validate`) : format, moteur connu, champs uniques, `visibleWhen`
  vers un choix placé avant. Pour une nouvelle nature : une définition, un moteur, les familles et
  natures de champ restent fermées (miroirs `StepFamily`, `StepFieldKind` testés).
- La source du catalogue (`arnod-steps`, `synced: false`) prépare la synchronisation décrite dans
  l'étude (index signé, cache local, épinglage par projet) : ne pas l'implémenter sans un plan.
- Accueil (`FirstRun.vue`) et « Tous les projets » : une entrée « Essayer une étape » (`TryStep`) ; le
  cœur reconnaît la nature (`steps_detect`, une méthode en tête passe dans le formulaire) ; la
  nature choisie ouvre son formulaire sous l'entrée et réduit créer / importer à une ligne. Le
  formulaire est généré depuis la définition (`NatureForm.vue`), jamais écrit à la main par nature.
- « Non classés » est un vrai projet par poste (`inboxProjectId` dans `arnod.json`), créé au
  premier essai dans `<emplacement par défaut>/Non classés`, premier de la liste, jamais dans un
  workspace (refusé par le cœur), avec son icône de bac. `trial_send` envoie dans l'environnement
  « Local », crée ou enrichit le contrat déduit de l'origine (`contracts/<hôte>.yaml`, OpenAPI 3.1
  marqué `x-arnod-deduced`, version masquée dans l'interface) et range « Essai N » dans le dossier
  du serveur, liée à son opération, vérifications proposées et exemple compris. Un contrat écrit
  à la main n'est jamais modifié ; un contrat déduit illisible refuse l'essai.
- Couleurs des familles : jetons `--color-family-*` (texte, fond, bord) des deux thèmes, contrastes
  vérifiés dans `tests/theme.spec.ts`.

## Saisie, texte et cURL

Référence : `docs/plan-mvp-b-saisie-curl.md` (cas SA1 à SA6, TX1 à TX4, CU1 à CU4, règles R1 à
R13), écrans `5.1-b ter`, `5.1-b quater`, `2.1-a sexies`.

- Le cœur décide : `input/` (pur) calcule variables connues et inconnues, en-têtes ajoutés à l'envoi
  et listes en texte ; `curl/` (pur) lit une commande cURL (syntaxe bash) et écrit celle d'une
  requête. L'interface ne fait que souligner (`lib/variables.ts`, directive `v-variables`, calque
  transparent sur le champ) et compter.
- `request_input` est appelée 250 ms après une frappe : elle lit environnements et `local.json`,
  jamais le coffre. `request_curl` lit le coffre pour remplacer chaque secret par une variable du
  shell (`$TOKEN`, `$API_KEY`, `-u "user:$PASSWORD"`) : aucun secret dans une commande copiée.
- Une valeur dynamique `{{$…}}` n'est jamais inconnue. Les variables de dossier et de collection
  sont connues ; leur remplacement à l'envoi relève de `request/resolve.rs`.
- Les variables connues passent de `RequestEditor` aux éditeurs par `provide(KNOWN_VARIABLES)` ; un
  nouveau champ de requête qui accepte `{{…}}` lit `useKnownVariables()` et prend `v-variables`.
- `PairsEditor` avec `text-format` (`header`, `query`) offre « Éditer en texte » : lecture et écriture
  par `pairs_from_text` / `pairs_to_text`, descriptions gardées par nom, une ligne illisible bloque
  le retour au tableau.
- Une commande cURL collée est reconnue par la règle `curl` de la définition REST (champs `method`,
  `headers`, `bodyType`, `body`) : le jeton sort du texte (`{{token}}`), reste en mémoire dans
  l'interface et ne va au coffre (variable locale secrète de « Local » de « Non classés ») qu'à
  l'envoi de l'essai (`trial_send`, `secrets`), réponse masquée.

## Lot D - Barre latérale

Référence : `docs/plan-mvp-d-barre-laterale.md` (cas F1 à F5, D1, D2, M1, M2, E1 à E3, W1, règles
R1 à R6), écrans 1.2-e, 1.2-d, 1.2-a ter, 3.2-a3 et 1.1-a.

- Filtre du référentiel (`TreeFilter.vue`, `lib/treeFilter.ts`, pur et testé) : filtre d'affichage
  fait dans l'interface (Q4 du plan d'ensemble), sur `TreeNode` (`url`, `tags` fournis par le cœur).
  Termes cumulés : `#mot` (début d'étiquette), méthode en capitales, sinon nom ou adresse sans casse
  ni accents. Pendant un filtre, les dossiers sont dépliés sans toucher à l'état replié enregistré.
  Ctrl+F ne vaut que quand la barre latérale a le focus.
- État d'affichage propre à la fenêtre dans le store `sidebar` : texte du filtre par projet,
  sections dépliées, réponse attendue à ouvrir (`openExample`, servie par `ResultSide` et
  `ExpectedResponses`), nombre de contrats des cartes (`contract_count`, sans lire les contrats).
- `SectionHeading` avec `expanded` : section repliable ; une section vide est repliée par défaut,
  dépliée elle montre sa ligne d'ajout ; le nombre n'est montré que replié.
- Sous un contrat, la ligne d'une opération **est** son endpoint (première requête liée) : clic,
  statut, nombre de réponses attendues (`LinkedView.examples`) et menu ⋯ ; dépliée, ses réponses
  attendues par code, puis les autres requêtes de l'opération.
- Projet vide (ni requête ni contrat) : `ProjectWelcome` montre quatre points de départ ; l'ajout
  d'un contrat passe par `useContractAdd`, partagé avec `ContractsSection`.
- Menu « + » du projet : Nouvelle requête (Ctrl+N), Nouveau dossier (Ctrl+Maj+N), Ajouter un
  contrat ▸, Nouvel environnement… ; une création déplie la section concernée.

## Corps de la réponse

Référence : `docs/plan-mvp-a-reponse.md` (lot A, cas F1 à F8, règles R1 à R10), écran `5.1-b bis`.

- Le cœur met en forme et filtre (`src-tauri/src/response/`, pur) : format détecté d'après
  `content-type` (`response_render`), aperçu HTML en arbre de balises sûres sans attribut
  (`response_preview`, ni `v-html` ni cadre), filtre JSONPath RFC 9535 (`response_filter`, crate
  `serde_json_path`) dont une expression invalide revient en français dans `FilterResult.error`.
- Chaque valeur JSON mise en forme et chaque correspondance portent le même chemin canonique
  (`$.items[0].sku`) : l'interface surligne en comparant des chemins, jamais en relisant le JSON.
- Une correspondance porte la vérification qu'elle donne (`FilterMatch.assertion`, cible
  `body.…` sans joker) ; le store `response` l'ajoute au brouillon, qui s'enregistre seul.
- Enregistrer passe par `response_save` (sélecteur natif appelé depuis Rust) ; effacer par
  `response_clear` (`ResultBook::clear`, diffusé par `arnod://resultat`). Un corps tronqué à 5 Mo
  est signalé à la copie et à l'enregistrement.
