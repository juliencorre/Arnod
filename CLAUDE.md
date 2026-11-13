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

## Import Bruno

Référence : `docs/plan-mvp-c-import-bruno.md` (cas IM1 à IM6, règles R1 à R11), écrans `2.3-a`,
`2.3-b` et `2.3-f`.

- Un seul « Importer » : le cœur reconnaît le format (`import::detect`, sans écriture) : collection
  Bruno en `.bru` (dossier avec `bruno.json`), projet Arnod ou OpenCollection (ajouté tel quel),
  contrat OpenAPI (rangé dans `contracts/`). L'interface ne demande jamais d'où vient l'utilisateur.
- Les fichiers d'origine ne sont jamais écrits : une collection Bruno devient un nouveau projet
  (`AppState::create_project_with`, dossier retiré en cas d'échec) ou un nouveau dossier, au nom
  libre, d'un projet (réglages de la collection dans son `folder.yml`, environnements complétés).
- `src-tauri/src/bruno/` : `parse` (analyseur `.bru` écrit à la main, blocs inconnus gardés bruts),
  `read` (disque, `.env`), `convert` (pur : fichiers OpenCollection validés contre le schéma et
  rapport) ; tests sur la collection d'exemple `bruno/fixtures/boutique/`. Un bloc ou une option
  sans équivalent est signalé, jamais inventé.
- Correspondances fixes : `vars:post-response` → action `set-variable` `after-response` (sélecteur
  `jsonq`, expression de Bruno telle quelle) ; scripts et `tests` → `runtime.scripts`, jamais
  exécutés ; `{{$…}}` gardé ; `vars:secret` et `{{process.env.X}}` → variables locales sans valeur,
  une valeur lue dans `.env` va au coffre.
- Rapport : `ImportReport` gardé sur le poste (`projects/<id>/import.json`), montré dans l'onglet
  `importReport` (`ImportReportTab`) ; chaque élément signalé cite son fichier et son bloc d'origine.

## Tests, étapes et variables

Référence : `docs/plan-mvp-e-tests-etapes.md` (lot E : cas TE1 à TE5, EX1 à EX5, VD1 à VD4, NT1, NT2,
MV1, MV2 ; règles R1 à R12), maquettes 4.2-a, 4.2-b bis, 5.1-h, 4.1-b.

- Un **test** est un dossier qui contient au moins une requête et aucun sous-dossier
  (`TreeNode::Folder.test`, calculé par `tree::read`) ; ses étapes suivent `info.seq` puis le nom.
  « Nouveau test » (`tree_create_test`) et « + Étape » (`tree_add_step`) posent `seq`. Une requête
  liée à un contrat n'est jamais une étape, et un run de contrat n'a pas de test.
- Run (`workbench.rs`) : unités `run_units` (étapes consécutives d'un test ensemble, requête hors test
  seule), concurrentes entre elles ; dans un test, une étape à la fois, arrêt à la première en échec
  ou en erreur, les suivantes `skipped` (« Non jouée », statut inchangé). `RunView.tests`
  (`run_tests`) donne le verdict de chaque test ; « Relancer les échecs » rejoue un test entier.
- **Extraction** = action OpenCollection `set-variable` hors phase `before-request`
  (`Request.extractions`, source `body.id`, `headers.Nom`, `status`, `request.body.…` écrite
  `req.…`) ; les autres actions restent conservées (`preserved`). Valeurs extraites : variables
  d'exécution, passées aux étapes suivantes du test, puis gardées par projet pour la session
  (`Workbench::runtime_variables`), jamais écrites dans un environnement ; elles priment sur lui.
- **Valeurs dynamiques** `{{$nom}}` (noms de Bruno, `request/dynamic.rs`, sans crate ajoutée) :
  tirées par `resolve.rs` à chaque occurrence, jamais inconnues (un nom inconnu part tel quel).
  `SendOutcome.sent` porte ce qui est parti (masqué), les valeurs tirées (avec leur source si elles
  forment seules une valeur envoyée), les variables lues et les valeurs extraites (onglet « Envoyé »,
  `SentPanel.vue`) ; « Garder » ajoute l'extraction de la valeur envoyée.
- Le cœur garde la requête envoyée avec le résultat (`RequestResult.request`) ; le store compare le
  brouillon (JSON aux clés triées) pour « modifié, pas encore envoyé » et « Revenir à la version
  envoyée » (`isChangedSinceSent`, `revertToSent`).
- Onglets : contenu `test` (`TestTab.vue`), barre des étapes au-dessus d'une étape (`StepBar.vue`),
  onglet « Variables » de l'éditeur (`VariablesEditor.vue`, « Utilisée par » :
  `request_variable_users`).
