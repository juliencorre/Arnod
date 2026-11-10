# Plan - Sélecteur de workspace et icônes des sections

**Version** 0.1 · **Date** 2026-11-10 · **Statut** Livré · **Base** version 0.0.2 + lot « accueil,
catalogue des étapes et Non classés »

Plan d'implémentation de la solution « b » retenue pour la barre des workspaces : les onglets de
workspaces laissent place à un **sélecteur**, et la rangée qu'ils occupaient disparaît. Il révise
les cas O1 à O7 de [`plan-workspaces-onglets.md`](plan-workspaces-onglets.md) (les règles du cœur
ne changent pas) et respecte les règles de [`CLAUDE.md`](../CLAUDE.md). Il ajoute aussi une icône
devant chaque section d'un projet dans la barre latérale.

Source : maquette Claude Design, page « Workspaces » (2a, 2b, 2b bis, 2e) et toutes les pages qui
montrent la barre de titre (version 165), page « Barre latérale » (icônes des sections, version
164).

## 1. Objectif

À la fin de ce plan, Arnod :

- montre le workspace ouvert dans un sélecteur en haut de la barre latérale, à la hauteur des
  onglets de contenu : plus de rangée d'onglets de workspaces au-dessus de la fenêtre ;
- liste dans ce sélecteur « Tous les projets » puis les workspaces (couleur, nombre de projets,
  raccourci), et y regroupe la création, le renommage, la couleur, la suppression et la gestion ;
- accepte qu'on glisse un projet sur le sélecteur pour l'ajouter à un workspace ;
- précède chaque section d'un projet (« Requêtes », « Contrats de service ») d'une icône qui dit
  son rôle.

## 2. Maquettes

