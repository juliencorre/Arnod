# Plan - Lot C : import Bruno

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** version 0.0.2 (Develop @e10ba9a)

Plan d'implémentation du lot C de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : une
collection Bruno en fichiers `.bru` devient un projet OpenCollection, avec son rapport d'import. Il
respecte les règles de [`CLAUDE.md`](../CLAUDE.md), les règles communes R1 à R10 du plan
d'ensemble et le format strict de [`plan-opencollection.md`](plan-opencollection.md).

Source : maquette « Arnod - Import » (page 2.3), écrans `2.3-a`, `2.3-b` et `2.3-f` ; point de
départ « Reprendre l'existant » de `1.2-d`.

## 1. Objectif

À la fin de ce plan, Arnod :

- reconnaît seul ce qu'on lui donne à importer : collection Bruno en `.bru`, projet Arnod ou
  collection OpenCollection, contrat OpenAPI ;
- convertit une collection Bruno en projet OpenCollection valide, sans modifier les `.bru`
  d'origine, dans un nouveau projet ou dans un dossier d'un projet ouvert ;
- ouvre ensuite un rapport d'import : ce qui a été repris, ce qui est signalé et son fichier
  d'origine.

## 2. Maquettes

| Repère  | Écran                                    | Dans ce plan                                                                                 |
| ------- | ---------------------------------------- | -------------------------------------------------------------------------------------------- |
| `2.3-a` | Premier lancement, carte « Importer… »   | IM1, IM2 : la carte ouvre le sélecteur, le format est reconnu                                |
| `2.3-b` | « Reprendre l'existant » et « Importer » | IM3 : point de départ d'un projet vide, destination « Dans le projet » ou « Nouveau projet » |
| `2.3-f` | Rapport d'import Bruno                   | IM4, IM5                                                                                     |

## 3. Modèle et règles

Correspondance Bruno → OpenCollection 1.0.0 :

| Bruno                                                 | OpenCollection                                                                            |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `bruno.json` (`name`, `ignore`)                       | Nom du projet (ou du dossier créé) ; dossiers ignorés non lus                             |
| `collection.bru`                                      | `request` de `opencollection.yml` (en-têtes, auth, variables, scripts), `docs`            |
| Dossier, `folder.bru` (`meta.name`, `seq`)            | Dossier réel, `folder.yml` (`info.type: folder`, `seq`, `request`, `docs`)                |
| Requête `*.bru` (`meta`, bloc de la méthode)          | Fichier `.yml` (`info.name`, `seq`, `tags`, `http.method`, `http.url`)                    |
| `params:query`, `params:path`, `headers`              | `http.params` (`type`), `http.headers` ; ligne `~` → `disabled: true`                     |
| `body:json`, `text`, `xml`, `sparql`                  | `http.body` brut, seulement le mode choisi dans le bloc de la méthode                     |
| `body:form-urlencoded`, `body:multipart-form`         | `form-urlencoded`, `multipart-form` (`@file(…)` → partie `file`)                          |
| `auth:basic`, `bearer`, `apikey`, `inherit`           | `http.auth` (`placement: queryparams` → `query`) ; `none` → absente                       |
| `auth:digest`, `ntlm`, `wsse`, `awsv4`                | `http.auth` conservée, signalée (non envoyée par Arnod)                                   |
| `auth:oauth2` et autres                               | Non reprise, signalée avec son bloc                                                       |
| `vars:pre-request`                                    | `runtime.variables`                                                                       |
| `vars:post-response { id: res.body.id }`              | `runtime.actions` : `set-variable`, `after-response`, sélecteur `jsonq`, portée `runtime` |
| `assert { res.status: eq 200 }`                       | `runtime.assertions` (`expression`, `operator`, `value` en texte)                         |
| `script:pre-request`, `script:post-response`, `tests` | `runtime.scripts` (`before-request`, `after-response`, `tests`)                           |
| `docs`, `settings`                                    | `docs`, `settings` (`timeout` nul → absent)                                               |
| `environments/<nom>.bru` (`vars`)                     | `environments/<nom>.yml`, ordre de la collection                                          |
| `vars:secret [ … ]`, valeur `{{process.env.X}}`       | Variable locale (`secret: true`), sans valeur dans le projet                              |

Les règles vivent dans le cœur Rust : `bruno/` (pur sauf la lecture du disque) et `import.rs`.

