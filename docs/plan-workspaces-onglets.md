# Plan - Workspaces en onglets

**Version** 0.1 · **Date** 2026-11-09 · **Statut** Livré · **Base** version 0.0.2 + lot « premier usage »

Plan d'implémentation de la nouvelle gestion des workspaces : onglets en haut de la fenêtre, onglet
épinglé « Tous les projets », workspace facultatif pour un projet. Il part du lot « projets et
workspaces » ([`plan-projets-workspaces.md`](plan-projets-workspaces.md)), dont il révise les règles
R3, R4, R7, R8 et R11, et respecte les règles de [`CLAUDE.md`](../CLAUDE.md).

Source : maquette Claude Design, pages « Workspaces » (2a à 2e) et « Premier usage » (1a, 1a bis).

> **Révisé** par [`plan-selecteur-workspace.md`](plan-selecteur-workspace.md) : les onglets de
> workspaces (O1 à O7) sont remplacés par un sélecteur en tête de la barre latérale (S1 à S6).

## 1. Objectif

À la fin de ce plan, Arnod :

- affiche les workspaces en onglets en haut de la fenêtre, précédés de l'onglet épinglé
  « Tous les projets » ; un clic change de workspace, « + » en crée un ;
- ne connaît plus de workspace par défaut : « Tous les projets » est le référentiel, un projet peut
  n'être dans aucun workspace ;
- propose, par clic droit (ou Maj+F10) sur un onglet : renommer, changer la couleur, gérer les
  workspaces, supprimer avec « Annuler » ;
- ajoute un projet à un workspace en glissant sa ligne de « Tous les projets » sur un onglet, ou par
  « Ajouter à… » au clavier ;
- rouvre au démarrage le dernier onglet utilisé, workspace ou « Tous les projets ».

## 2. Maquettes

| Écran  | Maquette                                 | Dans ce plan   |
| ------ | ---------------------------------------- | -------------- |
| 1a     | Premier lancement, rangement facultatif  | O1, O8         |
| 1a bis | Créer un workspace depuis le formulaire  | O8             |
| 2a     | Ouverture habituelle, menu ⋯ d'un projet | O2, O9         |
| 2a bis | Nouveau projet, workspace prérempli      | O8             |
| 2b     | Onglets, menu contextuel, « + »          | O3, O4, O5, O6 |
| 2c     | Ajouter un projet à un workspace         | O10            |
| 2d     | Gérer les workspaces                     | O6, O11        |
| 2e     | Tous les projets, glisser sur un onglet  | O1, O7, O12    |

## 3. Modèle et règles

« Tous les projets » n'est pas un workspace : c'est le référentiel, toujours présent, affiché dans
l'onglet épinglé. Un workspace est un regroupement nommé et coloré, facultatif.

Les règles R1, R2, R5, R6, R9 et R10 du plan précédent sont inchangées. Celles-ci les remplacent :

| ID  | Règle                                                                                                                                                                                    |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R3  | Il n'y a pas de workspace par défaut. Le premier lancement ne crée aucun workspace ; tout workspace peut être supprimé.                                                                  |
| R4  | Un projet créé ou importé entre dans « Tous les projets » et, si un workspace est choisi, dans ce seul workspace.                                                                        |
| R7  | Au démarrage, Arnod rouvre le dernier onglet utilisé : un workspace, ou « Tous les projets ». S'il n'existe plus, ou au premier lancement : « Tous les projets ».                        |
| R8  | Si le workspace ouvert est supprimé, Arnod revient sur « Tous les projets ». « Annuler » le rouvre.                                                                                      |
| R11 | Les onglets suivent l'ordre de création : un nouveau workspace s'ajoute à la fin. Les projets suivent leur ordre d'ajout.                                                                |
| R12 | Migration d'`arnod.json` version 1 : l'ancien workspace par défaut est retiré s'il contient tous les projets (il doublait « Tous les projets ») ; sinon il reste un workspace ordinaire. |

## 4. Cas d'usage

