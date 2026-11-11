# Plan - Requêtes rangées dans leur contrat

**Version** 1.0 · **Date** 2026-11-11 · **Statut** Livré · **Base** version 0.0.2 + lot « sélecteur
de workspace et icônes des sections »

Plan d'implémentation du rangement des requêtes liées à un contrat **sous ce contrat**, dans la barre
latérale, et de la disparition de la section « Requêtes ». Il révise la règle R5 de
[`plan-contrats-services.md`](plan-contrats-services.md) (pour l'affichage, pas pour le disque) et
le cas I1 de [`plan-selecteur-workspace.md`](plan-selecteur-workspace.md), et respecte les règles de
[`CLAUDE.md`](../CLAUDE.md).

Source : demande « lors de l'import d'un contrat, on crée bien un nouveau contrat mais les requêtes
générées le sont dans le menu principal Requêtes ; ce menu ne devrait pas exister ; on devrait
mettre les requêtes dans le contrat ». Maquette Claude Design : pages « Premier usage » (b : la
barre latérale d'un projet neuf a deux sections, « Contrats de service » et « Référentiel de
tests » ; c : « Essai 3 » sous `GET /orders/{id}/tracking` du contrat déduit) et « Contrats de
service » (a : un contrat se déplie en ressources `/products`, `/orders`, puis en endpoints
`GET /orders`, `POST /orders`).

## 1. Objectif

À la fin de ce plan, Arnod :

- montre les requêtes créées depuis un contrat **dans ce contrat** : le contrat se déplie en
  ressources, les ressources en opérations (`GET /pets/{petId}`), les opérations en requêtes ;
- ouvre l'endpoint d'un clic sur son opération, et crée sa requête si elle n'existe pas encore ;
- remplace la section « Requêtes » par le **référentiel de tests**, qui ne garde que les requêtes
  sans contrat ; les contrats viennent en premier, comme dans la maquette.

## 2. Maquettes

| Écran | Page de la maquette            | Dans ce plan |
| ----- | ------------------------------ | ------------ |
| a     | Contrats de service, a         | RC1 à RC5    |
| b     | Premier usage, b               | RC6          |
| c     | Premier usage, c (Non classés) | RC8          |

Écart relevé : la page « Barre latérale du projet » montre encore un arbre de requêtes libres
(« Clients › GET lister ») au-dessus des contrats. Elle date d'avant les pages « Premier usage » et
« Contrats de service », qui font foi ; elle est à aligner.

## 3. Modèle et règles

- **Requête liée** : requête dont l'étiquette `contract:<fichier>#<opération>` désigne un fichier
  présent dans `contracts/` (lisible ou non).
- **Ressource** : premier segment du chemin d'une opération (`/pets/{petId}` → `/pets`).
- **Endpoint** : la première requête liée d'une opération, dans l'ordre de l'arborescence ; les
  suivantes sont des essais sur cet endpoint (« Essai N » de Non classés, copies).
- **Référentiel de tests** : l'arborescence du projet sans les requêtes liées.

Les règles vivent dans le cœur Rust : `tree::tests_view` et `contract::file` (groupement), purs et
testés dans `tree/tests.rs` et `contract/tests.rs`.

| ID  | Règle                                                                                                                                                                        |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Une requête liée est montrée sous son contrat, jamais dans le référentiel de tests.                                                                                          |
| R2  | Sous un contrat, les opérations sont groupées par ressource (premier segment du chemin ; `/` reste `/`), ressources et opérations dans l'ordre du contrat.                   |
| R3  | Sous une opération, ses requêtes liées dans l'ordre de l'arborescence ; la première est l'endpoint.                                                                          |
| R4  | Une requête liée dont l'opération n'est plus dans le contrat, ou dont le contrat est illisible, est listée sous le contrat dans « Hors contrat ».                            |
| R5  | Le référentiel de tests est l'arborescence sans les requêtes liées ; un dossier qui contenait des requêtes et n'en garde aucune n'y figure pas ; un dossier vide y reste.    |
| R6  | Une requête dont le contrat n'existe plus (mis à la corbeille) revient dans le référentiel de tests ; elle signale le contrat absent (C11 du plan des contrats).             |
| R7  | Sur disque, rien ne change : les requêtes créées restent dans `<nom du contrat>/<premier tag>/` (R5 du plan des contrats), lisibles par Bruno comme des dossiers ordinaires. |

## 4. Cas d'usage

| ID  | Cas d'usage               | Écrans | Comportement attendu                                                                                                                                                                              | Priorité |
| --- | ------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| RC1 | Parcourir un contrat      | a      | Le chevron d'un contrat le déplie : ses ressources, avec leur nombre d'opérations ; une ressource se déplie en opérations « MÉTHODE /chemin », une opération en requêtes. Replié par défaut.      | Must     |
| RC2 | Ouvrir l'endpoint         | a      | Un clic sur une opération ouvre son endpoint (R3) en onglet ; la ligne est mise en évidence quand l'onglet actif est l'une de ses requêtes et qu'elle est repliée, sinon la requête elle-même.    | Must     |
| RC3 | Opération sans requête    | a      | Ligne estompée, « Créer la requête » au survol ; un clic crée la requête de l'opération (R4 à R8 du plan des contrats), l'ouvre et la montre.                                                     | Must     |
| RC4 | Révéler une requête liée  | a      | Une requête liée ouverte ailleurs (onglet du contrat, recherche, autre fenêtre) déplie son contrat, sa ressource et, si elle n'est pas seule, son opération.                                      | Should   |
| RC5 | Agir sur une requête liée | a      | Menu ⋯ d'une requête : Renommer, Dupliquer (un essai de plus sur l'opération), Mettre à la corbeille ; mêmes commandes que l'arborescence.                                                        | Should   |
| RC6 | Référentiel de tests      | b      | La section « Requêtes » devient « Référentiel de tests » (icône du référentiel), sous « Contrats de service » ; elle ne montre que les requêtes sans contrat (R5) ; ses ajouts restent les mêmes. | Must     |
| RC7 | Créer les requêtes        | a      | « Créer les requêtes » de l'onglet du contrat : les requêtes apparaissent sous le contrat, qui se déplie ; rien n'apparaît dans le référentiel.                                                   | Must     |
| RC8 | Essai dans Non classés    | c      | « Essai N » apparaît sous le contrat déduit, à son opération, sans section « Requêtes ».                                                                                                          | Must     |
| RC9 | Entre fenêtres            | a, b   | Une requête créée, renommée ou supprimée, un contrat ajouté ou supprimé dans une fenêtre mettent à jour les deux sections des autres fenêtres.                                                    | Must     |

