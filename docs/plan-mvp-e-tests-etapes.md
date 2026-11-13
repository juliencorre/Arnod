# Plan - MVP, lot E : tests et étapes

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** version 0.0.2 (Develop @e10ba9a)

Plan du lot E de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : le dossier devient un
test, une valeur de la réponse devient une variable, une valeur dynamique est tirée à chaque envoi.
Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan
d'ensemble (citées ici « R1 commun »…).

Source : maquette au 13 novembre 2026, pages 4.2 « Tests et étapes » (`4.2-a`, `4.2-b`,
`4.2-b bis`), 5.1 « Exécution » (`5.1-h`), 4.1 « Référentiel de tests » (`4.1-b`) et l'encart de
0.2 « Modèle et vocabulaire ».

## 1. Objectif

À la fin de ce plan, Arnod :

- exécute un **test** (dossier d'étapes) dans l'ordre, une étape à la fois, s'arrête à la première
  étape en échec et donne son verdict ; un dossier de rangement exécute ses tests ;
- range une valeur de la réponse dans une variable (**extraction**), sans script, et la passe aux
  étapes suivantes ;
- tire les **valeurs dynamiques** de Bruno (`{{$randomUUID}}`…) à chaque envoi, les montre dans
  « Envoyé » et sait les garder dans une variable ;
- crée un test avec son étape 1 (« Nouveau test », « + Étape ») et signale une requête modifiée
  depuis son dernier envoi, avec « Revenir à la version envoyée ».

## 2. Maquettes

| Repère      | Écran                                                      | Dans ce plan |
| ----------- | ---------------------------------------------------------- | ------------ |
| `4.2-a`     | Test ouvert : étapes numérotées, ▶, barre latérale         | TE1 à TE5    |
| `4.2-b`     | Écart : « modifié, pas encore envoyé », revenir            | MV1, MV2     |
| `4.2-b bis` | Onglet « Variables » : extractions                         | EX1 à EX5    |
| `5.1-h`     | Valeurs dynamiques tirées, « Envoyé », « Garder »          | VD1 à VD4    |
| `4.1-b`     | Nouveau test : dossier, étape 1 ouverte, « + Étape »       | NT1, NT2     |
| `0.2`       | Encart « Modèle et vocabulaire » : le dossier devient test | TE1          |

## 3. Modèle et règles

- **Test** : dossier qui contient au moins une requête et aucun sous-dossier (R3 commun). Ses
  requêtes sont ses **étapes**, dans l'ordre du dossier : `info.seq` (champ OpenCollection, comme
  Bruno), puis le nom. Un dossier qui contient des dossiers range des tests.
- **Extraction** : action OpenCollection `set-variable` en phase `after-response` (R2 commun), dans
  `runtime.actions` de la requête : `selector.expression` (`res.body.id`, `res.headers["location"]`,
  `res.status`, ou `req.body.ref` pour une valeur envoyée), `selector.method: jsonq`,
  `variable.name`, `variable.scope` (`runtime` à la création, gardé sinon).
- **Variable d'exécution** : valeur extraite, gardée en mémoire par le cœur ; jamais écrite dans un
  environnement ni dans le projet.
- **Valeur dynamique** : `{{$nom}}` (R1 commun), tirée par le cœur à chaque occurrence.