- Les variables extraites par les étapes précédentes du test (`tree::extracted_before`) et les
  variables d'exécution de la session sont connues de `request_input` : jamais soulignées
  inconnues. Une valeur `$…` fournie par l'appelant (copie cURL) n'est pas tirée.
- « Extraire dans une variable » du filtre JSONPath (`ResponseFilter.vue`, store `response`)
  ajoute l'extraction de la correspondance courante (`workbench.addExtraction`).

## Essais : chemins généralisés et transfert

Référence : écrans `2.2-c` et `2.2-b`.

- `trial/generalize.rs` (pur pour `template`, disque pour le reste) : un segment qui ressemble à
  un identifiant (nombre, UUID, mot de 6 caractères ou plus mêlant lettres et chiffres) donne un
  chemin généralisé (`/orders/{id}/tracking`, dernier paramètre `id`, les autres `<ressource>Id`).
  `trial_generalization` le propose, `trial_generalize` déplace l'opération du contrat déduit
  (fusionnée si le chemin existe) et fait suivre les requêtes liées, `trial_keep_path` l'écarte
  (`x-arnod-path-kept`). Un essai suivant au même chemin rejoint l'opération généralisée
  (`deduce::add_exchange`).
- `trial::transfer` (`trial_transfer`) : l'essai devient l'étape 1 d'un nouveau test du
  référentiel, sans lien au contrat. Un dossier qui ne range que des requêtes liées n'est pas un
  test (`tree::is_test`).
- Interface : `TrialGeneralize.vue` sous l'adresse de toute requête liée à un contrat déduit,
  `TrialTransfer.vue` (« Requête au point ? ») sous l'éditeur d'un essai de « Non classés ».

## Étiquettes proposées à la saisie

Référence : écran `4.9-a`.