## 5. Hors périmètre

- Renommer les requêtes du référentiel en « tests » et le modèle « un test = des étapes » : lot
  « Tests et étapes » de la feuille de route.
- Déplacer les fichiers des requêtes liées sur disque (R7) : inutile pour l'affichage, et Bruno lit
  les dossiers tels quels.
- Garder l'état déplié des contrats d'une session à l'autre : en mémoire pour ce lot.
- Glisser une requête du référentiel sur une opération pour la lier : plus tard.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

Aucun changement : ni format, ni emplacement. Le lien reste l'étiquette `contract:` de `info.tags`.

### 6.2 Cœur Rust

- `tree::TreeNode::Request` gagne `contract` : le fichier du contrat lié, ou rien.
- `tree::tests_view(nodes, contracts)` (pur) : l'arborescence du référentiel (R1, R5, R6).
- `contract::file::paths(root)` : les fichiers présents dans `contracts/`.
- `contract::file::resources(contract, links)` et `resource_of(path)` (purs) : ressources,
  opérations et requêtes d'un contrat (R2, R3) ; `ContractSummary` gagne `resources` et
  `unplaced` (R4). Une requête liée (`LinkedView`) porte son chemin, son nom et sa méthode.

### 6.3 Commandes IPC

| Commande        | Paramètres | Réponse                                       | Erreurs (`kind`) |
| --------------- | ---------- | --------------------------------------------- | ---------------- |
| `tree_load`     | `project`  | `TreeView` + `tests` (référentiel)            | inchangées       |
| `contract_list` | `project`  | `ContractSummary[]` + `resources`, `unplaced` | inchangées       |

### 6.4 Frontend

