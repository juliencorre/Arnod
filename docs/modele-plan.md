# Plan - {Sujet du lot}

**Version** 0.1 · **Date** AAAA-MM-JJ · **Statut** Brouillon · **Base** version {x.y.z}

> **Mode d'emploi.** Copier ce fichier en `docs/plan-{sujet}.md`, remplir chaque section et
> retirer les indications entre accolades. Lister les questions ouvertes au § 11 et les trancher
> avant la première tâche. Pendant le travail : un lot = une pull request, une tâche cochée dès
> qu'elle est livrée, `CLAUDE.md` (« Travail en cours ») pointe vers ce plan. Modèle tiré de
> [`plan-projets-workspaces.md`](plan-projets-workspaces.md), à consulter comme exemple.

Plan d'implémentation de {cas d'usage}. Il part de la version {x.y.z} et respecte les règles de
[`CLAUDE.md`](../CLAUDE.md).

Source : {maquette, PRD, demande}. Les écrans sont capturés dans [`docs/maquettes/`](maquettes/).

## 1. Objectif

À la fin de ce plan, Arnod :

- {capacité visible par l'utilisateur} ;
- {capacité visible par l'utilisateur}.

## 2. Maquettes

| Écran | Capture                   | Dans ce plan           |
| ----- | ------------------------- | ---------------------- |
| {id}  | `maquettes/{fichier}.png` | {cas d'usage couverts} |

## 3. Modèle et règles

{Définitions des notions du domaine.}

Les règles vivent dans le cœur Rust, module pur, avec au moins un test par règle.

| ID  | Règle                                                  |
| --- | ------------------------------------------------------ |
| R1  | {règle métier, formulée pour être testée telle quelle} |

## 4. Cas d'usage

Priorités : **Must** (indispensable au lot), **Should** (attendu), **Could** (si le temps le
permet). Identifiants : un préfixe propre au domaine, distinct de R (règles), T (tâches) et
Q (questions).

| ID   | Cas d'usage | Écrans | Comportement attendu                                   | Priorité |
| ---- | ----------- | ------ | ------------------------------------------------------ | -------- |
| {X1} | {titre}     | {id}   | {déclencheur, résultat, messages d'erreur, annulation} | Must     |

## 5. Hors périmètre

- {ce qui est volontairement exclu, et dans quel lot il est prévu}.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

- Fichiers créés ou modifiés, avec un exemple de contenu.
- Version du format. Fichier d'une version plus ancienne : migration. Fichier d'une version plus
  récente : refus clair, sans écriture.
- Compatibilité : un fichier de projet circule par Git entre des postes qui n'ont pas forcément
  la même version d'Arnod.

### 6.2 Cœur Rust

{Modules, fonctions pures testables, nouvelles natures de `CommandErrorKind`, dépendances
ajoutées.}

### 6.3 Commandes IPC

| Commande | Paramètres | Réponse | Erreurs (`kind`) |
| -------- | ---------- | ------- | ---------------- |
| `{nom}`  | {…}        | {…}     | {…}              |

### 6.4 Frontend

{Stores, composants, navigation, notifications.}

### 6.5 Design

{Nouveaux jetons de couleur (thèmes clair et sombre), comportement au clavier.}

## 7. Impacts sur l'existant

| Fichier                               | Changement                                    |
| ------------------------------------- | --------------------------------------------- |
| `CLAUDE.md`                           | {règles, arborescence, travail en cours}      |
| `README.md`                           | {tableau des fonctions, captures}             |
| `src-tauri/capabilities/default.json` | {inchangé, ou permission justifiée}           |
| `e2e/`, `scripts/smoke-*`             | {parcours à ajouter ou adapter}               |
| Installeur                            | {mesure après ajout de dépendances ({ 15 Mo)} |

## 8. Plan de tâches pour Claude Code

Un lot = une pull request, titrée en Conventional Commits. `npm run check` passe avant chaque
commit ; chaque tâche est relue avant commit.

**Lot A - {nom}**

- [ ] **T1** {tâche livrable et vérifiable}.

## 9. Critères d'acceptation

- [ ] {comportement vérifiable, de préférence par un test e2e sur l'application réelle}.

## 10. Risques

| Risque | Impact | Parade |
| ------ | ------ | ------ |
| {…}    | {…}    | {…}    |

## 11. Décisions

Questions ouvertes, à trancher avant la première tâche, puis décision datée.

| ID  | Question | Décision   |
| --- | -------- | ---------- |
| Q1  | {…}      | À trancher |