| ID  | Règle                                                                                                                                                                                         |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un dossier est un test s'il contient une requête et aucun sous-dossier ; ses étapes suivent `info.seq` puis le nom (dossiers sans `seq` : ordre alphabétique, inchangé).                      |
| R2  | Un run exécute les étapes d'un test une à une, dans l'ordre ; les tests et les requêtes hors test restent concurrents entre eux (8 au plus).                                                  |
| R3  | La première étape en échec ou en erreur arrête son test : les suivantes sont « non jouées » (`skipped`), ne changent pas leur statut et ne comptent ni comme réussies ni comme échecs.        |
| R4  | Verdict d'un test (`RunView.tests`) : réussi si toutes ses étapes réussissent, en échec dès qu'une étape échoue (avec le numéro de l'étape), annulé, en cours ou en attente sinon.            |
| R5  | Les valeurs extraites d'une étape sont lues par les étapes suivantes du même test ; elles priment sur l'environnement. Un run part sans variable d'exécution.                                 |
| R6  | Un envoi seul extrait aussi : la valeur est gardée pour le projet pendant la session (envois suivants), comme celles d'un run ; jamais écrite dans un environnement.                          |
| R7  | Une extraction lit le corps (chemin JSON), un en-tête ou le statut de la réponse, ou le corps et les en-têtes envoyés (`request.`) ; une valeur absente est signalée et ne crée rien.         |
| R8  | Lecture et écriture des actions : les extractions sont relues et réécrites ; les autres actions (phase `before-request`) sont conservées telles quelles, et seules elles restent `preserved`. |
| R9  | `{{$nom}}` connu est tiré à chaque occurrence (une valeur par occurrence) ; inconnu, il reste tel quel ; jamais cherché dans l'environnement ni signalé inconnu par le cœur.                  |
| R10 | Une valeur tirée qui forme seule une feuille du corps JSON ou un en-tête envoyé a une source (`request.body.ref`) : « Garder » en fait une extraction de la valeur envoyée.                   |
| R11 | « Nouveau test » crée un dossier « Nouveau test » (nom libre suivant) et son étape « Étape 1 » (`seq: 1`) ; « + Étape » ajoute « Étape N » après la dernière (`seq` posé sur les étapes).     |
| R12 | Le cœur garde la requête envoyée avec son résultat (`RequestResult.request`) ; l'éditeur la compare au brouillon pour « modifié, pas encore envoyé » et la reprend pour revenir.              |

Noms des valeurs dynamiques (ceux de Bruno, plus `$date`) : `$guid`, `$randomUUID`, `$timestamp`,
`$isoTimestamp`, `$date` (`$date:+3d`, `-1d`, `+2h`), `$randomInt`, `$randomFloat`,
`$randomBoolean`, `$randomAlphaNumeric`, `$randomHexColor`, `$randomIP`, `$randomIPV6`,
`$randomMACAddress`, `$randomPassword`, `$randomFirstName`, `$randomLastName`, `$randomFullName`,
`$randomUserName`, `$randomEmail`, `$randomPhoneNumber`, `$randomCity`, `$randomCountry`,
`$randomCountryCode`, `$randomDomainName`, `$randomUrl`, `$randomWord`, `$randomWords`,
`$randomSemver`, `$randomPrice`.

## 4. Cas d'usage

| ID  | Cas d'usage                  | Écrans     | Comportement attendu                                                                                                 | Priorité |
| --- | ---------------------------- | ---------- | -------------------------------------------------------------------------------------------------------------------- | -------- |
| TE1 | Voir un test                 | 4.2-a, 0.2 | Barre latérale : fiole, étapes numérotées, « réussies / exécutées » ; un clic ouvre l'onglet du test.                | Must     |
| TE2 | Onglet du test               | 4.2-a      | Chemin, nombre d'étapes, dernier résultat ; étapes numérotées (méthode, nom, chemin, extractions, vérifications, ✓). | Must     |
| TE3 | Exécuter un test             | 4.2-a      | ▶ joue les étapes dans l'ordre (R2), s'arrête au premier échec (R3) ; le run donne le verdict (R4).                  | Must     |
| TE4 | Exécuter un dossier          | 1.4, 4.2-a | Ses tests s'exécutent, chacun avec son verdict, agrégés dans « Résultat par test ».                                  | Must     |
| TE5 | Relancer les échecs          | 1.4        | Un test en échec est rejoué en entier (ses valeurs extraites sont recalculées).                                      | Should   |
| EX1 | Onglet « Variables »         | 4.2-b bis  | Variable, vient de, dernière valeur, portée ; « + Variable », retrait, activation.                                   | Must     |
| EX2 | Enregistrer une extraction   | 4.2-b bis  | Action `set-variable` écrite, le reste du fichier conservé (R8).                                                     | Must     |
| EX3 | Extraire à l'envoi           | 4.2-b bis  | Valeur extraite montrée dans « Envoyé » et dans l'onglet ; gardée pour la session (R6).                              | Must     |
| EX4 | Passer à l'étape suivante    | 4.2-a      | `{{client.id}}` d'une étape suivante prend la valeur extraite (R5).                                                  | Must     |
| EX5 | Utilisée par                 | 4.2-b bis  | Requêtes du projet qui lisent une variable de la requête.                                                            | Should   |
| VD1 | Tirer une valeur dynamique   | 5.1-h      | Adresse, paramètres, en-têtes, authentification, corps (R9).                                                         | Must     |
| VD2 | « Envoyé »                   | 5.1-h      | Méthode, adresse, en-têtes et corps envoyés ; valeurs tirées ; variables utilisées ; valeurs extraites.              | Must     |
| VD3 | « Garder »                   | 5.1-h      | Crée l'extraction de la valeur envoyée (R10) ; la ligne indique « gardée : nom ».                                    | Must     |
| NT1 | Nouveau test                 | 4.1-b      | Menu « + » : dossier et étape 1 créés, étape ouverte, curseur dans l'adresse.                                        | Must     |
| NT2 | « + Étape »                  | 4.1-b      | Dans l'onglet du test, la barre de l'étape et le menu du test : étape suivante créée et ouverte.                     | Must     |
| MV1 | Modifié, pas encore envoyé   | 4.2-b      | Après un envoi, toute modification affiche « modifié, pas encore envoyé ».                                           | Must     |
| MV2 | Revenir à la version envoyée | 4.2-b      | Le brouillon reprend la requête envoyée (enregistrement automatique).                                                | Must     |

## 5. Hors périmètre

- Réglage « Continuer » si une étape échoue (V4, décision Q2 du plan d'ensemble).
- Clic sur une valeur dans le corps de la réponse pour l'extraire (`4.2-b bis`) : à l'intégration de la vague 1, « Extraire dans une variable » s'ajoute au filtre JSONPath du lot A (`ResponseFilter.vue`) ; le clic direct dans le corps reste à faire.
- Liste des valeurs dynamiques à la frappe de `{{$` et soulignement d'un nom inconnu (lot B).
- Glisser une requête sur la liste des étapes (`4.1-f`, lot J), campagnes (6.3), réglages du test.
- Renommage général « requête » → « étape » (lot K, R4 commun).

## 6. Architecture

### 6.1 Formats sur disque

```yaml
info:
  name: Créer le client
  type: http
  seq: 1
runtime:
  actions:
    - type: set-variable
      phase: after-response
      selector: { expression: res.body.id, method: jsonq }
      variable: { name: client.id, scope: runtime }
```

Aucun champ inventé : `info.seq` et `runtime.actions` sont du schéma 1.0.0. Une ancienne version
d'Arnod garde ces actions (« conservées ») sans les appliquer.

### 6.2 Cœur Rust

- `request/dynamic.rs` (pur) : noms, tirage (`Dice`, graine tirée d'un UUID v4, aucune crate
  ajoutée ; graine fixe dans les tests), dates sans dépendance.
- `request/extract.rs` (pur) : application des extractions à la réponse et à la requête envoyée.
- `request/resolve.rs` : valeurs dynamiques tirées, variables utilisées, sources des valeurs tirées
  (`Prepared.drawn`, `used`).
- `opencollection/request.rs` : `runtime.actions` ↔ `Request.extractions`, `info.seq`.
- `tree/` : `TreeNode::Folder.test`, ordre `seq`, `create_test`, `add_step`, `test_view`,
  `variable_users`.
- `workbench.rs` : variables d'exécution par projet, extractions à l'envoi, run par unités (test
  séquentiel, requête seule), `ItemStatus::Skipped`, `RunView.tests` (`run_tests`), relance des
  tests entiers ; `SendOutcome.sent`, `drawn`, `used`, `extracted`.
- `results.rs` : `RequestResult.request` (version envoyée).
- `layout/` : contenu `Test { projectId, path }`, suivi des renommages et suppressions.

### 6.3 Commandes IPC

| Commande                 | Paramètres                 | Réponse          | Erreurs (`kind`)             |
| ------------------------ | -------------------------- | ---------------- | ---------------------------- |
| `test_read`              | `project`, `path`          | `TestView`       | `notFound`, `io`             |
| `tree_create_test`       | `project`, `parent`        | `CreatedTest`    | `notFound`, `nameTaken`…     |
| `tree_add_step`          | `project`, `test`          | chemin créé      | `notFound`, `invalidRequest` |
| `request_variable_users` | `project`, `path`, `names` | `VariableUser[]` | `notFound`, `io`             |

`request_send`, `run_*` et `request_result` gardent leur signature ; leurs réponses s'enrichissent.

### 6.4 Frontend

- `TestTab.vue` (zones) : onglet du test ; `StepBar.vue` : barre « TEST Étapes : N · Étape k
  - Étape » au-dessus d'une étape.
- `VariablesEditor.vue` : onglet « Variables » de `RequestEditor` ; `SentPanel.vue` : onglet
  « Envoyé » de `ResponsePanel`.
- `TreeRow.vue` : fiole d'un test, numéro des étapes, menu du test ; `TreePanel.vue` : « Nouveau
  test ». `RunPanel.vue` : « Non jouée », verdict et « Résultat par test ».
- Store `workbench` : `createTest`, `addStep`, `focusUrl`, `isChangedSinceSent`, `revertToSent`,
  `keepDrawn`.

### 6.5 Design

Aucun jeton ajouté. Fiole et numéros reprennent la grille `--tree-*` ; « non jouée » en
`--color-muted`, verdict en `--color-success` / `--color-danger` avec ✓ / ✕ (jamais la couleur
seule). Tout se fait au clavier (onglets, boutons, champs).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                   |
| ------------------------------------- | ------------------------------------------------------------ |
| `CLAUDE.md`                           | Section « Tests, étapes et variables ».                      |
| `docs/maquette-etat.yml`              | 4.1-b, 4.2-a, 4.2-b, 4.2-b bis, 5.1-h ; journal de 0.2.      |
| `src-tauri/capabilities/default.json` | Inchangé.                                                    |
| `e2e/arnod.e2e.ts`                    | `describe("Lot E - tests et étapes")` : trois étapes, échec. |
| Installeur                            | Aucune dépendance ajoutée.                                   |

## 8. Plan de tâches

- [x] **T1** Cœur : valeurs dynamiques, extractions, actions OpenCollection, `seq`.
- [x] **T2** Cœur : run d'un test (ordre, arrêt, verdict), variables d'exécution, version envoyée.
- [x] **T3** Cœur : test dans l'arborescence, nouveau test, « + Étape », onglet `test`, commandes.
- [x] **T4** Interface : onglet du test, barre de l'étape, arborescence, run.
- [x] **T5** Interface : onglet « Variables », « Envoyé », « Garder », « modifié, pas encore envoyé ».
- [x] **T6** Vitest, aperçus (4.2-a, 4.2-b-bis, 5.1-h, 4.1-b), e2e, maquette-etat, CLAUDE.md.

## 9. Critères d'acceptation

- [x] Un test de trois étapes passe une valeur extraite de l'étape 1 à l'étape 3 et s'arrête à la
      première étape en échec (`cargo test`, e2e écrit, joué à l'intégration).
- [x] Une extraction écrite puis relue est identique, le reste du fichier conservé.
- [x] `npm run check` passe.

## 10. Risques

| Risque                                    | Impact                         | Parade                                                                |
| ----------------------------------------- | ------------------------------ | --------------------------------------------------------------------- |
| Fichiers partagés avec les lots A, B, D   | Conflits de fusion             | Changements petits et localisés ; nouveaux composants à part.         |
| Ordre `seq` absent des dossiers existants | Étapes dans un ordre inattendu | Sans `seq`, ordre alphabétique inchangé ; « + Étape » pose les `seq`. |
| Variables d'exécution d'une session       | Valeur périmée réutilisée      | Un run repart de zéro ; « Envoyé » montre les valeurs utilisées.      |

## 11. Décisions

| ID  | Question                                   | Décision                                                                                                                                                                                   |
| --- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Q1  | Ordre des étapes                           | 2026-11-13 : `info.seq` du schéma (Bruno), puis le nom.                                                                                                                                    |
| Q2  | Où « Garder » range une valeur tirée       | 2026-11-13 : extraction de la valeur envoyée (`req.body.…`, comme les variables post-réponse de Bruno), sans champ inventé.                                                                |
| Q3  | Portée des valeurs extraites hors d'un run | 2026-11-13 : le projet, pendant la session ; un run en garde aussi les dernières.                                                                                                          |
| Q4  | Aléatoire                                  | 2026-11-13 : graine d'un UUID v4 (crate `uuid` déjà présente), générateur xorshift ; pas de crate ajoutée.                                                                                 |
| Q5  | Extractions importées de Bruno (lot C)     | 2026-11-13 : lues telles quelles (`res.body.…`, `res.headers['…']`, `res.status`, `res("…")`) ; une expression non reconnue donne un échec d'extraction lisible, jamais une erreur du run. |
