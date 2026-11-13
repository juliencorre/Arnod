# Plan - Lot G : démarrage et erreurs

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** version 0.0.2

Plan du lot G de la finalisation du MVP ([`plan-finalisation-mvp.md`](plan-finalisation-mvp.md),
vague 2) : une fenêtre prête ou une erreur expliquée, jamais d'écran « Chargement… ». Il respecte
les règles de [`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan d'ensemble.

Source : page 0.3 de la maquette, « Arnod - Démarrage et erreurs ».

## 1. Objectif

À la fin de ce plan, Arnod :

- n'apparaît qu'une fois prêt ; au-delà de 300 ms, il montre le squelette de l'écran attendu ;
- explique une erreur bloquante au démarrage : la cause en clair, ce qui est intact, deux issues
  avec leurs conséquences, la recommandée en premier ;
- réessaie seul un fichier verrouillé et dit où regarder ;
- donne des détails pour le support, repliés, à copier ou à joindre à une issue préremplie.

## 2. Maquettes

| Repère      | Dans ce plan                                  |
| ----------- | --------------------------------------------- |
| `0.3-a`     | DL1 : squelette du démarrage lent             |
| `0.3-b`     | ER1, ER2, ER3 : erreur expliquée, deux issues |
| `0.3-b bis` | VR1 : fichier verrouillé, essais automatiques |
| `0.3-b ter` | SU1, SU2 : détails pour le support, issue     |

## 3. Modèle et règles

Un **échec de démarrage** (`startup::Failure`) dit quel fichier (`arnod.json` ou `windows.json`), où,
et pourquoi : format plus récent, contenu illisible, verrou (accès refusé), dossier inaccessible.
Le cœur le décrit (`startup::describe`, pur) en un `StartupProblem` que l'interface affiche tel quel.

| ID  | Règle                                                                                                                                                                                                                                                                                                        |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | Un accès refusé (et, sous Windows, les codes 32 et 33 : fichier tenu par un autre programme) est un **verrou** ; toute autre erreur d'accès rend le dossier inaccessible.                                                                                                                                    |
| R2  | Toute erreur bloquante propose deux issues, la recommandée en premier : version plus récente → installer (recommandé) ou repartir d'un état vide ; illisible → repartir d'un état vide (recommandé) ou corriger puis réessayer ; verrou ou dossier → réessayer (recommandé) ou vérifier le dossier.          |
| R3  | Rien n'est écrit avant le choix de l'utilisateur. « Repartir d'un état vide » renomme le fichier (`arnod.json.format-3`, `.illisible`, puis `-2`, `-3`…), jamais ne le supprime ni ne l'écrase ; un verrou ne se met pas de côté. Le fichier des fenêtres d'un état bloqué n'est jamais réécrit à la sortie. |
| R4  | Un verrou ou un dossier inaccessible est réessayé seul trois fois (2, 4, 8 s) ; ensuite l'issue « Réessayer » dit que les essais sont épuisés.                                                                                                                                                               |
| R5  | Les détails pour le support donnent la nature, le fichier, l'erreur technique ou le format, la version d'Arnod, le système, la WebView et la date (UTC).                                                                                                                                                     |
| R6  | « Signaler le problème… » ouvre une issue GitHub préremplie dans le navigateur, à relire : rien n'est envoyé seul. Si le navigateur ne s'ouvre pas, l'adresse est copiée.                                                                                                                                    |
| R7  | `arnod.json` note la version d'Arnod qui l'a écrit (`writtenBy`) ; une version plus ancienne la cite dans l'erreur.                                                                                                                                                                                          |
| R8  | La fenêtre principale et les fenêtres détachées sont créées cachées, montrées quand l'interface a rendu son premier écran (`window_ready`), au plus tard après 300 ms.                                                                                                                                       |

## 4. Cas d'usage

| ID  | Cas d'usage                     | Écrans    | Comportement attendu                                                                            | Priorité |
| --- | ------------------------------- | --------- | ----------------------------------------------------------------------------------------------- | -------- |
| DL1 | Démarrage lent                  | 0.3-a     | Squelette (barre latérale, titre, cartes), « Ouverture de « … »… » une fois le workspace connu. | Must     |
| ER1 | `arnod.json` plus récent        | 0.3-b     | Cause, rien n'a été modifié, installer ou repartir d'un état vide ; fichier intact.             | Must     |
| ER2 | `arnod.json` illisible          | 0.3-b     | Plus de mise de côté silencieuse : l'utilisateur choisit ; le nom gardé est notifié après.      | Must     |
| ER3 | `windows.json` illisible/récent | 0.3-b     | Même écran ; repartir d'une disposition vide garde projets et workspaces.                       | Must     |
| VR1 | Fichier verrouillé              | 0.3-b bis | Décompte, essais automatiques, puis « Réessayer » ; « Ouvrir le dossier ».                      | Must     |
| SU1 | Détails pour le support         | 0.3-b ter | Repliés ; dépliés au clavier ; « Copier le détail ».                                            | Must     |
| SU2 | Signaler sur GitHub             | 0.3-b ter | Issue préremplie ouverte par le navigateur du système ; sinon adresse copiée et notifiée.       | Must     |

## 5. Hors périmètre

- Journal d'Arnod (`arnod.log`, ligne « Journal » de la maquette) : Arnod n'écrit pas de journal.
- Version de Windows détaillée (« Windows 11 ») : le cœur donne le système et l'architecture.

## 6. Architecture

- `src-tauri/src/startup.rs` (pur sauf `set_aside` et `open_external`) : classement, description,
  essais, détails, adresse de l'issue (encodage `percent-encoding`, déjà présent).
- `storage::load` renvoie `Result<Loaded, Failure>` sans rien renommer ; `AppState` garde l'échec
  (`failure`, `reload`, `set_warning`). `WindowsState` lit `windows.json` de la même façon.
- Ouvrir le navigateur ou l'explorateur : programme du système par chemin absolu (`xdg-open`,
  `open`, `explorer.exe`), sans plugin Tauri ni permission WebView (R8 du plan d'ensemble).
- `tauri.conf.json` : `visible: false` ; `windows::show_later` montre une fenêtre encore cachée
  après 300 ms ; les fenêtres détachées ne sont recréées qu'une fois l'erreur réglée.

### Commandes IPC

| Commande              | Paramètres              | Réponse                  | Erreurs                            |
| --------------------- | ----------------------- | ------------------------ | ---------------------------------- |
| `startup_status`      | —                       | `StartupProblem \| null` | —                                  |
| `startup_retry`       | `automatic: boolean`    | `StartupProblem \| null` | —                                  |
| `startup_start_empty` | —                       | `StartupProblem \| null` | `notFound`, `invalidRequest`, `io` |
| `startup_open`        | `target: StartupTarget` | `null`                   | `notFound`, `io`                   |
| `startup_quit`        | —                       | `null`                   | —                                  |
| `window_ready`        | —                       | `null`                   | `io`                               |

Aucune nature ajoutée à `CommandErrorKind` : `newerVersion`, `corruptState` et `io` suffisent aux
commandes ; la nature affichée (`ProblemNature`, avec `locked` et `inaccessible`) est propre à
l'écran.

Frontend : store `startup` (décompte, issues, copie), `StartupProblemScreen.vue`,
`StartupSkeleton.vue` ; `App.vue` vérifie `startup_status` avant de charger, reprend le démarrage
une fois l'erreur réglée et appelle `window_ready` après le premier rendu.

## 7. Vérification

- `cargo test` : `startup::tests` (R1 à R6), `storage`, `state`, `windows` (fichiers intacts).
- Vitest : `tests/startup/startup.spec.ts` (faux cœur `tests/fakeStartup.ts`), clavier compris.
- Aperçu : scénarios `0.3-a`, `0.3-b`, `0.3-b-bis`, `0.3-b-ter`, clair et sombre.
- e2e : `describe("Lot G - Démarrage et erreurs (0.3)")`, `arnod.json` illisible puis plus récent.

## 8. Tâches

- [x] **T1** — Cœur : `startup.rs`, `storage`, `state`, `windows`, commandes.
- [x] **T2** — Interface : store, écran d'erreur, squelette, `App.vue`, fenêtre cachée.
- [x] **T3** — Tests, aperçu, e2e, `maquette-etat.yml`, `CLAUDE.md`.

## 9. Écarts gardés avec la maquette

- Pas de ligne « Journal » dans les détails (pas de journal écrit) ; système sans numéro de
  version (« Windows », pas « Windows 11 »).
- Pas de gras sur la version dans la cause (texte simple venu du cœur).
- Démarrage lent : la légende « Ouverture de « … »… » n'apparaît que si le catalogue est lu ;
  avant, le squelette seul.
