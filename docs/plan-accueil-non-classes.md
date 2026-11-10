# Plan - Accueil toutes natures, catalogue des étapes et Non classés

**Version** 0.1 · **Date** 2026-11-10 · **Statut** Décisions prises (§ 11) · **Base** version 0.0.2

Plan d'implémentation du nouveau premier usage : un **accueil** qui propose d'essayer une étape de
n'importe quelle nature (une seule pour l'instant : **REST**), un écran **Catalogue des étapes**, et
le projet **Non classés** qui garde les premiers essais sous le **contrat déduit** de leur service.
Il part de la version 0.0.2 et respecte les règles de [`CLAUDE.md`](../CLAUDE.md). Le modèle des
natures d'étape suit l'étude [`etude-catalogue-etapes.md`](etude-catalogue-etapes.md).

Source : maquette « Arnod - Premier usage », parties 1 (accueil, écrans a, a ter à a quinquies)
et 2 (Non classés), et page « Arnod - Catalogue des étapes »
([lien](https://claude.ai/code/artifact/a9ee8986-2c7c-4be7-8600-fb3460033bbe)). Demande : « mettre à
jour le premier usage et la page d'accueil avec comme seule étape l'étape REST ; le catalogue avec
l'étape REST et la gestion des contrats de service dans le projet des tests non classés ».

## 1. Objectif

À la fin de ce plan, Arnod :

- accueille au premier lancement par « Bienvenue dans Arnod » et une **entrée unique** « Essayer
  une étape » qui reconnaît ce que l'on colle ; une adresse `http(s)://` est reconnue comme REST ;
- ouvre sous l'entrée le **formulaire de la nature** choisie (méthode, type et corps pour REST) et
  réduit alors « Créer un projet » et « Importer » à une ligne ;
- envoie l'essai sans projet : il est gardé dans **« Non classés »**, sous le contrat déduit de son
  service (`contracts/<hôte>.yaml`), comme requête liée à l'opération, avec ses vérifications
  proposées et sa réponse attendue ;
- enrichit ce contrat à chaque essai sur le même serveur ;
- affiche « Non classés » en tête de « Tous les projets », hors workspaces, avec son icône ;
- présente un écran **Catalogue des étapes** qui liste les natures disponibles (REST, intégrée).

## 2. Maquettes

| Écran                 | Page de la maquette           | Dans ce plan                                                    |
| --------------------- | ----------------------------- | --------------------------------------------------------------- |
| 1 a                   | Premier usage, accueil        | A1 : entrée unique, pastilles, créer, importer                  |
| 1 a ter à a quinquies | Premier usage, nature choisie | A2 à A4 : formulaire de la nature, cartes réduites (REST seule) |
| 2 a                   | Premier usage, Non classés    | A5, N1 : essai gardé sous le contrat déduit                     |
| 2 c, 2 f              | Premier usage, Non classés    | N2, N3 : contrat enrichi, « Tous les projets »                  |
| c, d                  | Catalogue des étapes          | K1 : tableau des natures, fiche d'une nature (lecture seule)    |

## 3. Modèle et règles

- **Nature d'étape** : une définition (données) et un moteur (étude § 1). Ce lot embarque la
  définition REST (`src-tauri/steps/rest.json`), moteur intégré `http`.
- **Essai** : une étape envoyée depuis l'accueil, sans projet choisi.
- **Non classés** : le projet du poste qui reçoit les essais.
- **Contrat déduit** : un contrat OpenAPI 3.1 écrit par Arnod d'après les échanges réels avec un
  serveur, marqué `x-arnod-deduced: true`.

Les règles vivent dans le cœur Rust : `steps` (définitions, reconnaissance) et `contract::deduce`
(contrat déduit) sont purs ; `trial` crée Non classés et écrit les fichiers.

| ID  | Règle                                                                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Une définition embarquée est lue et validée au démarrage : identifiant unique, version sémantique, famille connue, moteur connu, champs du formulaire uniques, `visibleWhen` vers un champ `choice` existant. Une définition invalide n'est pas proposée.                 |
| R2  | Reconnaissance REST : `[MÉTHODE ]http(s)://hôte…`, espaces autour ignorés. La méthode en tête (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS, sans tenir compte de la casse) est reprise dans le formulaire. Autre texte : aucune nature.                                  |
| R3  | Les valeurs d'un essai sont vérifiées par le cœur : une option hors de la liste du champ est refusée, un champ masqué (`visibleWhen`) est ignoré, l'adresse doit être `http(s)://` avec un hôte.                                                                          |
| R4  | « Non classés » est unique sur le poste, retenu dans `arnod.json` (`inboxProjectId`). Créé au premier essai dans `<emplacement par défaut>/Non classés` ; un dossier projet déjà présent y est réinscrit ; un dossier non projet est refusé.                              |
| R5  | « Non classés » n'entre dans aucun workspace ; il est en tête de « Tous les projets ». L'oublier est permis : le prochain essai le réinscrit.                                                                                                                             |
| R6  | Un contrat déduit par origine (schéma, hôte, port) : `contracts/<hôte>.yaml`, titre = hôte (et port), serveur = origine, `info.version` `0.0.0`, `x-arnod-deduced: true`.                                                                                                 |
| R7  | Opération = méthode + chemin de l'adresse (sans paramètres ni fragment, `/` par défaut). Paramètres de requête déclarés avec leur valeur d'exemple. Réponse = statut reçu : exemple et schéma déduits d'un corps JSON ; texte sinon ; aucun contenu si le corps est vide. |
| R8  | Schéma déduit d'un JSON : `type` de chaque valeur, `properties` d'un objet (toutes requises), `items` d'un tableau d'après son premier élément, profondeur 8 au plus.                                                                                                     |
| R9  | Enrichissement : même origine, même fichier. Opération absente : ajoutée. Présente : un statut nouveau est ajouté, un statut connu n'est pas modifié. Un contrat sans `x-arnod-deduced` n'est jamais modifié : un contrat déduit `<hôte>-2.yaml` est créé à côté.         |
| R10 | Un contrat déduit illisible (modifié à la main) refuse l'essai avec son message ; rien n'est écrit.                                                                                                                                                                       |
| R11 | Essai : requête « Essai N » (premier numéro libre du projet), dans le dossier `<hôte>/`, adresse telle que saisie, liée à l'opération, vérifications proposées d'après la réponse, réponse reçue gardée comme réponse attendue.                                           |
| R12 | Un envoi en échec (réseau, délai, annulation) ne crée ni projet, ni contrat, ni requête.                                                                                                                                                                                  |

## 4. Cas d'usage

### 4.1 Accueil

| ID  | Cas d'usage        | Écrans     | Comportement attendu                                                                                                                                             | Priorité |
| --- | ------------------ | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| A1  | Premier lancement  | 1 a        | « Bienvenue dans Arnod », entrée « Essayer une étape » (pastille « Auto »), pastilles des natures (REST) et « Parcourir le catalogue… », puis créer et importer. | Must     |
| A2  | Coller une adresse | 1 a ter    | La pastille devient « REST », le formulaire REST s'ouvre sous l'entrée, créer et importer se réduisent à une ligne, le bouton prend le verbe « Envoyer ».        | Must     |
| A3  | Choisir une nature | 1 a quater | Clic, Entrée ou Espace sur une pastille : même effet ; l'entrée attend le champ principal de la nature (« https://… »).                                          | Must     |
| A4  | Revenir            | 1 a        | Échap dans l'entrée, ou « Fermer » : l'entrée se vide, le formulaire se ferme, les cartes reviennent.                                                            | Should   |
| A5  | Envoyer            | 2 a        | Envoi de l'essai (R11) : Non classés s'ouvre, l'onglet « Essai 1 » montre la réponse. Échec : message sous l'entrée, rien n'est créé (R12).                      | Must     |

### 4.2 Catalogue des étapes

| ID  | Cas d'usage            | Écrans | Comportement attendu                                                                                                                                          | Priorité |
| --- | ---------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| K1  | Parcourir le catalogue | c      | Depuis l'accueil ou « Tous les projets » : source des définitions, tableau (nature, famille en couleur, moteur, éditeur, version, état « Intégrée »), retour. | Must     |
| K2  | Fiche d'une nature     | d      | Choisir une ligne : description, moteur, champs de son formulaire. Clavier : flèches dans le tableau, Entrée pour la fiche.                                   | Should   |

### 4.3 Non classés

| ID  | Cas d'usage                  | Écrans | Comportement attendu                                                                                                                     | Priorité |
| --- | ---------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| N1  | Premier essai sans projet    | 2 a    | Non classés créé (R4), contrat déduit (R6 à R8), requête « Essai 1 » liée (R11).                                                         | Must     |
| N2  | Essais suivants              | 2 c    | Depuis « Tous les projets » (entrée « Essayer une étape ») : même contrat enrichi (R9), « Essai N ».                                     | Must     |
| N3  | Non classés dans les listes  | 2 f    | Icône de bac, en tête de « Tous les projets », « Hors workspaces » sans « Ajouter à… » ; contrat déduit dans « Contrats », sans version. | Must     |
| N4  | Contrat déduit modifié       | -      | Lisible : enrichi. Illisible : essai refusé avec le message (R10). Contrat écrit à la main : jamais modifié (R9).                        | Should   |
| N5  | Non classés oublié ou effacé | -      | Le prochain essai le réinscrit ou le recrée (R4, R5).                                                                                    | Should   |

## 5. Hors périmètre

- Synchronisation du catalogue, natures autres que REST, moteurs WebAssembly, épinglage dans
  `extensions.arnod.steps` (étude § 7).
- « Ranger dans un projet… » (déplacer ou fusionner un contrat déduit) et la généralisation des
  chemins (`/orders/42` → `/orders/{id}`) : lot suivant « Ranger les essais ».
- Transfert d'un essai au référentiel de tests (modèle tests, étapes et suites).

## 6. Architecture

### 6.1 Formats sur disque

- `arnod.json` (format 2, inchangé) : champ facultatif `inboxProjectId`. Absent : aucun Non classés.
  Une version précédente d'Arnod l'ignore ; Non classés y est alors un projet ordinaire.
- `Non classés/` : projet OpenCollection ordinaire (`opencollection.yml`, `environments/Local.yml`).
- `Non classés/contracts/api.boutique.dev.yaml` :

  ```yaml
  openapi: 3.1.0
  info:
    title: api.boutique.dev
    version: 0.0.0
    description: Contrat déduit des échanges réels avec https://api.boutique.dev.
  x-arnod-deduced: true
  servers:
    - url: https://api.boutique.dev
  paths:
    /products:
      get:
        operationId: get_products
        summary: GET /products
        responses:
          "200":
            description: OK
            content:
              application/json:
                schema: { type: array, items: { type: object, … } }
                example: […]
  ```

- `Non classés/api.boutique.dev/essai-1.yml` : requête OpenCollection liée (`contract:` dans
  `info.tags`), comme une requête créée depuis un contrat.

### 6.2 Cœur Rust

- `steps/` (nouveau, pur) : `definition.rs` (format, validation R1), `detect.rs` (R2), `values.rs`
  (R3), `mod.rs` (catalogue des définitions embarquées, `include_str!`).
- `contract/deduce.rs` (nouveau, pur) : contrat déduit d'un échange, schéma JSON (R6 à R9).
- `trial.rs` (nouveau) : Non classés (R4, R5), écriture du contrat et de l'essai (R10, R11).
- `catalog` : `inbox_project_id`, ordre de « Tous les projets », refus d'ajout à un workspace.
- Aucune dépendance ajoutée.

### 6.3 Commandes IPC

| Commande        | Paramètres                              | Réponse             | Erreurs (`kind`)                                          |
| --------------- | --------------------------------------- | ------------------- | --------------------------------------------------------- |
| `steps_catalog` | aucun                                   | `StepCatalog`       | aucune                                                    |
| `steps_detect`  | `input`                                 | `Detection \| null` | aucune                                                    |
| `trial_send`    | `execId`, `nature`, `subject`, `values` | `TrialResult`       | `invalidRequest`, `network`, `timeout`, `cancelled`, `io` |

`CatalogSnapshot` gagne `inboxProjectId`.

### 6.4 Frontend

- Store `steps` : catalogue (chargé une fois), reconnaissance, essai.
- `components/home/` : `TryStep.vue` (entrée, pastilles, formulaire), `NatureForm.vue` (rendu des
  champs déclaratifs), `NaturePill.vue` ; `FirstRun.vue` les assemble ; `AllProjects.vue` reprend
  l'entrée en tête.
- `components/steps/StepsCatalog.vue` : écran du catalogue, vue de navigation `steps`.
- `SidebarProject.vue`, `AllProjects.vue` : Non classés (icône, ordre, pas de workspace).

### 6.5 Design

Jetons de famille `--color-family-<famille>-{bg,border,text}` pour les deux thèmes (contraste
vérifié par `tests/theme.spec.ts`). Pastilles au clavier (Tab, Entrée, Espace), formulaire `choice`
en groupe radio (flèches), Échap pour revenir.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                  |
| ------------------------------------- | ----------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Accueil, catalogue et Non classés », arborescence |
| `README.md`                           | Tableau des fonctions                                       |
| `src-tauri/capabilities/default.json` | Inchangé                                                    |
| `e2e/`                                | Premier lancement : nouvel accueil ; essai vers Non classés |
| Installeur                            | Aucune dépendance ajoutée                                   |

## 8. Plan de tâches pour Claude Code

**Lot A - Accueil, catalogue REST et Non classés** (une pull request)

- [x] **T1** Étude du catalogue d'étapes et ce plan.
- [x] **T2** Cœur : module `steps` (définition REST embarquée, validation, reconnaissance, valeurs),
      commandes `steps_catalog` et `steps_detect` ; tests.
- [x] **T3** Cœur : `contract::deduce`, `trial` et Non classés dans le catalogue, commande
      `trial_send` ; tests.
- [x] **T4** Interface : accueil (entrée unique, formulaire déclaratif, cartes réduites), essai et
      ouverture de Non classés ; Vitest au clavier.
- [x] **T5** Interface : écran Catalogue des étapes ; Non classés dans la barre latérale et « Tous
      les projets » ; Vitest.
- [x] **T6** e2e (premier lancement, essai), `CLAUDE.md`, README.

## 9. Critères d'acceptation

- [x] `npm run check` passe ; e2e clair et sombre.
- [x] Une adresse collée sur l'accueil ouvre le formulaire REST ; « Envoyer » crée Non classés, le
      contrat déduit et « Essai 1 », et montre la réponse.
- [x] Un second essai sur le même serveur enrichit le même contrat.

## 10. Risques

| Risque                                 | Impact                | Parade                                                    |
| -------------------------------------- | --------------------- | --------------------------------------------------------- |
| Corps de réponse énorme dans l'exemple | Contrat lourd         | Exemple gardé seulement si le corps fait moins de 64 Ko   |
| Contrat déduit modifié à la main       | Écrasement de travail | Jamais réécrit s'il n'est plus marqué déduit (R9, R10)    |
| Nom « Non classés » déjà pris          | Création refusée      | Projet existant au même dossier réinscrit ; sinon message |

## 11. Décisions

| ID  | Question                                  | Décision (2026-11-10)                                                                  |
| --- | ----------------------------------------- | -------------------------------------------------------------------------------------- |
| Q1  | D'où faire les essais suivants ?          | De l'accueil au premier lancement, puis de l'entrée en tête de « Tous les projets ».   |
| Q2  | Version d'un contrat déduit ?             | `0.0.0` avec `x-arnod-deduced: true` ; la version n'est pas affichée.                  |
| Q3  | Nom des essais ?                          | « Essai N », renommable comme toute requête.                                           |
| Q4  | Environnement de Non classés ?            | « Local », comme tout projet ; l'adresse de l'essai reste absolue.                     |
| Q5  | Ranger un contrat déduit dans un projet ? | Lot suivant, avec la fusion dans un contrat existant et la généralisation des chemins. |