- `ContractsSection.vue` : contrats dépliables, ressources, opérations, requêtes ; état déplié dans
  le store `contracts` (`expanded`), révélation de la requête active (RC4), création à la demande
  (`createForOperation`, RC3).
- `TreePanel.vue` : « Référentiel de tests », arborescence `tests`.
- `SidebarProject.vue` : contrats d'abord, puis le référentiel.
- Stores : relire l'arborescence relit les contrats du projet (et inversement), y compris pour une
  modification venue d'une autre fenêtre (RC9).

### 6.5 Design

Les lignes reprennent la grille de la barre latérale (`--tree-pad`, `--tree-indent`, `--tree-lead`,
`--tree-gap`), les pastilles de méthode et les couleurs existantes ; aucun jeton ajouté. Chevrons
avec `aria-expanded`, lignes en boutons, utilisables au clavier (Tab, Entrée, Espace).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                           |
| ------------------------------------- | -------------------------------------------------------------------- |
| `CLAUDE.md`                           | Sections « Barre latérale du projet » et « Contrats de services »    |
| `README.md`                           | Ligne I1 : « Contrats de service » et « Référentiel de tests »       |
| `docs/plan-contrats-services.md`      | R5 : renvoi vers ce plan pour l'affichage                            |
| `src-tauri/capabilities/default.json` | Inchangé                                                             |
| `e2e/`                                | Contrat : requêtes visibles sous le contrat, pas dans le référentiel |
| Installeur                            | Aucune dépendance ajoutée                                            |

## 8. Plan de tâches pour Claude Code

**Lot A - Requêtes rangées dans leur contrat** (une pull request)

- [x] **T1** Cœur : `contract` sur `TreeNode::Request`, `tests_view`, `paths`, `resources`,
      `resource_of`, `ContractSummary.resources` et `unplaced` ; tests règle par règle.
- [x] **T2** Types et faux cœur : `TreeView.tests`, `ContractSummary`, fixtures Vitest.
- [x] **T3** Interface : `ContractsSection` dépliable, `TreePanel` en référentiel, ordre des
      sections, synchronisation des deux stores.
- [x] **T4** Tests Vitest (RC1 à RC9, clavier) et e2e (contrat créé : requêtes sous le contrat).
- [x] **T5** Documentation : `CLAUDE.md`, `README.md`, plans liés.

## 9. Critères d'acceptation

- [x] Après « Créer les requêtes », aucune section « Requêtes » ; les requêtes sont sous le contrat,
      par ressource et opération (e2e).
- [x] Une requête sans contrat est dans le référentiel de tests ; un contrat supprimé y renvoie ses
      requêtes.
- [x] Un clic sur une opération ouvre son endpoint ; sur une opération sans requête, la crée.
- [x] `npm run check`, `cargo test`, `cargo clippy -D warnings` et les e2e passent.

## 10. Risques

| Risque                                          | Impact                             | Parade                                                     |
| ----------------------------------------------- | ---------------------------------- | ---------------------------------------------------------- |
| Lecture de toutes les requêtes à chaque liste   | Lenteur sur un gros projet         | Une seule lecture de l'arborescence par commande           |
| Deux sections à tenir à jour                    | Section en retard après une action | Relire l'une relit l'autre, aussi pour les autres fenêtres |
| Contrat profond (ressource, opération, requête) | Barre latérale étroite             | Repliés par défaut, révélation de la requête active        |

## 11. Décisions

| ID  | Question                                       | Décision (2026-11-11)                                                    |
| --- | ---------------------------------------------- | ------------------------------------------------------------------------ |
| Q1  | Déplacer les fichiers des requêtes liées ?     | Non (R7) : seul l'affichage change ; le disque reste lisible par Bruno.  |
| Q2  | Où vont les requêtes sans contrat ?            | Dans « Référentiel de tests », comme dans la maquette (Premier usage b). |
| Q3  | Que fait un clic sur une opération ?           | Il ouvre l'endpoint (sa première requête) ; sans requête, il la crée.    |
| Q4  | Ordre des sections                             | Contrats de service, puis Référentiel de tests (maquette).               |
| Q5  | Libellés « Nouvelle requête » du référentiel ? | Gardés : renommer en « test » relève du lot « Tests et étapes ».         |
