# Plan - Fin du premier usage : run, statuts, vérifications

**Version** 1.0 · **Date** 2026-11-12 · **Statut** Livré · **Base** version 0.0.2

Plan d'implémentation des écarts restants du premier usage
([`plan-premier-usage.md`](plan-premier-usage.md), tâches T1, T18, T25, T27 et T29) : écran du run
complet, résultat de la dernière exécution dans la barre latérale, échecs des vérifications
ignorables ou retirables, tests e2e du parcours. Il part de la version 0.0.2 et respecte les règles
de [`CLAUDE.md`](../CLAUDE.md).

Source : maquette « Arnod - Premier usage », écrans 1e et 1f, capturés dans
[`docs/maquettes/`](maquettes/). La disposition de ces captures (sélecteur d'environnement dans la
barre de titre, onglets « Requêtes / Runs ») est remplacée par celle des maquettes 6a à 6c ; ce
plan en reprend le contenu.

## 1. Objectif

À la fin de ce plan, Arnod :

- affiche le run comme la maquette 1f : « 56 réussis, 4 en échec », ligne
  `run #3 · Recette · 8 en même temps · 4,1 s`, barre verte et rouge, onglets « Échecs », « Tous »,
  « Lents » avec leurs compteurs, détail de chaque échec (vérification, attendu, reçu) et panneau
  « Résultat par dossier » qui filtre la liste ;
- suit l'avancement d'un run par les événements du cœur, sans interrogation périodique ;
- montre dans la barre latérale le résultat de la dernière exécution de chaque requête (✓ ou ✕)
  et, par dossier ou ressource de contrat, « réussies / exécutées », gardés d'une session à
  l'autre ;
- détaille les vérifications en échec sous la réponse, avec « Ignorer pour ce run » et
  « Retirer la vérification » (maquette 1e) ;
- couvre ce parcours par un test e2e sur l'application réelle, contre un serveur HTTP local.

## 2. Maquettes

| Écran | Capture                                                                     | Dans ce plan                                                                          |
| ----- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| 1e    | [Requête, assertions, réponse](maquettes/1e-requete-assertions-reponse.png) | Pastilles et compteurs de l'arborescence ; encadré d'échec, ignorer, retirer          |
| 1f    | [Run](maquettes/1f-run.png)                                                 | Tout l'écran ; « workers » devient « en même temps », « service » devient dossier     |
| 6a    | [Barre latérale du projet](maquettes/6a-barre-laterale-projet.png)          | Disposition de référence : les pastilles s'ajoutent aux lignes, sans autre changement |

## 3. Modèle et règles

- **Statut d'une requête** : résultat de sa dernière exécution (envoi seul ou ligne d'un run) :
  réussie ou en échec, avec sa date.
- **Groupe d'un run** : dossier de premier niveau sous le dossier exécuté (« service » de la
  maquette).
- **Vérification ignorée** : vérification en échec dont l'échec ne compte pas dans le résultat
  affiché ; elle reste dans la requête.

Les règles vivent dans le cœur Rust (`workbench.rs`, `statuses.rs`, `results.rs`), testées une à
une.

