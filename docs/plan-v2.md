# Plan - Version 2 : travailler en équipe, jouer les tests hors de l'application

**Version** 0.1 · **Date** 2026-11-27 · **Statut** En cours (lots A, B et C livrés ; questions du § 11 à trancher par lot) ·
**Base** version 0.0.4 (Develop @46f5d44)

Plan d'implémentation de la **V2** de la maquette : « Les projets sont déjà des fichiers dans Git :
on les partage, et on les joue hors de l'application. » Il part de la version 0.0.4 (MVP et V1
livrés) et respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et le design system
(`docs/design-system/`).

Source : maquette, page 0.1-a (feuille de route, section `feuille_de_route` de
[`maquette-etat.yml`](maquette-etat.yml)) et pages 1.5 « Recherche », 4.9 « Étiquettes », 5.3
« Ligne de commande », 7.1 « Synchronisation ». Les écrans sont cités par leur repère (voir
[`plan-suivi-maquette.md`](plan-suivi-maquette.md)).

## 1. Objectif

À la fin de ce plan, Arnod :

- retrouve tout élément des projets du workspace par Ctrl+K : par nom, par adresse
  (`POST /orders`) ou par une URL complète collée, résultats groupés, le premier ouvert par
  Entrée ;
- étiquette aussi un dossier (l'étiquette vaut pour tous ses tests), montre les étiquettes sur
  chaque ligne de la vue d'un dossier et filtre cette vue par étiquette ;
- joue un dossier, un test ou une étiquette hors de l'application par la commande `arnod run`,
  avec un résumé, des rapports JUnit, Allure et HTML et un code de sortie, sous Windows et sous
  Linux, sans installation à part ni Docker ; l'application copie la commande d'un test ou d'un
  dossier ;
- relie un projet à un dépôt GitHub, GitLab ou à une URL Git, récupère et envoie les
  modifications depuis une barre d'état, règle un conflit assertion par assertion, montre ce que
  les collègues ont changé, en langage clair, sans jamais faire voyager un secret.

## 2. Maquettes

| Repère     | Écran                                                                  | Lot | Cas           |
| ---------- | ---------------------------------------------------------------------- | --- | ------------- |
| `1.5-a`    | Recherche par nom : résultats groupés, Entrée ouvre le premier         | A   | RH1 à RH4     |
| `1.5-b`    | Recherche par adresse ou identifiant, URL complète collée              | A   | RH5, RH6      |
| `4.9-b`    | Vue d'un dossier filtrée par étiquette, étiquettes sur chaque ligne    | B   | ET1 à ET4     |
| `4.9-d`    | `arnod run --etiquette smoke`                                          | D   | LC3           |
| `5.3-a`    | `arnod run` d'un dossier : résultats, résumé, rapports, code de sortie | D   | LC1, LC2, LC4 |
| `5.3-c`    | « Copier la commande » du menu ⋯ (test, dossier ; campagne en V7)      | D   | LC6           |
| `7.1-a`    | Relier le projet : GitHub, GitLab, URL Git ; compte, dépôt, branche    | E   | SY1 à SY3     |
| `7.1-b`    | Synchroniser depuis la barre d'état : à récupérer, mes modifications   | E   | SY4 à SY7     |
| `7.1-e`    | Après une récupération : secrets manquants à renseigner                | E   | SY8           |
| `7.1-c`    | Conflit : choisir assertion par assertion, le reste fusionné           | F   | CF1 à CF3     |
| `7.1-d`    | Travail en équipe : pastilles, historique d'un test en clair           | F   | CF4, CF5      |
| transverse | Barre d'état (fonction « Barre d'état » de 0.1-a, sans écran propre)   | E   | SY4           |

Hors V2 dans ces pages : `5.3-b` (étape manuelle dans un terminal, V7), `4.9-c` (campagne par
étiquettes, V7), `5.3-d` (intégration continue, retirée de la V2 : Q4) ; les groupes
« Exigences », « Mocks et anomalies » de la recherche et les entrées « mocks » et « catalogue » de
la barre d'état viennent avec leurs versions (V5 à V7).

## 3. Modèle et règles

