# Plan - Contrats de services

**Version** 0.1 · **Date** 2026-11-09 · **Statut** Livré · **Base** version 0.0.2

Plan d'implémentation des contrats de services : un projet décrit les services qu'il teste par
leurs contrats OpenAPI, crée les requêtes depuis les opérations du contrat et les exécute en
vérifiant que les réponses respectent le contrat. Il part de la version 0.0.2 (premier usage,
workspaces en onglets, fenêtres et zones) et respecte les règles de [`CLAUDE.md`](../CLAUDE.md).

Source : demande « On doit pouvoir créer des contrats et des requêtes et les exécuter » ; feuille
de route d'Arnod (import OpenAPI en V1, version du contrat d'échange associée au projet). Pas de
maquette dédiée : les écrans reprennent les motifs livrés (barre latérale, onglets de contenu,
dialogues, run).

## 1. Objectif

À la fin de ce plan, Arnod :

- ajoute un contrat à un projet en important un fichier, en collant son texte, depuis une adresse
  ou à partir d'un modèle vide, et l'affiche dans la barre latérale avec sa version ;
- présente le contrat dans un onglet : services, serveurs, opérations groupées par tag, et le texte
  source, modifiable ;
- crée en un clic les requêtes des opérations choisies, prêtes à envoyer : adresse, paramètres,
  en-têtes, authentification, corps d'exemple et vérifications ;
- exécute les requêtes du contrat et vérifie que chaque réponse est conforme : statut prévu et
  corps conforme au schéma.

## 2. Maquettes

Pas de maquette dédiée. Les écrans suivent les motifs des maquettes 1c à 1f (arborescence,
requête, réponse, run) et des fenêtres et zones (onglets de contenu).

| Écran | Motif repris                          | Dans ce plan |
| ----- | ------------------------------------- | ------------ |
| a     | Barre latérale, section « Contrats »  | C1 à C4, C11 |
| b     | Onglet de contrat, vue « Opérations » | C5 à C8, C10 |
| c     | Onglet de contrat, vue « Source »     | C9           |
| d     | Éditeur de requête, lien au contrat   | C7           |
| e     | Run du contrat                        | C10          |

## 3. Modèle et règles