| ID  | Règle                                                                                                                                                                                                                                                                                                                                    |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un run porte un numéro propre au projet : celui du dernier run gardé, plus un (1 pour le premier). Il porte aussi le nombre de requêtes envoyées en même temps et, une fois terminé, sa date de fin.                                                                                                                                     |
| R2  | Chaque requête d'un run appartient au groupe du premier segment de son chemin sous le dossier exécuté ; une requête placée directement dans ce dossier appartient au groupe du dossier lui-même (« Projet » à la racine). Les groupes suivent l'ordre de l'arborescence ; chacun compte réussies, en échec (échec ou erreur) et total.   |
| R3  | Chaque changement d'état d'une requête d'un run est diffusé à toutes les fenêtres (`arnod://run`) avec le run entier et une révision croissante ; une fenêtre n'applique jamais une révision antérieure à celle qu'elle montre pour ce run.                                                                                              |
| R4  | Le statut d'une requête est mis à jour à la fin de chaque envoi (seul ou dans un run) : réussie, ou en échec (échec des vérifications, erreur réseau, variable inconnue). Un envoi annulé ne change rien.                                                                                                                                |
| R5  | Les statuts sont gardés par projet dans `projects/<id>/statuses.json` (données de l'application, jamais dans le projet). Ils suivent une requête ou un dossier renommé ou déplacé, et disparaissent avec une requête supprimée. Un fichier illisible vaut « aucun statut » ; un fichier d'une version plus récente n'est jamais réécrit. |
| R6  | Un dossier ou une ressource de contrat affiche « réussies / exécutées » parmi ses requêtes, sous-dossiers compris, dès qu'une au moins a un statut ; il est en échec si une au moins l'est.                                                                                                                                              |
| R7  | Ignorer une vérification en échec ne vaut que pour le résultat affiché (dernier envoi de la requête, ou sa ligne dans un run) : le résultat est recalculé sans elle (R11 du premier usage) et le statut de la requête suit ; elle est de nouveau évaluée au prochain envoi.                                                              |
| R8  | Retirer une vérification l'enlève de la requête (enregistrée comme toute modification) et l'ignore dans le résultat affiché. La conformité au contrat ne se retire pas : elle s'ignore seulement.                                                                                                                                        |

## 4. Cas d'usage

Préfixes : **RU** run, **ST** statuts, **VT** vérifications, **QA** qualité.

| ID  | Cas d'usage                   | Écrans | Comportement attendu                                                                                                                                                                                   | Priorité |
| --- | ----------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| RU1 | Résumé du run                 | 1f     | « 56 réussis, 4 en échec » (« Run en cours · 23 / 60 » pendant l'exécution), ligne `run #3 · Recette · 8 en même temps · 4,1 s`, barre verte, rouge et grise (en attente).                             | Must     |
| RU2 | Échecs, Tous, Lents           | 1f     | Onglets avec compteurs (`role="tablist"`, flèches) ; « Échecs » par défaut quand il y en a, sinon « Tous ».                                                                                            | Must     |
| RU3 | Détail d'un échec             | 1f     | ✕, méthode, nom (ouvre la requête), « — cible » de la première vérification en échec, puis une ligne par vérification en échec : message (« attendu number, reçu "129.90" ») ; erreur réseau en clair. | Must     |
| RU4 | Résultat par dossier          | 1f     | Panneau : par groupe (R2), barre et « 15/16 » ; un clic filtre la liste sur ce groupe, un second retire le filtre. Choix « Grouper par dossier » ou « À plat ».                                        | Should   |
| RU5 | Avancement par événements     | -      | Le run se met à jour par `arnod://run` (R3) dans toutes les fenêtres ; plus d'interrogation de `run_status`.                                                                                           | Should   |
| RU6 | Run vide                      | -      | « Aucune requête à exécuter dans ce dossier. »                                                                                                                                                         | Must     |
| ST1 | Statut d'une requête          | 1e     | ✓ vert ou ✕ rouge en fin de ligne de requête, dans le référentiel de tests comme sous un contrat ; titre « Réussie le … » / « En échec le … ».                                                         | Must     |
| ST2 | Compteur d'un dossier         | 1e     | « 15/16 » à la place du nombre de requêtes (R6), en rouge s'il y a un échec ; titre « 15 réussies sur 16 exécutées (18 requêtes) ». Sans statut : le nombre de requêtes, comme avant.                  | Must     |
| ST3 | Suivi en direct               | -      | Chaque envoi et chaque ligne de run met à jour la barre latérale de toutes les fenêtres (`arnod://statuts`).                                                                                           | Must     |
| ST4 | Gardé au redémarrage          | -      | Les statuts sont relus au redémarrage (R5).                                                                                                                                                            | Should   |
| VT1 | Encadré d'échec               | 1e     | Sous la réponse : une ligne par vérification en échec, « ✕ headers.Location — en-tête absent de la réponse », avec « Ignorer pour ce run » et « Retirer la vérification ».                             | Must     |
| VT2 | Ignorer pour ce run           | 1e, 1f | Depuis l'encadré ou une ligne du run (R7) : la vérification passe en « ignorée », le verdict et le statut sont recalculés, dans toutes les fenêtres.                                                   | Should   |
| VT3 | Retirer la vérification       | 1e     | Depuis l'encadré (R8) : la vérification disparaît de l'onglet Tests et du fichier ; le résultat est recalculé.                                                                                         | Should   |
| VT4 | Origine affichée              | 1e     | « proposée », « par défaut » ou « manuelle » en bout de ligne de l'onglet Tests (origines de session, Q2).                                                                                             | Could    |
| QA1 | Parcours e2e du premier usage | -      | Créer un dossier et deux requêtes, envoyer, proposer les vérifications, ajouter un échec, l'ignorer, exécuter le dossier, lire le run, retrouver les statuts au redémarrage.                           | Must     |

## 5. Hors périmètre

- Copie `curl` et enregistrement du corps (T28), glisser-déposer de l'arborescence (T22) : non
  demandés par les écrans 1e et 1f.
- Historique des runs : seul le dernier run de chaque projet reste gardé (R14 du premier usage).
- Remplacement des captures 1c à 1f par celles de la maquette actuelle : la maquette en ligne ne se
  rend pas depuis ce dépôt ; les captures du README viennent des tests e2e.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

Rien ne change dans le projet. Données locales, dans `app_local_data_dir()` :

- `projects/<id>/last-run.json` : le run gagne `number`, `concurrency`, `finishedAt`, `revision`,
  `groups`, et chaque résultat de vérification `target` et `ignored`. Un fichier ancien se relit
  (valeurs par défaut, groupes recalculés).
- `projects/<id>/statuses.json` (nouveau, R5) :

```json
{ "version": 1, "requests": { "commandes/creer.yml": { "passed": false, "at": 1794432959000 } } }
```

### 6.2 Cœur Rust

- `statuses.rs` (pur) : `StatusBook` (enregistrer, renommer un chemin ou un dossier, retirer),
  lecture et écriture de `statuses.json` (version, refus d'une version plus récente).
- `workbench.rs` : numéro, durée, révision et groupes du run (`RunView::refresh`) ; statuts
  enregistrés à la fin de chaque ligne de run ; `Notifier` (trait) prévenu de chaque changement
  de run et de statuts, branché sur les événements par `lib.rs` (un faux notificateur dans les
  tests) ; `ignore_in_run`.
- `request/assertions.rs` : `AssertionResult` gagne `target` et `ignored` ; `request_passed`
  ne compte pas les vérifications ignorées.
- `results.rs` : `ResultBook::ignore` recalcule le dernier résultat d'une requête.
- Aucune nouvelle nature de `CommandErrorKind`, aucune dépendance.

### 6.3 Commandes IPC

| Commande              | Paramètres                      | Réponse           | Erreurs (`kind`) |
| --------------------- | ------------------------------- | ----------------- | ---------------- |
| `request_statuses`    | `{ project }`                   | `RequestStatuses` | `notFound`       |
| `result_ignore_check` | `{ project, path, check }`      | `RequestResult`   | `notFound`       |
| `run_ignore_check`    | `{ project, run, path, check }` | `RunView`         | `notFound`       |

Événements : `arnod://run` (existant) porte aussi l'avancement, avec `origin` vide quand il vient du
cœur ; `arnod://statuts` (nouveau) porte `{ projectId, statuses }`. Un envoi, un run, un
renommage, un déplacement ou une suppression le déclenche.

### 6.4 Frontend

- Store `workbench` : `statuses[projectId]`, chargés avec le projet, remplacés par
  `arnod://statuts` ; runs appliqués par révision (R3), sans `setTimeout`.
- `lib/status.ts` : agrégation d'affichage des statuts d'un dossier ou d'une ressource (R6).
- `RunPanel.vue` réécrit (RU1 à RU6) ; `TreeRow.vue`, `LinkedRequestRow.vue` et
  `ContractsSection.vue` affichent statuts et compteurs ; `ResponsePanel.vue` gagne l'encadré
  d'échec (VT1 à VT3) ; `AssertionsEditor.vue` affiche « manuelle » (VT4).

### 6.5 Design

- Couleurs existantes uniquement : `--color-success`, `--color-danger`, `--color-border`,
  `--color-success-soft`, `--color-danger-soft`. Une couleur ne porte jamais seule le résultat :
  ✓ et ✕ accompagnent le vert et le rouge.
- Onglets du run en `role="tablist"` (flèches gauche et droite) ; groupes en boutons
  `aria-pressed` ; avancement annoncé par `role="status"`.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                     |
| ------------------------------------- | ------------------------------------------------------------------------------ |
| `CLAUDE.md`                           | Travail en cours, arborescence (`statuses.rs`, `lib/status.ts`), règles du run |
| `docs/plan-premier-usage.md`          | T1, T18, T25, T27, T29 cochés ou annotés ; écarts mis à jour                   |
| `src-tauri/capabilities/default.json` | Inchangé                                                                       |
| `e2e/`                                | Parcours « Premier usage » contre un serveur HTTP local                        |
| Installeur                            | Aucune dépendance ajoutée                                                      |

## 8. Plan de tâches pour Claude Code

Une seule pull request (une seule branche de travail) : les lots A à C partagent le cœur
(notificateur, statuts) et sont livrés dans un même commit, le lot D dans les suivants.
`npm run check` passe avant chaque commit ; chaque tâche est relue avant commit.

**Lot A - Écran du run**

- [x] **T1** Cœur : numéro, concurrence, fin, révision et groupes du run (R1, R2) ; `target` des
      résultats de vérification ; tests, dont la relecture d'un `last-run.json` ancien.
- [x] **T2** Cœur : notificateur et avancement diffusé par `arnod://run` (R3) ; store sans
      interrogation ; tests.
- [x] **T3** `RunPanel.vue` : résumé, barre, onglets, détail des échecs, résultat par dossier,
      run vide (RU1 à RU6) ; tests Vitest au clavier.

**Lot B - Statuts dans la barre latérale**

- [x] **T4** `statuses.rs` et enregistrement à chaque envoi et ligne de run, renommage,
      déplacement, suppression (R4, R5) ; commande `request_statuses`, événement
      `arnod://statuts` ; tests.
- [x] **T5** Interface : ✓ / ✕ des requêtes, « réussies / exécutées » des dossiers et ressources
      (ST1 à ST4, R6) ; tests Vitest.

**Lot C - Vérifications en échec**

- [x] **T6** Cœur : vérification ignorée (R7) dans le dernier résultat et dans un run ; commandes
      `result_ignore_check` et `run_ignore_check` ; tests.
- [x] **T7** Interface : encadré d'échec, « Ignorer pour ce run », « Retirer la vérification »,
      origine « manuelle » (VT1 à VT4) ; tests Vitest.

**Lot D - Qualité et documentation**

- [x] **T8** Test e2e du parcours QA1 contre un serveur HTTP local.
- [x] **T9** `CLAUDE.md`, plan du premier usage (T1, T18, T25, T27, T29), relecture d'ensemble.

## 9. Critères d'acceptation

- [x] Un dossier de requêtes dont une échoue s'exécute : « N réussis, 1 en échec », la ligne
      d'échec donne la vérification, l'attendu et le reçu ; « Lents » et « Tous » comptent juste.
- [x] Le panneau « Résultat par dossier » montre un groupe par sous-dossier ; un clic filtre la
      liste.
- [x] Pendant un run, la barre latérale de toutes les fenêtres se met à jour ligne par ligne.
- [x] Après redémarrage, les ✓ / ✕ et les compteurs des dossiers sont toujours là.
- [x] Ignorer l'échec d'une vérification rend la requête réussie sans modifier son fichier ;
      la retirer la retire du fichier.
- [x] `cargo test`, `npm run check` et le parcours e2e passent.

## 10. Risques

| Risque                                                   | Impact                               | Parade                                                            |
| -------------------------------------------------------- | ------------------------------------ | ----------------------------------------------------------------- |
| Événements reçus avant la réponse de `run_start`         | Run qui revient en arrière à l'écran | Révision croissante, appliquée seulement si plus récente (R3)     |
| Écritures concurrentes de `statuses.json` pendant un run | Statuts perdus                       | Un seul verrou dans l'établi, écriture atomique                   |
| Beaucoup d'événements pour un gros run                   | Interface ralentie                   | Un événement par changement d'état d'une ligne (deux par requête) |

## 11. Décisions

| ID  | Question                                                           | Décision (2026-11-12)                                                                                                                                                                                                      |
| --- | ------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | Où garder les statuts : dans le projet ou sur le poste ?           | Sur le poste (`statuses.json`) : un résultat dépend de l'environnement et du poste, il ne se versionne pas.                                                                                                                |
| Q2  | Écrire l'origine des vérifications pour afficher « manuelle » ?    | Non (aucun champ inventé dans OpenCollection) : l'origine reste de session ; relue d'un fichier, une vérification est « manuelle », sauf la durée par défaut (`responseTime` inférieur à 500 ms), reconnue « par défaut ». |
| Q3  | Compteur d'un dossier : réussies / total ou réussies / exécutées ? | Réussies / exécutées : une requête jamais exécutée n'est pas un échec ; le total figure dans le titre.                                                                                                                     |
| Q4  | Que veut dire « Ignorer pour ce run » hors d'un run ?              | Le dernier résultat affiché de la requête (R7) ; le prochain envoi la réévalue.                                                                                                                                            |