**Recherche** : un index des éléments des projets du workspace (ou de « Tous les projets ») :
projets, dossiers, tests, requêtes, opérations des contrats (endpoints), contrats,
environnements. **Étiquette effective** d'une requête : les siennes plus celles de ses dossiers
parents. **Cible** d'`arnod run` : un chemin de dossier, de test ou de requête relatif au projet,
ou une étiquette. **Projet relié** : un dossier de projet qui est un dépôt Git avec une branche
suivie sur un serveur (`origin`). **Élément partagé** : ce qu'un fichier du projet représente pour
l'utilisateur (test, requête, dossier, contrat, environnement, réglages du projet), jamais un
chemin de fichier ni un commit.

Les règles vivent dans le cœur Rust, modules purs testés règle par règle.

| ID  | Règle                                                                                                                                                                                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | La recherche compare sans casse ni accents ; un terme `#mot` ne garde que les éléments dont une étiquette effective commence par `mot` ; une méthode en capitales filtre la méthode.                                          |
| R2  | Une saisie `MÉTHODE /chemin` ou une URL complète se compare aux chemins des opérations (`{id}` vaut un segment) : correspondance exacte d'abord, puis les opérations de la même ressource, puis les requêtes qui l'appellent. |
| R3  | Les résultats sont groupés (Tests, Endpoints, Contrats, Environnements, Projets), au plus 8 par groupe, dans un ordre stable (exact, début de nom, contenu, puis nom).                                                        |
| R4  | Une étiquette posée sur un dossier vaut pour tous les tests et requêtes qu'il contient, à toute profondeur ; elle s'écrit dans `info.tags` de son `folder.yml`.                                                               |
| R5  | `arnod run` n'écrit rien hors du dossier des rapports : ni statuts, ni historique, ni `local.json`, ni coffre, ni fichier du projet.                                                                                          |
| R6  | Une variable locale (et un secret) se lit, dans l'ordre : `ARNOD_VAR_<nom>`, puis le coffre et `local.json` du poste s'ils existent ; une variable sans valeur arrête l'exécution avant tout envoi, en la nommant.            |
| R7  | Code de sortie d'`arnod run` : 0 tout réussi, 1 au moins un test en échec ou en erreur, 2 commande invalide (projet, cible ou environnement introuvable, variable manquante).                                                 |
| R8  | Les rapports décrivent le run tel que le cœur l'a calculé (tests, étapes, assertions, durées) ; un secret n'y figure jamais (valeurs masquées comme dans l'onglet « Envoyé »).                                                |
| R9  | Tout échange avec un serveur Git ou une forge passe par `network::Network` (proxy et autorités du poste) ; un jeton est rangé dans le coffre du système, jamais dans le projet ni dans `.git/config`.                         |
| R10 | Seuls les fichiers du projet voyagent ; ce qui est propre au poste (`local.json`, statuts, historique, cookies, jetons) est hors du dossier du projet et n'est jamais envoyé.                                                 |
| R11 | Arnod récupère toujours avant d'envoyer ; un envoi refusé parce que le serveur a avancé relance la récupération, jamais un envoi forcé.                                                                                       |
| R12 | Une fusion se fait élément par élément (comme le conflit du lot I) : ce qu'un seul côté a changé est pris sans question ; seul un élément changé des deux côtés, différemment, est un conflit.                                |
| R13 | Un conflit non réglé bloque l'envoi ; rien n'est écrit dans le projet tant que la résolution n'est pas validée, et le dépôt reste dans l'état d'avant la récupération si l'utilisateur abandonne.                             |
| R14 | Le journal d'un élément et les pastilles se décrivent en langage clair (auteur, date, ce qui a changé), d'après les versions successives du fichier ; jamais un hash ni un chemin.                                            |
| R15 | Après une récupération, une variable locale déclarée par un environnement et sans valeur sur ce poste est signalée, jamais inventée.                                                                                          |
| R16 | `arnod` est un exécutable autonome : il se lance sans installation à part, sans droits administrateur, sans Docker ni bibliothèque du système à ajouter (Linux : lié statiquement).                                           |

## 4. Cas d'usage

Priorités : **Must** (indispensable au lot), **Should** (attendu), **Could** (si le temps le
permet).

### Lot A - Recherche (1.5)

| ID  | Cas d'usage             | Écrans  | Comportement attendu                                                                                                                            | Priorité |
| --- | ----------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| RH1 | Ouvrir la recherche     | `1.5-a` | Ctrl+K (action « Rechercher » des raccourcis modifiables) ouvre une palette au-dessus de la zone ; portée affichée « Workspace X · N projets ». | Must     |
| RH2 | Chercher par nom        | `1.5-a` | Résultats groupés pendant la frappe (R1, R3), terme surligné ; 150 ms après la dernière frappe.                                                 | Must     |
| RH3 | Ouvrir un résultat      | `1.5-a` | ↑ ↓ choisissent, Entrée ouvre dans la zone active, Ctrl+Entrée dans une autre zone, Échap ferme et rend le focus.                               | Must     |
| RH4 | Filtrer par étiquette   | `1.5-a` | `#smoke paie` combine étiquette et nom (R1).                                                                                                    | Should   |
| RH5 | Chercher par adresse    | `1.5-b` | `POST /orders` : correspondance exacte, endpoints de `/orders`, tests qui l'appellent (R2).                                                     | Must     |
| RH6 | Coller une URL complète | `1.5-b` | `https://api…/orders/42` retrouve l'opération `GET /orders/{id}` (R2).                                                                          | Should   |

### Lot B - Étiquettes partagées (4.9-b)

| ID  | Cas d'usage                 | Écrans  | Comportement attendu                                                                                                | Priorité |
| --- | --------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------- | -------- |
| ET1 | Étiqueter un dossier        | `4.9-b` | « + Étiquette » dans l'en-tête de la vue du dossier (mêmes propositions que 4.9-a) ; écrit dans `folder.yml`.       | Must     |
| ET2 | Étiquettes sur chaque ligne | `4.9-b` | La vue d'un dossier montre les étiquettes effectives de chaque requête ; un sous-dossier dit « N tests · M smoke ». | Must     |
| ET3 | Filtrer la vue d'un dossier | `4.9-b` | « Étiquettes smoke × + » : « 6 tests sur 28 · les autres sont estompés » (texte estompé par sa couleur).            | Must     |
| ET4 | Même filtre partout         | `4.9-b` | Le filtre du référentiel et la recherche prennent les étiquettes effectives (R4).                                   | Must     |

### Lot D - Ligne de commande (5.3-a, 5.3-c, 4.9-d)

| ID  | Cas d'usage                  | Écrans  | Comportement attendu                                                                                                                                  | Priorité |
| --- | ---------------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| LC1 | Lancer un dossier ou un test | `5.3-a` | `arnod run "commandes/v2" --env "Recette fonctionnelle"` : en-tête (version, projet, environnement), une ligne par test, résumé, code de sortie (R7). | Must     |
| LC2 | Rapports                     | `5.3-a` | `--rapport junit,allure,html` écrit `rapports/junit.xml`, `rapports/allure-results/`, `rapports/rapport.html` ; `--rapports <dossier>` les déplace.   | Must     |
| LC3 | Lancer une étiquette         | `4.9-d` | `arnod run --etiquette smoke` : « 6 tests dans 2 dossiers » ; plusieurs `--etiquette` se cumulent (ou).                                               | Must     |
| LC4 | Variables locales et secrets | -       | `ARNOD_VAR_token` fournit `token` (R6) ; une variable manquante est nommée, code 2.                                                                   | Must     |
| LC5 | Aide et erreurs              | -       | `arnod --help`, `arnod run --help`, `arnod --version` en français ; une cible inconnue propose la plus proche.                                        | Should   |
| LC6 | Copier la commande           | `5.3-c` | Menu ⋯ d'un test ou d'un dossier : « Copier la commande », avec l'environnement actif ; notification « ✓ Commande copiée ».                           | Must     |

### Lot E - Synchronisation (7.1-a, 7.1-b, 7.1-e)

| ID  | Cas d'usage                       | Écrans  | Comportement attendu                                                                                                                                                              | Priorité |
| --- | --------------------------------- | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| SY1 | Se connecter                      | `7.1-a` | GitHub ou GitLab : connexion par le navigateur (Q2) ; « Arnod ne voit jamais votre mot de passe » ; jeton au coffre (R9). Autre URL Git : jeton saisi.                            | Must     |
| SY2 | Choisir ou créer le dépôt         | `7.1-a` | Liste des dépôts du compte (recherche), « + Créer un dépôt « boutique-tests » », branche ; « Relier et envoyer ».                                                                 | Must     |
| SY3 | Projet déjà dans Git              | `7.1-a` | Un projet ouvert depuis un clone est reconnu relié ; seule la connexion du compte est demandée, au premier envoi.                                                                 | Must     |
| SY4 | Barre d'état                      | `7.1-b` | En bas de la fenêtre principale : « Pas encore partagé », « À jour avec l'équipe », « 3 à récupérer · 2 à envoyer », « 1 conflit à résoudre » ; un clic ouvre la synchronisation. | Must     |
| SY5 | Voir ce qui arrive et ce qui part | `7.1-b` | « À récupérer » (élément, ce qui a changé, auteur, quand) ; « Mes modifications » par élément, à cocher ; message ; « vérifié il y a 1 min ».                                     | Must     |
| SY6 | Récupérer puis envoyer            | `7.1-b` | Récupère, fusionne (R12), envoie les éléments cochés avec le message (R11) ; « Envoyer seulement » reste offert quand rien n'arrive.                                              | Must     |
| SY7 | Récupérer                         | `7.1-b` | Les onglets ouverts se rechargent (lot I) ; un brouillon non enregistré est enregistré d'abord ou arrête la récupération (`useDrafts`).                                           | Must     |
| SY8 | Secrets manquants                 | `7.1-e` | « 2 secrets à renseigner pour Recette fonctionnelle : token, cardKey » ; « Renseigner » ouvre la page des environnements, « Plus tard » ferme (R15).                              | Must     |

### Lot F - Conflits et équipe (7.1-c, 7.1-d)

| ID  | Cas d'usage              | Écrans  | Comportement attendu                                                                                                                                            | Priorité |
| --- | ------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| CF1 | Régler un conflit        | `7.1-c` | « 1 conflit à résoudre » : pour chaque élément changé des deux côtés, « Vous » ou l'auteur, avec date ; « Garder les deux assertions » pour une ligne de liste. | Must     |
| CF2 | Ce qui est déjà fusionné | `7.1-c` | « Corps de la requête ✓ fusionné automatiquement · seul Marc l'a modifié » ; « Voir le YAML ».                                                                  | Must     |
| CF3 | Valider ou abandonner    | `7.1-c` | « Valider la résolution » écrit et permet l'envoi ; abandonner remet le dépôt d'avant la récupération (R13).                                                    | Must     |
| CF4 | Pastilles                | `7.1-d` | Un élément changé par un collègue depuis la dernière récupération porte sa pastille (initiale) dans la barre latérale et la vue du dossier.                     | Should   |
| CF5 | Historique d'un test     | `7.1-d` | Onglet « historique » d'un test : qui, quand, ce qui a changé, en clair (R14) ; « Voir ce qui a changé » montre la comparaison.                                 | Should   |

## 5. Hors périmètre

- Branches : une seule branche suivie par projet, choisie en le reliant ; ni création de
  branche, ni demande de fusion (pull request) depuis Arnod.
- SSH : seule l'adresse HTTPS d'un dépôt est gérée (Q1).
- Étape manuelle dans un terminal (`5.3-b`) et campagnes (`4.9-c`, campagne de `5.3-c`) : V7.
- Résultats « Exigences », « Mocks », « Anomalies » de la recherche : V5 à V7. Entrées « mocks »
  et « catalogue » de la barre d'état : V5 et V6.
- Intégration continue (`5.3-d`, Q4) : ni image Docker, qui demande une installation sur le
  poste, ni archives pour les agents, ni exemples GitHub Actions et Jenkins, ni « Intégration
  continue… ». `arnod run` garde ses codes de sortie et ses rapports.
- macOS (Q9).
- Synchronisation de « Non classés » : ce projet reste propre au poste.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust. La ligne de
commande appelle le même cœur, sans Tauri.

### 6.1 Formats sur disque

- **Projet** (partagé) : `folder.yml` reçoit `info.tags` (prévu par le schéma OpenCollection 1.0.0,
  `FolderInfo.tags`), écrit après validation comme tout fichier. Rien d'autre ne change dans le
  projet ; aucune clé n'est ajoutée à `extensions.arnod` (Q7).
- **Dépôt** : `.git/` du dossier du projet, géré par la bibliothèque Git (Q1). L'adresse du dépôt
  est dans `.git/config`, sans identifiant ni jeton (R9). « Relier » initialise le dépôt s'il
  n'existe pas : la règle « Arnod n'initialise pas Git » est levée pour ce seul geste (Q10).
- **Poste** : `projects/<id>/sync.json` (format 1, écriture atomique) : compte relié, dernière
  vérification, sommet connu de la branche distante à la dernière récupération (pour les
  pastilles, CF4), choix « Plus tard » des secrets. Jeton d'un compte au coffre, clé
  `arnod-git:<hôte>:<compte>`.
- **Rapports** (`arnod run`) : `rapports/junit.xml`, `rapports/allure-results/*-result.json`,
  `rapports/rapport.html` (un fichier autonome, sans ressource distante).

### 6.2 Cœur Rust

- **Espace de travail Cargo** (lot C) : `src-tauri/` devient l'application ; le cœur passe dans
  une bibliothèque sans Tauri (`src-tauri/core`, crate `arnod-core` : la cible `src-tauri/target`
  et les scripts qui la lisent ne changent pas) : modèle et fichiers (`request`, `tree`,
  `opencollection`, `manifest`, `environment`, `local`, `contract`), envoi (`http`, `network`,
  `secrets`), établi (`workbench`, `statuses`, `history`, `results`), recherche, rapports,
  synchronisation. Le seul lien de l'établi à Tauri (`tauri::async_runtime::spawn` d'un run)
  passe par un trait `Spawner`. Les tests qui lisent des fichiers du dépôt (`ts_contract`,
  `ts_mirror`, `interface/tests.rs`) gardent leurs chemins par `CARGO_MANIFEST_DIR`.