- **Contrat** : description d'un service au format OpenAPI 3.0 ou 3.1, ou Swagger 2.0, en YAML ou
  en JSON. Il est rangé dans le dossier `contracts/` du projet, texte d'origine conservé pour que
  Git montre les vrais changements. Son **nom** est `info.title`, sa **version** `info.version`
  (la version du contrat d'échange).
- **Opération** : une méthode sur un chemin (`GET /pets/{petId}`), identifiée par cette **clé**.
  Elle porte un résumé, un `operationId`, des tags, des paramètres, un corps et des réponses.
- **Requête liée** : requête dont le fichier porte `contract: { file, operation }`. Elle reste une
  requête ordinaire, modifiable, rangée où l'on veut.
- **Conformité** : la vérification « est conforme au contrat » réussit si le statut reçu est prévu
  par l'opération (code exact, puis `2XX`, puis `default`) et si le corps JSON respecte le schéma de
  cette réponse.

Les règles vivent dans le cœur Rust, module pur `contract/`, avec au moins un test par règle.

| ID  | Règle                                                                                                                                                                                                            |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un texte est un contrat s'il a un champ `openapi` (3.x) ou `swagger: "2.0"` et un objet `paths` ; sinon il est refusé avec la raison, sans rien écrire.                                                          |
| R2  | Un contrat ajouté est écrit dans `contracts/<nom>.yaml` (ou `.json`, selon son format), nom tiré du titre ; un nom pris donne `<nom>-2`. Le texte est conservé tel quel.                                         |
| R3  | Les références locales (`#/components/…`, `#/definitions/…`) sont résolues, cycles compris ; une référence externe est signalée sur l'opération, sans bloquer le reste.                                          |
| R4  | Une opération ne crée qu'une requête : une opération déjà liée à une requête du projet n'en crée pas d'autre.                                                                                                    |
| R5  | Les requêtes créées vont dans `<nom du contrat>/<premier tag>/`, nommées d'après le résumé, sinon l'`operationId`, sinon la clé ; un nom pris reçoit un numéro.                                                  |
| R6  | Une requête créée est prête à envoyer : `{{baseUrl}}` + chemin, paramètres de chemin remplacés par leur exemple, paramètres requis actifs et facultatifs inactifs, corps d'exemple.                              |
| R7  | Une requête créée vérifie la durée (par défaut), le statut de succès prévu (plus petit `2xx` déclaré) et la conformité au contrat.                                                                               |
| R8  | À la création des requêtes, si `baseUrl` est vide ou absente de l'environnement choisi, elle prend l'adresse absolue du premier serveur du contrat ; une variable locale n'est jamais modifiée.                  |
| R9  | La conformité valide le sous-ensemble courant de JSON Schema : `type` (et `nullable`), `enum`, `const`, `required`, `properties`, `additionalProperties`, `items`, bornes, longueurs, `allOf`, `anyOf`, `oneOf`. |
| R10 | Un contrat modifié dans Arnod n'est enregistré que s'il reste lisible (R1) ; un fichier devenu illisible hors d'Arnod est listé avec son erreur et reste modifiable.                                             |
| R11 | Une requête liée dont l'opération a disparu du contrat est signalée dans l'onglet du contrat ; sa vérification de conformité échoue en le disant.                                                                |
| R12 | Le run d'un contrat exécute toutes ses requêtes liées, dans l'ordre de l'arborescence.                                                                                                                           |

## 4. Cas d'usage

| ID  | Cas d'usage          | Écrans | Comportement attendu                                                                                                                                                                    | Priorité |
| --- | -------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| C1  | Importer un fichier  | a      | « + Contrat › Importer un fichier… » ouvre le sélecteur natif (JSON, YAML) ; le contrat est ajouté (R2) et s'ouvre en onglet. Fichier refusé : notification avec la raison (R1).        | Must     |
| C2  | Coller un contrat    | a      | « Coller un contrat… » : dialogue avec une zone de texte ; « Ajouter » vérifie (R1) et ajoute ; l'erreur s'affiche dans le dialogue.                                                    | Must     |
| C3  | Depuis une adresse   | a      | « Depuis une adresse… » : l'adresse est lue par le cœur ; réponse non lisible ou injoignable : erreur dans le dialogue.                                                                 | Should   |
| C4  | Nouveau contrat vide | a, c   | « Nouveau contrat… » demande un nom et crée un contrat OpenAPI 3.0 minimal avec une opération d'exemple, ouvert sur sa source.                                                          | Must     |
| C5  | Consulter un contrat | b      | Onglet : nom, version, format, description, serveurs ; opérations groupées par tag avec méthode, chemin, résumé, « obsolète » ; couverture « n opérations · m avec requête ».           | Must     |
| C6  | Créer les requêtes   | b      | Cases à cocher (toutes par défaut) ; « Créer les requêtes » crée les requêtes manquantes (R4 à R8) ; notification avec le nombre créé et `baseUrl` renseignée le cas échéant.           | Must     |
| C7  | Requête liée         | b, d   | Une opération liée ouvre sa requête ; l'éditeur de requête affiche « Contrat : nom · clé » qui ouvre le contrat ; la vérification « est conforme au contrat » s'ajoute aussi à la main. | Must     |
| C8  | Envoyer et vérifier  | d      | Envoi : la vérification de conformité donne le détail du premier écart (`body.items[0].id : attendu integer, reçu "abc"`, statut non prévu, champ obligatoire absent).                  | Must     |
| C9  | Modifier la source   | c      | Vue « Source » : texte, « Enregistrer » (Ctrl+S) refusé si illisible (R10) avec l'erreur ; « Remplacer par un fichier… » met à jour depuis le disque.                                   | Should   |
| C10 | Exécuter le contrat  | b, e   | « Exécuter » lance le run des requêtes liées (R12) dans l'environnement actif ; le run affiche « Run du contrat « nom » », la confirmation d'environnement protégé s'applique.          | Must     |
| C11 | Supprimer un contrat | a      | Menu du contrat : « Mettre à la corbeille » ; ses requêtes restent (elles signalent le contrat absent) ; son onglet se ferme dans toutes les fenêtres.                                  | Should   |
| C12 | Écarts               | b      | Requêtes dont l'opération a disparu listées « hors contrat » (R11) ; opérations dont le contrat a une référence externe signalées.                                                      | Should   |

## 5. Hors périmètre

- WSDL et SOAP : contrats et requêtes SOAP (enveloppe XML, `SOAPAction`) dans un lot dédié, avec
  l'envoi SOAP (hors périmètre du squelette).
- Références externes à d'autres fichiers ou adresses (`$ref: autre.yaml#/…`) : signalées (R3).
- `pattern` et `format` de JSON Schema : non vérifiés (pas de moteur d'expressions régulières ajouté
  pour tenir l'installeur sous 15 Mo).
- Corps `multipart/form-data` avec fichiers, XML : la requête est créée avec un corps texte.
- Mocks à partir du contrat, génération de tests avancés (cas limites, valeurs invalides), import
  Postman et Bruno : plus tard (feuille de route).

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

- `contracts/` à la racine du projet, réservé comme `environments/` : absent de l'arborescence des
  requêtes, refusé comme nom de dossier racine. Un fichier par contrat, texte d'origine.
- Fichier de requête : champ facultatif `contract`, ignoré par une version d'Arnod qui ne le
  connaît pas (pas de changement de version du format) :

```yaml
# Requête Arnod
version: 1
id: 6c1f…
name: Trouver un animal
method: GET
url: "{{baseUrl}}/pets/1"
contract:
  file: contracts/petstore.yaml
  operation: GET /pets/{petId}
assertions:
  - id: duree
    target: responseTime
    op: lessThan
    value: 500
    origin: default
  - id: statut
    target: status
    op: equals
    value: 200
    origin: default
  - id: contrat
    target: response
    op: matchesContract
    origin: default
```

- `RunView` (dernier run, `local.json`) reçoit `contract` (nom du contrat), facultatif.

### 6.2 Cœur Rust

- `src-tauri/src/contract/` (pur, testé dans `contract/tests.rs`) : `parse.rs` (R1, R3, opérations),
  `sample.rs` (exemples d'après les schémas), `generate.rs` (R5 à R7), `conformance.rs` (R9, R11),
  `file.rs` (dossier `contracts/`, R2, R10, liens, R4).
- Aucune dépendance ajoutée : YAML par `serde-saphyr`, JSON par `serde_json`, lecture d'une
  adresse par `reqwest`, sélecteur de fichier par `tauri-plugin-dialog` (depuis Rust).
- `Operator::MatchesContract` : évaluée par l'établi (`workbench.rs`) qui lit le contrat lié.
- `ProjectScope::Contracts` : diffusé par `arnod://projet` quand les contrats changent.
- Onglet de contenu `Content::Contract { project_id, path }` ; supprimer un contrat ferme ses
  onglets dans toutes les fenêtres.

### 6.3 Commandes IPC

| Commande               | Paramètres                                     | Réponse             | Erreurs (`kind`)            |
| ---------------------- | ---------------------------------------------- | ------------------- | --------------------------- |
| `contract_list`        | `project`                                      | `ContractSummary[]` | `notFound`                  |
| `contract_read`        | `project`, `path`                              | `ContractView`      | `notFound`                  |
| `contract_add`         | `project`, `source` (texte, adresse, modèle)   | `ContractView`      | `invalidRequest`, `network` |
| `contract_import_file` | `project`, `replace` (chemin ou `null`)        | `ContractView?`     | `invalidRequest`, `io`      |
| `contract_save`        | `project`, `path`, `text`                      | `ContractView`      | `invalidRequest`, `io`      |
| `contract_delete`      | `project`, `path`, `permanent`                 | —                   | `io`                        |
| `contract_generate`    | `project`, `path`, `operations`, `environment` | `GenerateReport`    | `invalidRequest`, `io`      |
| `contract_run`         | `project`, `path`, `environment`, `confirmed`  | `RunView`           | `protectedEnvironment`      |

### 6.4 Frontend

- Store `contracts` : contrats par projet, vues par onglet, actions ; suit `arnod://projet`
  (portée `contracts`).
- `components/contracts/` : `ContractsSection` (barre latérale), `ContractDialog` (coller,
  adresse, nouveau), `ContractTab` (opérations, source).
- `ZoneView` affiche `ContractTab` pour un onglet de contrat ; `RequestEditor` montre le lien au
  contrat et l'opérateur « est conforme au contrat » ; `RunPanel` titre le run du contrat.

### 6.5 Design

Aucun jeton ajouté : méthodes colorées comme dans l'arborescence, états avec les jetons de succès
et d'échec existants. Tout est atteignable au clavier : menu « + Contrat », cases à cocher, vues
« Opérations » et « Source » en onglets (`tablist`), Ctrl+S dans la source.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                      |
| ------------------------------------- | ------------------------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Contrats de services », arborescence, travail en cours                |
| `README.md`                           | Statut, tableau des cas C1 à C12, capture                                       |
| `src-tauri/capabilities/default.json` | Inchangé                                                                        |
| `e2e/`                                | Parcours : contrat collé, requêtes créées, run conforme contre un serveur local |
| Installeur                            | Aucune dépendance ajoutée                                                       |

## 8. Plan de tâches pour Claude Code

**Lot A - Contrats de services**

- [x] **T1** Plan (ce document) et `CLAUDE.md` (travail en cours).
- [x] **T2** Cœur pur `contract/` : lecture (R1, R3), exemples, création des requêtes (R5 à R7),
      conformité (R9, R11), tests règle par règle.
- [x] **T3** Fichiers : `contracts/` (R2, R10), liens (R4), `baseUrl` (R8), arborescence qui
      ignore `contracts/`, champ `contract` des requêtes, opérateur `matchesContract`.
- [x] **T4** Commandes `contract_*`, run du contrat (R12), portée `contracts`, onglet de contrat,
      types TypeScript et concordance.
- [x] **T5** Store `contracts` et faux cœur de test.
- [x] **T6** Barre latérale : section « Contrats », menu « + Contrat », dialogue, suppression.
- [x] **T7** Onglet de contrat : opérations, création des requêtes, exécution, écarts, source.
- [x] **T8** Éditeur de requête et run : lien au contrat, opérateur de conformité, titre du run.
- [x] **T9** Tests Vitest (parcours au clavier) et e2e contre un serveur HTTP local.
- [x] **T10** README, `CLAUDE.md`, captures ; plan coché.

## 9. Critères d'acceptation

- [x] Un contrat collé apparaît dans la barre latérale avec sa version et s'ouvre en onglet.
- [x] « Créer les requêtes » crée une requête par opération, prête à envoyer, sans doublon au
      second clic.
- [x] Le run du contrat contre un service conforme réussit ; un écart de schéma fait échouer la
      vérification de conformité avec le chemin du champ en cause.
- [x] Un texte qui n'est pas un contrat est refusé avec la raison, sans fichier écrit.

## 10. Risques

| Risque                                             | Impact                         | Parade                                                              |
| -------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------- |
| Contrats réels volumineux (centaines d'opérations) | Onglet lent                    | Liste simple sans calcul côté interface ; lecture faite par le cœur |
| Schémas récursifs                                  | Exemple ou validation sans fin | Cycles détectés (R3), profondeur bornée                             |
| Sous-ensemble de JSON Schema                       | Faux « conforme »              | Mots-clés non vérifiés listés (§ 5)                                 |

## 11. Décisions

| ID  | Question                                        | Décision (2026-11-09)                                                                            |
| --- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Q1  | Quels formats de contrat ?                      | OpenAPI 3.0, 3.1 et Swagger 2.0. WSDL attend l'envoi SOAP, hors périmètre du squelette.          |
| Q2  | Où ranger les contrats ?                        | Dans le projet, `contracts/`, pour qu'ils voyagent avec les requêtes par Git.                    |
| Q3  | Paramètres de chemin : variables ou exemples ?  | Exemples : la requête part tout de suite ; l'utilisateur remplace par une variable s'il le veut. |
| Q4  | Valider le schéma : dépendance ou code maison ? | Code maison sur un sous-ensemble (R9) : pas de dépendance, installeur sous 15 Mo.                |
| Q5  | Enregistrer une source illisible ?              | Non (R10) : le contrat reste exploitable ; l'erreur s'affiche.                                   |

## Écarts constatés à l'implémentation

- Un run lancé depuis une commande synchrone (`run_start`, `contract_run`) paniquait dans
  l'application réelle : `tokio::spawn` hors d'un contexte tokio. Le run part désormais sur le
  runtime de Tauri (`tauri::async_runtime::spawn`) ; l'e2e du contrat le couvre. Le défaut venait
  du premier usage, dont les runs n'avaient pas encore d'e2e.
- Le run affiche, pour une requête en échec, le détail de la première vérification en échec
  (« 1 vérification(s) en échec : body[0].id : attendu entier, reçu "un" »).
- `serde_json` garde l'ordre du document (`preserve_order`) : opérations dans l'ordre du contrat,
  corps des réponses affichés dans l'ordre du serveur.
- Thème sombre : « ● Modifiée » de l'éditeur et l'outil de zone enfoncé utilisaient le jeton du
  texte posé sur l'accent (`--color-accent-text`, sombre) sur un fond sombre ; ils prennent
  `--color-link`, comme les nouveaux éléments des contrats.
