# Plan - Barre latérale du projet

**Version** 0.1 · **Date** 2026-11-10 · **Statut** Livré · **Base** version 0.0.2

Plan d'implémentation de l'option a de la maquette « Arnod - Barre latérale du projet » :
environnement et actions d'un projet dans la barre latérale. Il part de la configuration d'un
endpoint livrée et respecte les règles de [`CLAUDE.md`](../CLAUDE.md).

Source : « le développement de la partie environnement dans le menu gauche n'est pas ergonomique.
Propose une meilleure solution pour les actions et le rendu », puis « ok pour a mais l'alignement
gauche ne me semble pas parfait ». Écrans capturés dans [`docs/maquettes/`](maquettes/).

## 1. Objectif

Sous chaque projet ouvert, la barre latérale empilait trois lignes d'outils : une liste
déroulante sans libellé suivie de « Gérer », puis « + Requête » et « + Dossier », puis « Tout
exécuter ». Elles repoussaient l'arborescence, se répétaient pour chaque projet et
ressemblaient à des liens. À la fin de ce plan :

- la ligne du projet porte ses actions (▶ exécuter, + ajouter, ⋯ autres), visibles au survol et
  au focus, et toujours sur le projet ouvert ;
- une pastille d'environnement, sous le nom, ouvre le menu des environnements (couleur, nom,
  adresse, protégé, « Gérer les environnements… », « + Nouvel environnement ») ; un projet
  replié la montre à droite de son nom ;
- un projet vide ou sans contrat propose une ligne d'ajout à la place d'un texte d'aide ;
- toutes les lignes suivent une même grille.

## 2. Maquettes

| Écran | Capture                                     | Dans ce plan                     |
| ----- | ------------------------------------------- | -------------------------------- |
| a     | `maquettes/6a-barre-laterale-projet.png`    | B1, B2, B5 (ligne, pastille)     |
| a bis | `maquettes/6b-menu-environnements.png`      | B2 (menu des environnements)     |
| a ter | `maquettes/6c-projet-vide-menu-ajouter.png` | B3, B4 (menu « + », projet vide) |

## 3. Grille

Chaque ligne commence au même retrait ; une colonne de tête de largeur fixe porte le chevron et
la pastille d'un projet, le chevron et le dossier d'un dossier, la méthode d'une requête, « API »
d'un contrat ou le « + » d'une ligne d'ajout ; le libellé commence juste après. Chaque profondeur
décale de la largeur d'un chevron et de son espace.

| Variable        | Valeur    | Effet                                                                               |
| --------------- | --------- | ----------------------------------------------------------------------------------- |
| `--tree-pad`    | 0,375 rem | Retrait de toute ligne ; « PROJETS » et les chevrons y sont                         |
| `--tree-indent` | 1,25 rem  | Une profondeur : la tête d'un enfant sous la pastille du parent                     |
| `--tree-lead`   | 2,5 rem   | Colonne de tête                                                                     |
| `--tree-gap`    | 0,5 rem   | Espace avant le libellé ; la pastille d'environnement s'aligne sur le nom du projet |

## 4. Cas d'usage

| ID  | Cas d'usage              | Écrans   | Comportement attendu                                                                                                                                         | Priorité |
| --- | ------------------------ | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| B1  | Ligne du projet          | a        | Chevron, pastille, nom ; ▶, +, ⋯ au survol et au focus (toujours sur le projet ouvert). ▶ dit « Exécuter 3 requêtes sur Local ».                             | Must     |
| B2  | Pastille d'environnement | a, a bis | Sous le nom ; menu des environnements (motif menu, choix exclusif), adresse de chacun, « protégé », gestion et création.                                     | Must     |
| B3  | Menu « + »               | a ter    | Nouvelle requête (Ctrl+N), Nouveau dossier, Ajouter un contrat ▸ (quatre façons), Nouvel environnement….                                                     | Must     |
| B4  | Projet vide              | a ter    | « + Nouvelle requête » et « + Ajouter un contrat » à la place des textes d'aide.                                                                             | Should   |
| B5  | Menu ⋯ et projet replié  | a        | Ouvrir le projet, Gérer les environnements… (Ctrl+Maj+E), et dans un workspace : afficher dans « Tous les projets », retirer. Replié : la pastille à droite. | Should   |
| B6  | Raccourcis               | -        | Ctrl+N et Ctrl+Maj+E agissent sur le projet sélectionné : celui de l'onglet actif, sinon le projet ouvert, sinon le premier déplié.                          | Should   |

## 5. Hors périmètre

- « Aucun environnement » : un projet a toujours un environnement actif (premier usage).
- Variantes b (barre d'outils sous le nom) et c (environnement près d'« Envoyer ») de la maquette.

## 6. Architecture

Aucun changement du cœur : la pastille d'un projet replié lit ses environnements par la commande
existante `environments_load` (`workbench.peekEnvironments`).

- `layout/SidebarProject.vue` (nouveau) : un projet de la barre latérale ; il coordonne
  `EnvironmentChip`, `TreePanel` et `ContractsSection` (méthodes exposées).
- `workbench/EnvironmentChip.vue` remplace `EnvironmentBar.vue` ; `EnvironmentsDialog` s'ouvre sur un
  nouvel environnement (`startNew`).
- `ui/BaseMenu.vue` : entrées `radio` (`menuitemradio`), `swatch`, `tag`, `detail` ; le menu prend
  la largeur de son contenu.
- `AppSidebar.vue` : la grille (`--tree-*`) et les raccourcis.

## 7. Plan de tâches

- [x] **T1** Maquette (options 0, a, a bis, a ter, b, c), grille d'alignement, plan.
- [x] **T2** Pastille et menu des environnements ; `BaseMenu` (radio, pastille, mention, détail).
- [x] **T3** Ligne du projet et ses menus ; lignes d'ajout ; raccourcis.
- [x] **T4** Grille des lignes (projet, arborescence, contrats).
- [x] **T5** Vitest (clavier compris), e2e ; README, `CLAUDE.md`.

## 8. Décisions

| ID  | Question                         | Décision (2026-11-10)                                                                                                                                                     |
| --- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | Actions d'un projet replié ?     | Non : sa pastille d'environnement suffit ; le déplier montre ses actions.                                                                                                 |
| Q2  | Actions visibles en permanence ? | Sur le projet ouvert ; ailleurs au survol et au focus, toujours dans l'ordre de tabulation. Cachées, elles ne prennent pas de place : le nom n'est pas tronqué pour rien. |