- **`src-tauri/cli`** (crate `arnod-cli`) : le binaire `arnod` ; lecture des arguments écrite à la main ou par
  `lexopt` (sans dépendance lourde), sortie en français, un `Notifier` qui écrit l'avancement,
  un `SecretStore` qui lit `ARNOD_VAR_*` puis, s'il existe, le coffre (R6). Exécutable autonome
  (R16) : sous Linux, lié statiquement (cible `x86_64-unknown-linux-musl`), sans le coffre (pas de
  D-Bus sur un agent : les variables passent par `ARNOD_VAR_*`) ; sous Windows, coffre compris.
- **`search/`** (pur) : index d'un projet (`tree::read`, contrats, environnements), normalisation,
  correspondance d'adresse (réutilise `contract::deduce::path_matches`), tri et groupes (R1 à R3).
- **`tree`** : étiquettes effectives (`TreeNode.inheritedTags`, R4) ; `tree_set_folder_tags`.
- **`report/`** (pur) : JUnit, Allure, HTML depuis `RunView` (R8).
- **`command/`** (pur) : commande `arnod run` d'un test ou d'un dossier, avec l'environnement
  actif, citée pour PowerShell ou sh (LC6).
- **`sync/`** : `git.rs` (bibliothèque Git, Q1), `transport.rs` (transport HTTPS « smart » de Git
  au-dessus de `network::Network`, R9), `forge.rs` (API GitHub et GitLab : compte, dépôts,
  création, connexion par le navigateur, Q2), `describe.rs` (pur : fichiers changés → éléments
  partagés, résumé en clair, R14), `merge.rs` (pur : fusion à trois versions élément par élément,
  bâtie sur `watch::conflict`, R12), `state.rs` (barre d'état, `sync.json`). Une fusion écrite
  passe par `watch::record_write` comme toute écriture d'Arnod.

### 6.3 Commandes IPC

| Commande               | Paramètres                                | Réponse          | Erreurs (`kind`)                  |
| ---------------------- | ----------------------------------------- | ---------------- | --------------------------------- |
| `search_query`         | `workspace?`, `text`                      | `SearchResults`  | -                                 |
| `tree_set_folder_tags` | `project`, `path`, `tags`                 | `TreeView`       | `invalidRequest`, `changedOnDisk` |
| `cli_command`          | `project`, `path`, `environment`, `shell` | `string`         | `notFound`                        |
| `sync_status`          | `project`                                 | `SyncView`       | -                                 |
| `sync_connect`         | `forge`, `host?`                          | `SyncAccount`    | `network`, `cancelled`            |
| `sync_repositories`    | `account`, `query`                        | `Repository[]`   | `network`                         |
| `sync_link`            | `project`, `remote`, `branch`, `create`   | `SyncView`       | `network`, `invalidRequest`       |
| `sync_check`           | `project`                                 | `SyncView`       | `network`                         |
| `sync_pull_push`       | `project`, `items`, `message`, `push`     | `SyncView`       | `network`, `conflict`             |
| `sync_conflicts`       | `project`                                 | `SyncConflicts`  | `notFound`                        |
| `sync_resolve`         | `project`, `choices`                      | `SyncView`       | `invalidRequest`                  |
| `sync_abandon`         | `project`                                 | `SyncView`       | -                                 |
| `sync_journal`         | `project`, `path`                         | `JournalEntry[]` | -                                 |
| `sync_missing_secrets` | `project`                                 | `MissingSecrets` | -                                 |

Nouvelle nature d'erreur `conflict` (Rust, `src/types/ipc.ts`, `COMMAND_ERROR_KINDS`). Événement
`arnod://synchro` (`SyncView`) après chaque vérification, récupération ou envoi : la barre d'état
de chaque fenêtre suit.

### 6.4 Frontend

- Store `search` et `SearchPalette.vue` (motif combobox, `aria-activedescendant`, focus rendu à
  la fermeture) ; action « Rechercher » (Ctrl+K) ajoutée à `interface::ACTIONS` et à
  `DEFAULT_SHORTCUTS`.
- `FolderTab.vue` : étiquettes du dossier (`TagsEditor`), étiquettes par ligne, filtre
  d'étiquettes ; `lib/treeFilter.ts` prend les étiquettes effectives.
- Menu ⋯ d'un test ou d'un dossier : « Copier la commande ».
- Store `sync`, `StatusBar.vue` (monté par `AppShell`, `role="status"`), `SyncPanel.vue` (7.1-b),
  `LinkDialog.vue` (7.1-a), `ConflictTab.vue` (contenu `conflict`, 7.1-c), `MissingSecrets.vue`
  (7.1-e), pastilles dans `TreeRow` et `FolderTab`, onglet « historique » d'un test (7.1-d).

### 6.5 Design

- Barre d'état : hauteur `--control-sm`, fond `--color-sidebar`, texte `--text-small` ; aucune
  couleur nouvelle prévue. Une pastille d'auteur reprend les couleurs des workspaces
  (`--color-ws-*`) ; si une nouvelle teinte est nécessaire, elle est définie pour les deux thèmes
  avec sa note dans `scripts/design-system.mjs`.
- Clavier : la palette, le panneau de synchronisation et le conflit sont utilisables sans souris
  (onglets par `onTablistKeydown`, listes à cocher au clavier).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `CLAUDE.md`                           | Sections « Recherche », « Ligne de commande », « Synchronisation » ; arborescence (cœur, ligne de commande)   |
| `README.md`                           | Fonctions, `arnod run`, « Publier une version »                                                               |
| `src-tauri/`, `Cargo.toml`            | Espace de travail Cargo, cœur déplacé dans `src-tauri/core` (`arnod-core`, lot C)                             |
| `src-tauri/capabilities/default.json` | Inchangé : Git, forge et navigateur appelés depuis Rust                                                       |
| `.github/workflows/build.yml`         | Exécutable `arnod` : Windows (livré avec l'application), Linux (joint à la release)                           |
| Installeur                            | Mesuré avec la ligne de commande et la bibliothèque Git : reste < 15 Mo (`bundle-size.mjs`)                   |
| `e2e/`                                | Recherche, étiquette d'un dossier, synchronisation contre un dépôt Git local de test                          |
| `docs/maquette-etat.yml`              | 1.5-a, b ; 4.9-b, d ; 5.3-a, c ; 5.3-d sorti de la V2 ; 7.1-a à e ; écart de 1.6-f bis (« Rechercher » listé) |

## 8. Plan de tâches pour Claude Code

Un lot = une branche `claude/v2-<lot>` et une pull request, titrée en Conventional Commits.
`npm run check` passe avant chaque commit ; chaque tâche est relue avant commit. Ordre : A et B
d'abord (courts, visibles), puis C, D, puis E, F.

**Lot A - Recherche (1.5-a, 1.5-b)**

- [x] **T1** `search/` : index, normalisation, correspondance d'adresse, tri et groupes (R1 à R3),
      tests règle par règle.
- [x] **T2** `search_query`, types IPC, faux cœur.
- [x] **T3** `SearchPalette.vue`, store `search`, action « Rechercher » (Ctrl+K) ; Vitest au
      clavier ; e2e ; 1.6-f bis : écart retiré.

**Lot B - Étiquettes partagées (4.9-b)**

- [x] **T4** Étiquettes d'un dossier (`folder.yml`) et étiquettes effectives (R4), `tree_set_folder_tags`.
- [x] **T5** `FolderTab` : étiquettes du dossier, par ligne, filtre ; filtre du référentiel et
      recherche sur les étiquettes effectives ; Vitest, e2e.

**Lot C - Cœur sans Tauri**

- [x] **T6** Espace de travail Cargo, `src-tauri/core` (`arnod-core`), trait `Spawner` ; aucune différence
      visible ; `cargo test`, `npm run check`, e2e verts ; installeur mesuré.

**Lot D - Ligne de commande (5.3-a, 5.3-c, 4.9-d)**

- [ ] **T7** `src-tauri/cli` (`arnod-cli`) : `arnod run` (cibles, `--env`, `--etiquette`, `--parallele`),
      sortie, codes de sortie (R5 à R7) ; tests contre un serveur local.
- [ ] **T8** `report/` : JUnit, Allure, HTML (R8), validés contre leurs schémas.
- [ ] **T9** « Copier la commande » (`cli_command`, LC6) ; `arnod.exe` livré avec l'application
      Windows, `arnod` Linux joint à la release (Q3, R16).

**Lot E - Synchronisation (7.1-a, 7.1-b, 7.1-e)**

- [ ] **T10** `sync/git.rs` et `sync/transport.rs` (R9) ; tests contre un serveur Git HTTP local.
- [ ] **T11** `sync/forge.rs` : connexion GitHub et GitLab (Q2), dépôts, création ; jeton au coffre.
- [ ] **T12** Relier (7.1-a), `sync.json`, barre d'état et vérification périodique (Q6).
- [ ] **T13** Récupérer puis envoyer (7.1-b, R10, R11), éléments partagés (`describe.rs`),
      brouillons (`useDrafts`), secrets manquants (7.1-e, R15) ; e2e sur un dépôt local.

**Lot F - Conflits et équipe (7.1-c, 7.1-d)**

- [ ] **T14** `sync/merge.rs` (R12, R13), `ConflictTab.vue` (7.1-c).
- [ ] **T15** Pastilles et historique en clair (7.1-d, R14).

## 9. Critères d'acceptation

- [ ] `cargo test` : règles R1 à R16, dont fusion et transport Git contre un serveur local.
- [ ] Vitest : recherche au clavier, étiquettes d'un dossier, barre d'état, synchronisation,
      conflit, secrets manquants, sur le faux cœur.
- [ ] e2e : Ctrl+K ouvre et trouve un test par nom et par adresse ; une étiquette de dossier filtre
      sa vue ; un projet se relie à un dépôt local, envoie, récupère la modification d'un « collègue »
      (second clone), règle un conflit.
- [ ] `arnod run`, sous Windows et sous Linux, sans installation à part, joue un dossier contre
      un serveur local, écrit JUnit et Allure, rend 1 sur un échec.
- [ ] Installeur Windows < 15 Mo ; `npm run check` passe.

## 10. Risques

| Risque                                            | Impact                              | Parade                                                                              |
| ------------------------------------------------- | ----------------------------------- | ----------------------------------------------------------------------------------- |
| Bibliothèque Git trop lourde pour l'installeur    | Installeur > 15 Mo                  | Mesure dès T10 ; sans HTTPS ni SSH propres (transport sur reqwest) ; repli Q1       |
| Proxy et autorités d'entreprise ignorés par Git   | Synchronisation impossible au poste | Transport HTTPS sur `network::Network` (R9), testé avec un proxy local              |
| Applications OAuth à enregistrer (GitHub, GitLab) | Connexion par le navigateur bloquée | Q2 ; jeton personnel au coffre en repli, pour tout hôte                             |
| Fusion qui perd une modification                  | Perte de données de l'équipe        | Fusion élément par élément (R12), testée sur des cas réels ; abandon possible (R13) |
| Extraction du cœur (lot C) qui casse un parcours  | Régression                          | Lot sans changement visible, suites complètes (Rust, Vitest, e2e) avant et après    |
| Binaire Windows de l'application sans console     | `arnod run` muet sous Windows       | Binaire de ligne de commande séparé (Q3)                                            |
| Exécutable Linux qui dépend du système            | Ne démarre pas sur un poste Linux   | Liaison statique (musl, R16), testée sur un système minimal                         |

## 11. Décisions

Questions ouvertes, à trancher avant la première tâche du lot concerné.

| ID  | Question                                                                                        | Décision                                                                                                                                                                                                                                                                                                                                                                                             |
| --- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | Quelle bibliothèque Git ? (lot E)                                                               | À trancher. Recommandé : `git2` (libgit2) sans HTTPS ni SSH propres, avec un transport HTTPS enregistré qui passe par `network::Network` (proxy, autorités internes, R9). Autres : `git` du système (rien à embarquer, mais Git requis et proxy à part) ; `gix` (pur Rust, envoi encore incomplet).                                                                                                  |
| Q2  | Connexion « par le navigateur » : qui enregistre les applications OAuth d'Arnod ? (lot E)       | À trancher. Recommandé : une application OAuth GitHub (flux « device ») et une GitLab.com (PKCE) au nom du projet Arnod ; pour un GitLab auto-hébergé ou une autre URL, un jeton personnel saisi, rangé au coffre.                                                                                                                                                                                   |
| Q3  | Comment livrer `arnod` ? (lot D)                                                                | À trancher. Recommandé : un exécutable autonome (R16). Sous Windows, `arnod.exe` livré avec l'application, dans un dossier `bin` ajouté au `PATH` de l'utilisateur par l'installeur `currentUser` ; l'application garde son binaire sans console, qui s'appelle déjà `arnod.exe` (`mainBinaryName`) mais n'est pas sur le `PATH`. Sous Linux, l'exécutable joint à la release, à côté de l'AppImage. |
| Q4  | Intégration continue (5.3-d : image Docker `arnod/arnod`, exemples GitHub Actions et Jenkins) ? | Décidé le 2026-11-27 : retirée de la V2 (ancien lot E). Pas d'image Docker, qui demande une installation sur le poste ; ni archives pour les agents, ni exemples, ni « Intégration continue… ». L'écran 5.3-d est à redécider.                                                                                                                                                                       |
| Q5  | Ce que `arnod run` écrit sur le poste                                                           | Proposé : rien hors des rapports (R5), pour des runs reproductibles d'un poste à l'autre.                                                                                                                                                                                                                                                                                                            |
| Q6  | Fréquence de la vérification de synchronisation                                                 | Proposé : au démarrage, puis toutes les 5 minutes et au retour sur la fenêtre ; « vérifié il y a … » dans le panneau.                                                                                                                                                                                                                                                                                |
| Q7  | « Étiquettes communes au projet » : simple réunion des étiquettes des fichiers, ou registre ?   | Proposé : réunion des étiquettes des requêtes et des dossiers (rien dans `extensions.arnod`) ; un registre décrit viendrait avec les campagnes (V7).                                                                                                                                                                                                                                                 |
| Q8  | Conflit sur un fichier qui n'est pas une requête (contrat, `opencollection.yml`)                | Proposé : environnements variable par variable ; contrat et racine : garder une version entière, avec la comparaison des deux.                                                                                                                                                                                                                                                                       |
| Q9  | macOS et Linux (question de la page 0.1-a)                                                      | Proposé : l'application reste Windows et Linux (AppImage, déjà construite) ; `arnod` en V2 pour Windows et Linux (exécutable autonome) ; macOS hors V2.                                                                                                                                                                                                                                              |
| Q10 | Levée de « Arnod n'initialise pas Git »                                                         | Proposé : seulement pour « Relier » (7.1-a), sur un projet qui n'est pas déjà un dépôt.                                                                                                                                                                                                                                                                                                              |
| Q11 | Auteur des modifications envoyées                                                               | Proposé : nom et adresse du compte relié (adresse privée « noreply » de la forge si l'adresse n'est pas publique).                                                                                                                                                                                                                                                                                   |