| ID  | Règle                                                                                                                                                                                                                                   |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Les fichiers d'origine ne sont jamais écrits : l'import crée un nouveau projet (`<emplacement>/<nom de la collection>`) ou un nouveau dossier, au nom libre, dans un projet.                                                            |
| R2  | Le format est reconnu par le cœur, sans écriture : dossier avec `opencollection.yml` (ou `arnod.yml`), dossier avec `bruno.json`, fichier `.bru` ou `bruno.json` (sa collection), contrat OpenAPI. Le reste est refusé (`notAProject`). |
| R3  | Tout fichier écrit est validé contre le schéma OpenCollection 1.0.0 ; un fichier converti non conforme n'est pas écrit et est signalé. Aucun champ inventé.                                                                             |
| R4  | Une vérification garde son opérateur Bruno (mêmes noms qu'Arnod) ; sans opérateur connu en tête, c'est `eq` sur toute la valeur. `matches` et `notMatches` sont repris et signalés « à vérifier ».                                      |
| R5  | L'authentification suit le mode du bloc de la méthode (ou `auth { mode }` d'un dossier, de la collection) ; `inherit` reste `inherit` et l'héritage d'Arnod s'applique (`request/inherit.rs`).                                          |
| R6  | Scripts et tests sont gardés dans `runtime.scripts` du fichier, jamais exécutés, et signalés « non repris » avec leur code.                                                                                                             |
| R7  | Un secret (`vars:secret`, valeur `{{process.env.X}}`) devient une variable locale sans valeur ; une valeur lue dans `.env` va au coffre du système, sinon elle est « à renseigner ». Aucun secret n'est écrit dans le projet.           |
| R8  | Une valeur dynamique (`{{$randomUUID}}`, R1 commune) est gardée telle quelle et comptée.                                                                                                                                                |
| R9  | Une extraction est une action `set-variable` (R2 commune) ; une expression qui n'est pas un simple chemin de la réponse est reprise telle quelle et signalée « à vérifier ».                                                            |
| R10 | Le rapport (comptes, éléments signalés avec fichier et bloc d'origine, exemples) est gardé sur le poste (`projects/<id>/import.json`), jamais dans le projet ; un fichier illisible ne bloque pas le reste.                             |
| R11 | Dans un projet existant, un environnement de même nom est complété (variables absentes ajoutées, valeurs existantes gardées) ; sinon il est créé.                                                                                       |

## 4. Cas d'usage

| ID  | Cas d'usage                      | Écrans | Comportement attendu                                                                                                                  | Priorité |
| --- | -------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| IM1 | Importer au premier lancement    | 2.3-a  | « Choisir un dossier… » ou « ou un fichier… » ; le format reconnu s'affiche dans « Importer » ; « Importer » crée le projet.          | Must     |
| IM2 | Importer depuis Tous les projets | 2.3-a  | « Importer… » ouvre la fenêtre vide : choisir un dossier ou un fichier ; un format inconnu est expliqué dans la fenêtre.              | Must     |
| IM3 | Reprendre l'existant             | 2.3-b  | Projet vide : « Importer… » ; destination « Dans <projet> » (nouveau dossier) ou « Nouveau projet » ; dossier modifiable.             | Must     |
| IM4 | Rapport d'import                 | 2.3-f  | Onglet « Rapport d'import · <collection> » : comptes, « À reprendre » (5 lignes puis « … N autres »), exemples, détail avant / après. | Must     |
| IM5 | Ouvrir la requête d'origine      | 2.3-f  | « Ouvrir la requête » ouvre la requête créée ; flèches, Début, Fin parcourent la liste.                                               | Should   |
| IM6 | Projet Arnod ou contrat OpenAPI  | 2.3-a  | Un projet est ajouté tel quel (P12) ; un contrat crée un projet à son titre (ou s'ajoute au projet) et s'ouvre.                       | Should   |

## 5. Hors périmètre

- Exécution des scripts, tests et extractions : lot E (extractions), V4 (scripts).
- « Remplacer par une assertion… » et « Accepter sans ce script » (2.3-f) : pas de réécriture sans
  code d'un script Bruno au MVP.
- Dépôt d'un dossier sur la carte « Importer… » : le glisser-déposer natif est désactivé
  (`dragDropEnabled: false`).
- Postman (2.3-d, V4), SoapUI (2.3-e, V5), requêtes GraphQL (signalées, non importées).
- OAuth 2 : signalé, à régler dans Auth (lot H).

## 6. Architecture

### 6.1 Formats sur disque

Projet créé : `opencollection.yml` (avec `request` et `docs` de la collection), dossiers et
`folder.yml`, requêtes `.yml`, `environments/*.yml`. Rapport : `projects/<id>/import.json` dans les
données de l'application (format JSON de `ImportReport`). Aucun nouveau champ dans le projet.

### 6.2 Cœur Rust

- `bruno/parse.rs` : analyseur `.bru` écrit à la main (blocs, dictionnaires, `~`, `'''`, listes,
  blocs inconnus gardés bruts) ; `bruno/read.rs` : collection sur disque, `.env` ;
  `bruno/convert.rs` : conversion pure et rapport ; `bruno/tests.rs` sur la collection d'exemple
  `bruno/fixtures/boutique/`.
- `import.rs` : `detect`, `run` (nouveau projet par `AppState::create_project_with`, ou dossier
  d'un projet), environnements, rapport. `state.rs` : `create_project_with` (le remplissage
  remplace l'environnement « Local » ; dossier retiré en cas d'échec).
- `layout` : contenu `importReport { projectId }`.

### 6.3 Commandes IPC

| Commande        | Paramètres       | Réponse                | Erreurs (`kind`)                                                     |
| --------------- | ---------------- | ---------------------- | -------------------------------------------------------------------- |
| `import_pick`   | `file`           | `string \| null`       | `io`                                                                 |
| `import_detect` | `path`           | `ImportSource`         | `notAProject`, `notFound`, `io`                                      |
| `import_run`    | `path`, `target` | `ImportDone`           | `notAProject`, `nameTaken`, `folderNotEmpty`, `invalidRequest`, `io` |
| `import_report` | `projectId`      | `ImportReport \| null` | `notFound`                                                           |

`import_run` émet `arnod://projet` (arborescence, environnements, contrats) pour un projet existant ;
un projet créé passe par `arnod://catalogue`.

### 6.4 Frontend

Store `imports` (fenêtre, rapports) ; `components/import/` : `ImportDialog` (dans `SharedDialogs`),
`ImportReportTab` (onglet de zone), `ImportStart` (« Reprendre l'existant ») ; `lib/imports.ts`
(libellés). Entrées : `FirstRun.vue`, `AllProjects.vue`, `ProjectWelcome.vue`.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                |
| ------------------------------------- | ------------------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Import Bruno »                                                  |
| `src-tauri/capabilities/default.json` | Inchangé (dialog appelé depuis Rust)                                      |
| `e2e/arnod.e2e.ts`                    | `describe` « Import d'une collection Bruno », capture `c1-rapport-import` |
| Installeur                            | Aucune dépendance ajoutée                                                 |

## 8. Tâches

- [x] **T1** Analyseur `.bru`, testé bloc par bloc.
- [x] **T2** Lecture de la collection et conversion OpenCollection, rapport ; tests sur la collection d'exemple.
- [x] **T3** Import (nouveau projet, dossier d'un projet), reconnaissance, rapport sur le poste ; commandes `import_*`.
- [x] **T4** Fenêtre « Importer », onglet du rapport, entrées « Importer » et « Reprendre l'existant » ; Vitest.
- [x] **T5** Aperçu (2.3-a, 2.3-b, 2.3-f, clair, sombre, 900 px), e2e, `maquette-etat.yml`, `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Une collection Bruno en `.bru` devient un projet que l'arborescence et l'éditeur lisent, sans
      modifier ses fichiers (`cargo test`, e2e).
- [x] Aucun secret dans le projet ; les scripts sont gardés et signalés.
- [x] Le rapport liste chaque élément signalé avec son fichier d'origine.

## 10. Risques

| Risque                                   | Impact                      | Parade                                                                            |
| ---------------------------------------- | --------------------------- | --------------------------------------------------------------------------------- |
| Variantes du langage `.bru`              | Fichier illisible           | Blocs inconnus gardés bruts ; un fichier illisible est signalé, le reste importé. |
| Coffre indisponible (Linux sans service) | Valeur de `.env` non rangée | L'élément reste « à renseigner », l'import réussit.                               |
| Ordre `seq` non lu par l'arborescence    | Ordre d'affichage par nom   | `seq` écrit ; l'ordre d'un test est l'affaire du lot E (R3).                      |

## 11. Décisions

| ID  | Question                       | Décision                                                                                          |
| --- | ------------------------------ | ------------------------------------------------------------------------------------------------- |
| Q1  | Où montrer le rapport ?        | 2026-11-13 : un onglet de zone (`importReport`), comme dans la maquette ; relu depuis le poste.   |
| Q2  | Valeurs de `.env` ?            | 2026-11-13 : au coffre du système (connues), jamais écrites ; `vars:secret` reste à renseigner.   |
| Q3  | Expression d'une extraction ?  | 2026-11-13 : celle de Bruno, telle quelle (`res.body.id`), sélecteur `jsonq` ; le lot E l'évalue. |
| Q4  | Import dans un projet ouvert ? | 2026-11-13 : un nouveau dossier au nom de la collection, réglages dans son `folder.yml`.          |