| ID  | Cas d'usage                     | Écrans     | Comportement attendu                                                                                                                                                 | Priorité |
| --- | ------------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| O1  | Onglet « Tous les projets »     | 1a, 2e     | Toujours en tête, épinglé. Sans projet : premier lancement. Sinon : tableau des projets, recherche, « Importer… », « Nouveau projet ».                               | Must     |
| O2  | Changer de workspace            | 2a, 2b     | Clic sur un onglet, ou flèches puis Entrée. L'onglet actif est mémorisé (R7). Les onglets défilent, avec ‹ › quand ils débordent.                                    | Must     |
| O3  | Créer un workspace              | 2b         | « + » ouvre « Nouveau workspace » près du bouton ; « Créer » ajoute l'onglet à la fin et l'ouvre. Nom vide ou pris : erreur sous le champ.                           | Must     |
| O4  | Renommer depuis l'onglet        | 2b         | Menu contextuel « Renommer » : le nom devient éditable dans l'onglet ; Entrée enregistre, Échap annule. Nom pris : notification d'erreur, édition conservée.         | Must     |
| O5  | Couleur depuis l'onglet         | 2b         | Sous-menu « Changer la couleur », couleur courante cochée.                                                                                                           | Must     |
| O6  | Supprimer un workspace          | 2b, 2d     | Sans confirmation ; notification « Workspace « X » supprimé · N projets conservés dans « Tous les projets » » avec « Annuler » (R5, R6, R8).                         | Must     |
| O7  | Glisser un projet sur un onglet | 2e         | La ligne glissée affiche « Ajouter à « X » » sur l'onglet survolé ; le dépôt ajoute le projet et le notifie. Déjà présent : rien n'est ajouté, notification.         | Should   |
| O8  | Rangement facultatif            | 1a, 2a bis | « Tous les projets » toujours ; « + Ajouter à un workspace » choisit un workspace existant ou en crée un sans quitter le formulaire. Prérempli avec l'onglet ouvert. | Must     |
| O9  | Menu ⋯ d'un projet              | 2a         | Ouvrir, Ajouter à un autre workspace ▸, Afficher dans « Tous les projets », Retirer de « X » avec « Annuler ».                                                       | Must     |
| O10 | Ajouter un projet               | 2c         | Créer, importer un projet Arnod existant, ou reprendre des projets de « Tous les projets » (cases à cocher, workspaces actuels affichés).                            | Must     |
| O11 | Gérer les workspaces            | 2d         | Liste, nom, couleur, projets avec « Retirer », « + Ajouter un projet », « Supprimer ce workspace », « + Nouveau workspace ». Plus de rôle « par défaut ».            | Must     |
| O12 | Dernière exécution              | 2e         | Colonne « Dernière exécution » (« Exécution » depuis S9 du plan du sélecteur) : requêtes réussies sur le total et heure du dernier run, sinon « — ».                 | Should   |

## 5. Hors périmètre

- Barre de titre personnalisée : les onglets sont dans la fenêtre, sous la barre native (la
  maquette les montre dans la barre de titre ; voir Q1).
- Colonne « Contrats » et compteurs de contrats : pas de contrats de service avant le lot dédié.
- Import Bruno, Postman et OpenAPI : feuille de route (V1 et plus tard). « Importer… » inscrit un
  projet Arnod existant.
- Réordonner les onglets par glisser-déposer (Could, lot suivant).
- Barre d'état (« 2 mocks actifs ») : lot « mocks ».

## 6. Architecture

### 6.1 Format sur disque

`arnod.json` passe en version 2 : `defaultWorkspaceId` disparaît, `lastWorkspaceId` devient
facultatif (`null` : « Tous les projets »).

```json
{
  "version": 2,
  "projects": [
    { "id": "…", "name": "boutique-tests", "path": "C:\\Users\\julien\\arnod\\boutique-tests" }
  ],
  "workspaces": [{ "id": "…", "name": "E-commerce", "color": "orange", "projectIds": ["…"] }],
  "lastWorkspaceId": null
}
```

Un fichier version 1 est migré à la lecture (R12) puis réécrit en version 2 à la première
modification ; un fichier version 3 ou plus est refusé sans écriture (inchangé).

### 6.2 Cœur Rust

- `catalog/` : suppression du workspace par défaut, `last_workspace_id: Option<Uuid>`,
  `open_tab(Option<Uuid>)`, création et inscription avec `Option<Uuid>`, ordre de création ;
  `CatalogFile` version 2 et `migrate_v1`.
- `CommandErrorKind::DefaultWorkspace` disparaît (plus aucun cas).
- `workbench` : `last_run_summary` pour la colonne « Dernière exécution ».

### 6.3 Commandes IPC

| Commande                | Paramètres                                | Réponse            | Changement                    |
| ----------------------- | ----------------------------------------- | ------------------ | ----------------------------- |
| `workspace_open`        | `workspaceId: string \| null`             | `CatalogSnapshot`  | `null` : « Tous les projets » |
| `project_create`        | `name`, `location`, `workspaceId \| null` | `ProjectCreated`   | workspace facultatif          |
| `project_add_existing`  | `path`, `workspaceId \| null`             | `ProjectCreated`   | workspace facultatif          |
| `workspace_set_default` | —                                         | —                  | supprimée                     |
| `project_last_runs`     | —                                         | `ProjectLastRun[]` | nouvelle                      |

`CatalogSnapshot` perd `defaultWorkspaceId` ; `lastWorkspaceId` vaut `null` pour « Tous les
projets ». `WorkspaceView` perd `isDefault`.

### 6.4 Frontend

- `components/layout/WorkspaceTabs.vue` : onglets (`role="tablist"`), défilement, « + », menu
  contextuel (`BaseMenu` ouvert par `open()` exposé), renommage dans l'onglet, cible de dépôt.
