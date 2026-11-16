# Plan - MVP, lot D : barre latérale

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** Develop @e10ba9a

Plan du lot D de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : filtre de l'arborescence,
projet créé et ses points de départ, menu « + » complet, réponses attendues dans l'arborescence et
nombre de contrats sur les cartes de projet. Il prolonge
[`plan-barre-laterale-projet.md`](plan-barre-laterale-projet.md) et
[`plan-requetes-dans-contrats.md`](plan-requetes-dans-contrats.md), respecte les règles de
[`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan d'ensemble (vocabulaire
inchangé, R4 ; ajouts en fin de liste dans les fichiers partagés, R5).

Source : maquette au 13 novembre 2026, pages 1.2 (« Arnod - Barre latérale du projet »), 3.2
(« Arnod - Endpoints ») et 1.1 (« Arnod - Workspaces »).

## 1. Objectif

À la fin de ce plan, Arnod :

- filtre le référentiel de tests d'un projet par nom, adresse, `#étiquette` ou méthode, dossiers des
  résultats dépliés et compte affiché (1.2-e) ;
- montre un projet créé avec ses deux sections repliées et quatre points de départ (1.2-d) ;
- propose dans le « + » du projet requête, dossier, contrat et environnement, avec leurs
  raccourcis (1.2-a ter) ;
- liste sous chaque endpoint ses réponses attendues, avec leur nombre ; un clic ouvre la réponse
  (3.2-a3) ;
- affiche le nombre de contrats de service sur les cartes de projet d'un workspace (écart 1.1-a).

## 2. Maquettes

| Repère    | Écran                                         | Dans ce plan |
| --------- | --------------------------------------------- | ------------ |
| 1.2-e     | Filtrer l'arborescence                        | F1 à F5      |
| 1.2-d     | Projet créé, quatre points de départ          | D1, D2       |
| 1.2-a ter | Projet vide et menu « + »                     | M1, M2       |
| 3.2-a3    | Réponses attendues dans l'arborescence        | E1 à E3      |
| 1.1-a     | Cartes de projet (écart : nombre de contrats) | W1           |

## 3. Modèle et règles

- **Filtre** : texte saisi en tête du référentiel, découpé en termes (espaces). Une requête
  correspond si **chaque** terme correspond : `#mot` à une étiquette qui commence par `mot` ; un
  terme écrit en capitales qui est une méthode (`GET`, `POST`…) à la méthode ; tout autre terme au
  nom ou à l'adresse (sans tenir compte de la casse ni des accents).
- **Réponse attendue** : exemple (`examples`) du fichier OpenCollection de la requête, avec son code
  et son nom (`request/example.rs`, EP8).

Le filtre est un filtre d'affichage, fait dans l'interface (décision Q4 du plan d'ensemble) ; le
cœur fournit ce qu'il faut pour filtrer et afficher.

| ID  | Règle                                                                                                                                                       |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | `TreeNode::Request` porte l'adresse (`url`) et les étiquettes (`tags`, sans l'étiquette `contract:`) de la requête ; vides pour un fichier illisible.       |
| R2  | `LinkedView` porte les réponses attendues de la requête liée (`examples` : index, code, nom), dans l'ordre du fichier.                                      |
| R3  | Le filtre ne garde que les requêtes qui correspondent et les dossiers qui en contiennent ; ces dossiers sont dépliés tant que le filtre est actif.          |
| R4  | Le compte du filtre est « N sur M » : requêtes retenues sur requêtes du référentiel.                                                                        |
| R5  | `contract_count` compte les fichiers de `contracts/` du projet, sans les lire.                                                                              |
| R6  | Une section vide (aucun contrat, aucune requête) est repliée par défaut ; dépliée, elle montre sa ligne d'ajout. Le choix de l'utilisateur prime (mémoire). |

## 4. Cas d'usage

| ID  | Cas d'usage             | Écrans    | Comportement attendu                                                                                                                                                                                        | Priorité |
| --- | ----------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| F1  | Filtrer                 | 1.2-e     | Un champ (`SearchInput`) en tête du référentiel dès qu'il a une requête ; la saisie filtre aussitôt ; « 2 sur 66 » dans le champ ; × efface.                                                                | Must     |
| F2  | Termes                  | 1.2-e     | Nom et adresse ; `#étiquette` ; méthode en capitales ; plusieurs termes se cumulent. La partie trouvée du nom est surlignée ; trouvé seulement dans l'adresse, la ligne le dit (« adresse »).               | Must     |
| F3  | Dossiers dépliés        | 1.2-e     | Les dossiers des résultats sont dépliés pendant le filtre, sans changer l'état replié enregistré ; un clic ouvre la requête et garde le filtre.                                                             | Must     |
| F4  | Aucun résultat          | 1.2-e     | « Aucune requête ne correspond. » à la place de l'arborescence.                                                                                                                                             | Must     |
| F5  | Clavier                 | 1.2-e     | Échap efface (puis quitte le champ) ; Ctrl+F met le focus dans le filtre du projet sélectionné quand la barre latérale a le focus ; une aide « Nom, adresse, #étiquette, méthode · Échap efface » au focus. | Should   |
| D1  | Sections du projet créé | 1.2-d     | Sections « Contrats de service » et « Référentiel de tests » avec un chevron ; vides, elles sont repliées, sans ligne d'ajout (R6).                                                                         | Must     |
| D2  | Quatre points de départ | 1.2-d     | Projet ouvert et vide : « Par où commencer ? » et quatre cartes : Reprendre l'existant (Importer…), Décrire un contrat de service (+ Contrat), Écrire une requête (+ Requête), Configurer un environnement. | Must     |
| M1  | Menu « + » du projet    | 1.2-a ter | Nouvelle requête Ctrl+N, Nouveau dossier Ctrl+Maj+N ; Ajouter un contrat ▸ ; Nouvel environnement…. La création déplie la section concernée.                                                                | Must     |
| M2  | Raccourci du dossier    | 1.2-a ter | Ctrl+Maj+N crée un dossier dans le projet sélectionné (règle B6).                                                                                                                                           | Should   |
| E1  | Nombre de réponses      | 3.2-a3    | La ligne d'une opération montre le nombre de réponses attendues de son endpoint.                                                                                                                            | Must     |
| E2  | Réponses par code       | 3.2-a3    | Dépliée, l'opération liste les réponses attendues de son endpoint (code coloré par famille, nom), puis ses autres requêtes (essais, copies).                                                                | Must     |
| E3  | Ouvrir une réponse      | 3.2-a3    | Un clic sur une réponse ouvre l'endpoint sur l'onglet « Réponses attendues », la réponse ouverte en modification.                                                                                           | Must     |
| W1  | Contrats sur les cartes | 1.1-a     | « 3 contrats de service », « 1 contrat de service », « aucun contrat de service » sous le dossier d'un projet présent.                                                                                      | Must     |

## 5. Hors périmètre

- « Nouveau test » et « Requête libre » du menu « + » : la notion de test vient du lot E, le
  vocabulaire du lot K (R4) ; ici « Nouvelle requête ».
- Filtre des requêtes rangées sous les contrats : la maquette ne le montre que sur le référentiel.
- Recherche dans tout le workspace (Ctrl+K) : V2.
- Import d'une collection Bruno depuis « Reprendre l'existant » : lot C (2.3-b) ; ici « Importer… »
  importe un fichier OpenAPI, ce que l'application sait déjà faire.
- « Plus » et les autres sections (1.2-d bis) : V4.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

Aucun changement.

### 6.2 Cœur Rust

- `tree::TreeNode::Request` gagne `url` et `tags` (R1), lus avec la requête.
- `contract::file::Linked` garde les réponses attendues ; `LinkedView.examples` (R2) :
  `ExampleRef { index, status, name }`.
- `contract::file::count(root)` (R5).

### 6.3 Commandes IPC

| Commande         | Paramètres | Réponse                                    | Erreurs (`kind`)        |
| ---------------- | ---------- | ------------------------------------------ | ----------------------- |
| `tree_load`      | `project`  | `TreeView` (requêtes : `url`, `tags`)      | inchangées              |
| `contract_list`  | `project`  | `ContractSummary[]` (`examples` des liées) | inchangées              |
| `contract_count` | `project`  | `number`                                   | `projectNotFound`, `io` |

### 6.4 Frontend

- `lib/treeFilter.ts` (pur, testé) : lecture du filtre, correspondance, arborescence filtrée, compte,
  partie à surligner.
- Store `sidebar` (nouveau) : texte du filtre par projet, sections dépliées (R6), demande d'ouverture
  d'une réponse attendue (E3, lue par `ResultSide` et `ExpectedResponses`).
- `TreeFilter.vue` : champ, compte, effacement, aide. `TreePanel.vue` et `TreeRow.vue` : filtre,
  surlignage, mention « adresse ».
- `SectionHeading.vue` : chevron facultatif (`expanded`, événement `toggle`).
- `ContractsSection.vue` : opération avec nombre et réponses attendues, ⋯ de l'endpoint (RC5).
- `ProjectWelcome.vue` : quatre points de départ ; `useContractAdd` partage l'ajout d'un contrat
  avec `ContractsSection`.
- `SidebarProject.vue`, `AppSidebar.vue` : menu « + », Ctrl+Maj+N, Ctrl+F.
- `ProjectCard.vue` : nombre de contrats (store `contracts`, `loadCount`).

### 6.5 Design

Aucun jeton ajouté : codes des réponses aux couleurs de `ExpectedResponses` (succès, lien, danger),
surlignage `--color-accent-soft`. Le champ suit `SearchInput` (`--control-md`).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                |
| ------------------------------------- | --------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Lot D - Barre latérale »                        |
| `docs/maquette-etat.yml`              | 1.2-e, 1.2-d, 1.2-a ter, 3.2-a3 livrés ; écart 1.1-a levé |
| `src-tauri/capabilities/default.json` | Inchangé                                                  |
| `e2e/`                                | `describe` « Lot D - Barre latérale » en fin de fichier   |
| Installeur                            | Aucune dépendance ajoutée                                 |

## 8. Plan de tâches

- [x] **T1** Cœur : `url` et `tags` des requêtes, réponses attendues des requêtes liées,
      `contract_count` ; tests règle par règle et forme JSON.
- [x] **T2** Types, `ipc`, faux cœur ; `lib/treeFilter.ts` et ses tests.
- [x] **T3** Filtre du référentiel (F1 à F5), sections repliables (D1, R6).
- [x] **T4** Points de départ (D2), menu « + » et raccourcis (M1, M2).
- [x] **T5** Réponses attendues sous l'endpoint (E1 à E3) ; contrats sur les cartes (W1).
- [x] **T6** Vitest (clavier compris), aperçu comparé à la maquette, e2e écrits, `maquette-etat.yml`,
      `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] « stat » filtre le référentiel, dossiers dépliés, « 2 sur N » affiché ; Échap efface (Vitest).
- [x] Un projet créé montre deux sections repliées et quatre points de départ (Vitest, aperçu).
- [x] Un clic sur « 201 » sous `POST /orders` ouvre la réponse 201 de l'endpoint (Vitest).
- [ ] Parcours e2e du lot sur l'application réelle (à l'intégration).

## 10. Risques

| Risque                                           | Impact                       | Parade                                                     |
| ------------------------------------------------ | ---------------------------- | ---------------------------------------------------------- |
| Filtre sur un gros référentiel                   | Saisie lente                 | Filtre pur, en mémoire, sans appel au cœur                 |
| Endpoint qui n'est plus une ligne à part         | Renommer ou supprimer perdus | Menu ⋯ sur la ligne de l'opération (RC5)                   |
| Ouvrir une réponse dans une autre fenêtre        | Réponse non choisie          | Demande gardée par fenêtre ; l'onglet s'ouvre quand même   |
| Lot C sur la même carte « Reprendre l'existant » | Conflit de fusion            | Carte isolée dans une liste de données de `ProjectWelcome` |

## 11. Décisions

| ID  | Question                                            | Décision (2026-11-13)                                                                                      |
| --- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Q1  | Filtre aussi sous les contrats ?                    | Non : la maquette ne le montre que sur le référentiel.                                                     |
| Q2  | Méthode : `get` en minuscules ?                     | Un terme en minuscules cherche dans le nom et l'adresse ; seule la méthode en capitales filtre la méthode. |
| Q3  | Projet vide : sections repliées ou lignes d'ajout ? | Repliées (1.2-d) ; dépliées, elles montrent leur ligne d'ajout (1.2-a ter).                                |
| Q4  | Où va l'endpoint quand son opération se déplie ?    | La ligne de l'opération est l'endpoint (clic, ⋯) ; dessous ses réponses, puis les autres requêtes.         |
| Q5  | Nombre de contrats sans les lire ?                  | Oui : `contract_count` compte les fichiers (R5) ; un fichier illisible compte.                             |