| Écran  | Page de la maquette     | Dans ce plan                   |
| ------ | ----------------------- | ------------------------------ |
| 2a     | Workspaces              | S1 (sélecteur fermé)           |
| 2b     | Workspaces              | S2, S3, S5 (sélecteur ouvert)  |
| 2b bis | Workspaces              | S4 (menu « ⋯ » d'un workspace) |
| 2e     | Workspaces              | S6 (glisser un projet)         |
| toutes | barre de titre          | S1, S8 (une rangée de moins)   |
| 2a     | Barre latérale (icônes) | I1, I2                         |

## 3. Modèle et règles

Aucune règle nouvelle dans le cœur : le sélecteur appelle les commandes existantes
(`workspace_open`, `workspace_create`, `workspace_rename`, `workspace_set_color`,
`workspace_delete`, `workspace_add_projects`). Les règles R1 à R12 des deux plans précédents
restent en vigueur (noms uniques, « Tous les projets » mémorisé par `lastWorkspaceId: null`,
suppression avec « Annuler »).

## 4. Cas d'usage

| ID  | Cas d'usage                  | Écrans | Comportement attendu                                                                                                                                                                                                                            | Priorité |
| --- | ---------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| S1  | Voir le workspace ouvert     | 2a     | En haut de la barre latérale, à la hauteur des onglets de contenu : pastille de couleur (grille pour « Tous les projets »), nom, ▾. Sans barre latérale (tableau, gestion, catalogue, barre détachée) : même sélecteur dans une rangée en haut. | Must     |
| S2  | Changer de workspace         | 2b     | Clic ou ↓ sur le sélecteur : menu « Tous les projets », puis « Workspaces · N » avec couleur, nombre de projets et Ctrl+1…9 ; l'actuel est coché. Flèches, Début, Fin, Entrée ; Échap rend le focus au sélecteur. Ctrl+0 à Ctrl+9 partout.      | Must     |
| S3  | Créer un workspace           | 2b     | « + Nouveau workspace » en bas du menu ouvre « Nouveau workspace » sous le sélecteur ; « Créer » ouvre le workspace créé.                                                                                                                       | Must     |
| S4  | Renommer, couleur, supprimer | 2b bis | « ⋯ » au survol d'un workspace, clic droit, Maj+F10 ou touche Menu : Renommer (F2), Changer la couleur ▸, Gérer les workspaces…, Supprimer (Suppr) avec « Annuler ». Renommer ouvre une fenêtre de saisie ; nom pris : erreur sous le champ.    | Must     |
| S5  | Gérer les workspaces         | 2b     | « Gérer les workspaces… » (Ctrl+Maj+W) ouvre la gestion sur le workspace ouvert.                                                                                                                                                                | Must     |
| S6  | Glisser un projet            | 2e     | Une ligne de « Tous les projets » glissée sur le sélecteur l'ouvre ; le workspace survolé s'entoure et « Ajouter à « X » » s'affiche ; le dépôt ajoute le projet et le notifie ; déjà présent, rien n'est ajouté (notification).                | Should   |
| S7  | Barre latérale détachée      | -      | La fenêtre de la barre détachée porte le même sélecteur ; la fenêtre principale garde le sien.                                                                                                                                                  | Must     |
| S8  | Hauteur rendue               | toutes | Plus de rangée d'onglets de workspaces : la zone centrale commence en haut de la fenêtre.                                                                                                                                                       | Must     |
| I1  | Icônes des sections          | 2a     | Dans un projet déplié, « Requêtes » et « Contrats de service » sont des en-têtes précédés d'une icône (requête envoyée, document de contrat), avec leur nombre et « + ».                                                                        | Must     |
| I2  | Icônes cohérentes            | 2a     | Icônes en trait de 1,4 px sur 16 px, couleur du texte (thème clair et sombre), décoratives (`aria-hidden`) : le nom de la section reste le texte.                                                                                               | Must     |

## 5. Hors périmètre

- Les sections de la maquette qui n'existent pas encore dans l'application (Tableau de bord,
  Exigences, Mocks, Templates, Référentiel de tests, Suites, Campagnes, Anomalies) : leurs icônes
  sont prêtes dans `SectionIcon`, les sections viendront avec leurs lots.
- Les onglets de contenu dans la barre de titre native : Arnod garde la barre du système ; le
  sélecteur s'aligne sur la rangée d'onglets des zones.

## 6. Architecture

### 6.1 Formats sur disque

Aucun changement : `arnod.json` (format 2) et `windows.json` sont inchangés.

### 6.2 Cœur Rust

Aucun changement.

### 6.3 Commandes IPC

Aucune commande nouvelle.

### 6.4 Frontend

- `components/layout/WorkspaceSwitcher.vue` remplace `WorkspaceTabs.vue` : déclencheur, menu
  (`role="menu"`, entrées `menuitemradio`), menu d'actions (`BaseMenu` ouvert par le parent,
  comme l'ancien menu contextuel), fenêtre « Renommer le workspace » (`NameDialog`), glisser-déposer
  (`lib/dnd.ts`), raccourcis globaux Ctrl+0…9 et Ctrl+Maj+W.
- `NewWorkspacePopover.vue` s'ouvre sous le sélecteur (aligné à gauche).
- `AppShell.vue` : le sélecteur coiffe la barre latérale (hauteur de `.zone__bar`) ; sans barre
  latérale, il occupe une rangée en haut de la zone principale. `SidebarWindow.vue` : le sélecteur
  remplace les onglets compacts.
- `components/ui/SectionIcon.vue` : icônes des sections (celles de la maquette, plus « requêtes »).
  `TreePanel.vue` et `ContractsSection.vue` reçoivent un en-tête de section commun.

### 6.5 Design

Jetons existants seulement (`--color-surface`, `--color-border`, `--color-chip`,
`--color-accent-soft`, `--color-accent`…) : pas de nouvelle couleur.

## 7. Impacts sur l'existant

- `docs/plan-workspaces-onglets.md` : O1 à O7 révisés par S1 à S6 (le reste inchangé).
- Tests Vitest des onglets réécrits pour le sélecteur (`tests/workspaces/switcher.spec.ts`) ;
  autres tests et parcours e2e adaptés (aide `tests/switcher.ts`).

## 8. Plan de tâches pour Claude Code

**Lot A - Sélecteur et icônes** (une pull request)

- [x] **T1** Ce plan.
- [x] **T2** `WorkspaceSwitcher` (S1 à S7) et mise en page (S8) ; Vitest au clavier.
- [x] **T3** `SectionIcon` et en-têtes de sections (I1, I2) ; Vitest.
- [x] **T4** Tests adaptés, e2e clair et sombre, `CLAUDE.md`, README.

## 9. Critères d'acceptation

- [x] `npm run check` passe ; e2e clair et sombre.
- [x] Plus de rangée d'onglets de workspaces ; changer, créer, renommer, colorer, supprimer, gérer
      et glisser un projet passent par le sélecteur, à la souris comme au clavier.
- [x] Les sections d'un projet déplié portent leur icône dans les deux thèmes.

## 10. Risques

| Risque                                      | Impact                    | Parade                                                    |
| ------------------------------------------- | ------------------------- | --------------------------------------------------------- |
| Workspaces moins visibles qu'en onglets     | Un clic de plus           | Ctrl+0…9 et le nom du workspace toujours affiché          |
| Raccourcis Ctrl+chiffre pris par la WebView | Zoom au lieu du workspace | `preventDefault` ; le zoom clavier de Tauri est désactivé |

## 11. Décisions

| ID  | Question                                                   | Décision                                                                                                                               |
| --- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | Où placer le sélecteur sans barre de titre personnalisée ? | En tête de la colonne de la barre latérale, à la hauteur de `.zone__bar` : même rendu que la maquette sans dessiner de barre de titre. |
| Q2  | Renommer dans le menu ou dans une fenêtre ?                | Fenêtre `NameDialog` : le menu se ferme, la saisie ne dépend pas du survol.                                                            |
| Q3  | Raccourcis                                                 | Ctrl+0 « Tous les projets », Ctrl+1…9 les neuf premiers workspaces, Ctrl+Maj+W la gestion.                                             |
| Q4  | Icône de « Requêtes »                                      | Une requête envoyée (flèche dans un cadre), dans le style des icônes de la maquette.                                                   |