- `components/projects/AllProjects.vue` : « Tous les projets » (2e), remplace l'onglet
  « Référentiel » de l'écran de gestion ; premier lancement quand il n'y a aucun projet.
- Navigation : vues `all`, `workspace`, `project` (`workspaceId` facultatif), `manage`.
- Supprimés : `WorkspaceSwitcher.vue` (2b ancienne version), `RegistryTab.vue`, vue `firstRun`.
- Tauri : `dragDropEnabled: false` sur la fenêtre, pour que le glisser-déposer HTML fonctionne
  sous Windows (aucune permission ajoutée ; Arnod ne reçoit pas de fichiers déposés).

### 6.5 Design et clavier

Barre d'onglets sur `--color-sidebar`, onglet actif sur `--color-bg` ; aucun nouveau jeton.
Clavier : ← → Début Fin entre les onglets, Entrée ou Espace pour ouvrir, Maj+F10 ou touche Menu
pour le menu contextuel, F2 pour renommer, Suppr pour supprimer (avec « Annuler »).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                   |
| ------------------------------------- | ------------------------------------------------------------ |
| `CLAUDE.md`                           | Section « Projets et workspaces » : règles révisées, onglets |
| `README.md`                           | Tableau des fonctions                                        |
| `src-tauri/capabilities/default.json` | Inchangé                                                     |
| `src-tauri/tauri.conf.json`           | `dragDropEnabled: false`                                     |
| `e2e/`                                | Parcours adaptés : plus de « Mes projets »                   |

## 8. Plan de tâches

**Lot A - Cœur**

- [x] **T1** Catalogue sans workspace par défaut (R3, R4, R7, R8, R11), tests règle par règle.
- [x] **T2** `arnod.json` version 2 et migration de la version 1 (R12), tests sur dossier temporaire.
- [x] **T3** Commandes `workspace_open`, `project_create`, `project_add_existing` révisées,
      `workspace_set_default` retirée, `project_last_runs` ajoutée ; types IPC.

**Lot B - Interface**

- [x] **T4** Navigation (`all`, `workspace`, `project`, `manage`) et barre d'onglets (O2, O3).
- [x] **T5** Menu contextuel d'un onglet : renommer, couleur, gérer, supprimer (O4, O5, O6).
- [x] **T6** « Tous les projets » (O1, O12) et glisser-déposer sur un onglet (O7).
- [x] **T7** Formulaire de projet : rangement facultatif et création de workspace (O8).
- [x] **T8** Vue workspace, barre latérale, menu ⋯, ajout de projets, gestion (O9, O10, O11).

**Lot C - Vérification**

- [x] **T9** Tests Vitest des parcours, au clavier compris ; e2e adaptés.
- [x] **T10** Captures comparées à la maquette (clair et sombre), README et `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Premier lancement : seul l'onglet « Tous les projets » existe ; le projet créé sans workspace y
      apparaît.
- [x] « + » crée un workspace dont l'onglet s'ajoute à la fin et s'ouvre.
- [x] Le menu contextuel d'un onglet renomme, recolore et supprime (avec « Annuler ») ; tout est
      faisable au clavier.
- [x] Glisser une ligne de « Tous les projets » sur un onglet ajoute le projet au workspace.
- [x] Après relance, Arnod rouvre le dernier onglet utilisé.
- [x] Un `arnod.json` version 1 est migré sans perte de projet ni de workspace utile.

## 10. Risques

| Risque                                                  | Impact                    | Parade                                                     |
| ------------------------------------------------------- | ------------------------- | ---------------------------------------------------------- |
| Glisser-déposer HTML bloqué par la WebView sous Windows | O7 inopérant sous Windows | `dragDropEnabled: false` ; « Ajouter à… » reste disponible |
| Migration qui retire un workspace utilisé               | Regroupement perdu        | Retrait seulement s'il contient tous les projets (R12)     |
| Trop d'onglets                                          | Onglets illisibles        | Défilement horizontal et boutons ‹ ›, nom tronqué          |

## 11. Décisions

| ID  | Question                                                   | Décision (2026-11-09)                                                                               |
| --- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Q1  | Onglets dans la barre de titre, comme la maquette ?        | Non : barre native conservée (déplacement, ancrage, accessibilité). Les onglets sont juste dessous. |
| Q2  | Que devient « Mes projets » chez un utilisateur existant ? | R12 : retiré s'il double « Tous les projets », gardé sinon.                                         |
| Q3  | Où revient-on après suppression du workspace ouvert ?      | « Tous les projets » (R8), comme au premier lancement.                                              |
| Q4  | « Importer… » sans import Bruno, Postman ni OpenAPI ?      | Inscrit un projet Arnod existant ; le texte le dit, sans promettre les autres formats.              |