- `TagsEditor.vue` (pastilles et champ d'ajout de `RequestEditor`) propose les étiquettes du projet
  avec leur nombre de requêtes, puis « + Créer « … » » : comptage d'affichage dans `lib/tags.ts`
  (pur, testé) sur les `TreeNode.tags` fournis par le cœur, sans casse ni accents.
- Motif combobox : ↑ ↓, Entrée ajoute la proposition choisie (sinon le texte), Échap annule.

## Valeurs dynamiques à la saisie

Référence : écran `5.1-h` (liste ouverte à la frappe de `{{$`).

- Le cœur fournit noms, descriptions et un exemple tiré (`dynamic_values`, `request::dynamic::catalog`,
  ordre de `NAMES`) ; le store `input` les lit une fois (`loadDynamicValues`).
- `DynamicValuesMenu.vue` (monté par `RequestEditor`) écoute les champs marqués par `v-variables`
  (`data-variables`) : `lib/dynamicValues.ts` (pur, testé) repère la saisie au curseur, filtre et
  insère `{{$nom}}`. Motif combobox : le champ garde le focus, ↑ ↓, Entrée ou Tab, Échap.

## Lot H - Réseau et authentification

Référence : `docs/plan-mvp-h-reseau-auth.md` (cas PR1 à PR6, EC1 à EC6, OA1 à OA5, CK1 à CK4,
règles R1 à R15), écrans 1.6-e, 5.1-i, 3.4-a, 3.4-g.

- Tout envoi (requête, run, essai, jeton) passe par `network::Network` (tenu par
  `Workbench::network`) : client reqwest construit une fois pour les préférences du poste et
  reconstruit quand elles changent (`network.json` dans le dossier de données, écriture atomique,
  mot de passe du proxy au coffre). `http::send` sans préférences ne sert qu'aux tests.
- Proxy : du système (variables `HTTPS_PROXY`/`HTTP_PROXY`/`NO_PROXY`, puis registre Windows lu par
  `windows-registry` derrière le trait `SystemProxySource`), aucun ou manuel ; exceptions `*`,
  `<local>`, `.domaine`.
- TLS : autorités du système (`rustls-native-certs`, repli sur les racines Mozilla) plus les
  autorités internes ; le vérificateur délègue à webpki et garde le certificat refusé pour
  l'expliquer. **La vérification n'est jamais coupée** : aucune option, aucun bouton ; une autorité
  s'ajoute après avoir montré sujet, validité et empreinte (`CertificateCard`).
- Un échec réseau est classé par `network::failure::classify` et voyage dans
  `CommandError.failure` (absent du JSON sinon) et `RequestResult.failure` ; `IpcError.failure` le
  rend à l'interface, `NetworkFailurePanel` l'affiche. Une nouvelle cause s'ajoute à `FailureCause`
  (Rust et `src/types/ipc.ts`) avec un test sur un serveur local (`network/tests.rs`, certificats
  construits par `network/testcert.rs`, sans clé versionnée).
- OAuth 2 : seul le flux `client_credentials` du schéma est géré (`AuthKind::OAuth2`, `type: oauth2`
  écrit tel quel, champs non gérés conservés) ; jeton en mémoire par projet, environnement, URL,
  client et portée, redemandé 60 s avant expiration et masqué partout (`Prepared.hidden`). Le
  secret client du projet va au coffre (variable locale secrète `oauth2ClientSecret`), jamais en
  clair dans `opencollection.yml`.
- Cookies : magasin par projet et environnement (`network::cookies`), persistants dans
  `projects/<id>/cookies.json` (poste), de session en mémoire ; jamais ajoutés si la requête a son
  propre en-tête `Cookie` ; valeurs `HttpOnly` masquées.
- Préférences : `PreferencesDialog` (Ctrl+, et `arnod://preferences` du menu Fichier, adressé à
  une fenêtre) ; une section de plus s'ajoute à `SECTIONS` et au type `PreferencesSection` du
  store `network`.

## Onglets et clavier

Référence : `docs/plan-mvp-f-onglets-clavier.md` (lot F : cas AP1 à AP4, TO1, TO2, CT1, CT2, CL1 à
CL3 ; règles R1 à R8), écrans 1.3-a, 1.3-b, 1.3-c et 1.4-j.

- Le cœur tient l'aperçu et l'ordre d'utilisation (`layout/`) : `Tab.preview` (au plus un par zone,
  remplacé sur place par le prochain aperçu, `Layout::open_tab`), `Zone.recent` (le plus récent
  d'abord, toujours complet, réparé à la lecture de `windows.json`). Un onglet déplacé, divisé,
  détaché, rattaché, ouvert sans aperçu ou modifié devient permanent (`layout_pin`).
- Seul un simple clic dans la barre latérale ouvre en aperçu (`openSelection(content, true)`) ;
  le double-clic ouvre sans aperçu. Toute ouverture par une action (créer, essayer, ouvrir la
  requête d'origine) reste permanente : n'y passez pas `preview`.
- Affichage seulement dans l'interface (`lib/tabs.ts`, pur) : onglets qui tiennent dans la barre
  (`fitTabs`, les autres sous « +N »), ◐ d'une exécution (`isRunning`), ordre de Ctrl+Tab
  (`byUse`). Fermer plusieurs onglets passe par `useTabActions().closeMany` (enregistre d'abord).
- Raccourcis par le code de la touche : Ctrl+Tab / Ctrl+Maj+Tab et Ctrl+J dans `ZoneArea`, Ctrl+\
  dans `AppShell`. Le menu natif « Fenêtre » porte leurs entrées sans accélérateur (il les ferait
  deux fois) et émet `arnod://raccourci` (`MenuShortcut`) vers la fenêtre visée.
- Place du résultat : l'icône de l'en-tête du résultat et Ctrl+J passent de l'autre côté, ou
  reviennent à l'automatique (`nextPlacement`) ; barre latérale masquée :
  `WindowState.sidebarHidden` de la fenêtre principale, gardé dans `windows.json`.

## Fichiers modifiés hors d'Arnod

Référence : `docs/plan-mvp-i-fichiers-externes.md` (lot I : cas FE1 à FE7, règles R1 à R12), écran
`4.1-e`.

- `src-tauri/src/watch/` : un observateur `notify` par projet ouvert (`Watchers`, démarré par
  `tree_load`, arrêté quand le projet est oublié ou introuvable), un fil qui regroupe les événements
  300 ms, un registre de l'état connu (empreinte de chaque YAML / JSON, dossiers). `.git/`, éléments
  cachés, `*.tmp` et `*~` sont ignorés.
- Toute écriture d'Arnod dans un projet passe par `opencollection::yaml::write` ou
  `contract::file::write_text` (qui appellent `watch::record_write`) ; une suppression ou un
  renommage appelle `watch::settle`. Une nouvelle écriture directe (`fs::write`, `fs::rename`) dans
  un projet doit faire de même, sinon son écho est pris pour une modification externe.
- Un lot est diffusé par `arnod://fichiers` (`FilesChange` : renommés, supprimés…), puis par
  `arnod://projet` (origine vide) pour chaque portée et chaque requête modifiée. Un renommage est
  reconnu à l'empreinte (un supprimé, un ajouté identique).
- `request_read` retient la version lue (`watch::acknowledge`) ; `request_save` refuse
  (`changedOnDisk`) d'écrire sur une version plus récente. Le store passe alors l'onglet en conflit
  (`Editor.conflict`, `ConflictPanel.vue`) : l'enregistrement automatique est suspendu, le cœur
  compare (`request_conflict`, `watch::conflict::compare`, pur) et fusionne
  (`request_conflict_resolve`). L'ancien bandeau « Modifiée dans une autre fenêtre » n'existe plus.
- Un fichier de requête illisible n'est jamais réécrit (`request::file::write` le refuse).

## Démarrage et erreurs

Référence : `docs/plan-mvp-g-demarrage-erreurs.md` (lot G : cas DL1, ER1 à ER3, VR1, SU1, SU2 ;
règles R1 à R8), écrans 0.3-a, 0.3-b, 0.3-b bis, 0.3-b ter.

- Un fichier d'état (`arnod.json`, `windows.json`) illisible, plus récent, verrouillé ou un dossier
  de données inaccessible donne un `startup::Failure` : rien n'est lu ni écrit (pas même
  `windows.json` à la sortie) tant que l'utilisateur n'a pas choisi. Le cœur décrit l'écran
  (`startup::describe`, pur : cause, ce qui est intact, deux issues dont la recommandée en premier,
  détails, adresse de l'issue) ; `StartupProblemScreen.vue` affiche tel quel.
- « Repartir d'un état vide » renomme le fichier (`.format-N`, `.illisible`, puis `-2`…), jamais ne
  le supprime ; un verrou est réessayé seul (2, 4, 8 s, `startup_retry` automatique) puis à la main.
- Navigateur et explorateur : programme du système par chemin absolu (`startup::open_external`),
  sans plugin ni permission ; à défaut, l'adresse de l'issue est copiée. Rien n'est envoyé seul.
- `arnod.json` porte `writtenBy` (version d'Arnod qui l'a écrit), cité par une version plus ancienne.
- Fenêtres créées cachées (`visible: false`, `.visible(false)` des détachées) : `App.vue` appelle
  `window_ready` après le premier rendu ; `windows::show_later` les montre après 300 ms sinon, avec
  le squelette (`StartupSkeleton`), jamais un écran « Chargement… ».
