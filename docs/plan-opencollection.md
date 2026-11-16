# Plan - Format OpenCollection strict

**Version** 0.2 · **Date** 2026-11-09 · **Statut** Livré · **Base** version 0.0.2

Plan de passage des fichiers d'un projet Arnod au format [OpenCollection](https://spec.opencollection.com)
1.0.0, strictement : chaque fichier écrit par Arnod est valide pour le schéma officiel
[`v1.0.0.json`](https://schema.opencollection.com/opencollection/v1.0.0.json), conservé dans
[`docs/opencollection/`](opencollection/opencollection-v1.0.0.schema.json). Il part de la version
0.0.2 (premier usage, workspaces, fenêtres et zones, contrats de services) et respecte les règles
de [`CLAUDE.md`](../CLAUDE.md).

Source : demande « Je veux qu'on utilise pour l'instant le format open collection strict ».
OpenCollection est la spécification ouverte de collections d'API publiée par Bruno, format natif
de Bruno depuis la v3.1.

## 1. Objectif

À la fin de ce plan :

- un projet Arnod est une collection OpenCollection sur disque : `opencollection.yml` à la racine,
  un fichier `.yml` par requête, des dossiers réels, `environments/` ;
- tout fichier écrit par Arnod est valide pour le schéma 1.0.0, sans champ inventé ;
- un dossier de collection OpenCollection (Bruno 3 par exemple) s'ajoute à Arnod comme un projet
  existant, et ce qu'Arnod ne gère pas (scripts, autres authentifications, exemples, docs…) est
  conservé tel quel quand il enregistre ;
- un projet créé par une version précédente d'Arnod (`arnod.yml`) est converti à l'ouverture.

## 2. Maquettes

Pas d'écran nouveau : le format change sous l'interface. L'éditeur affiche les opérateurs de
vérification d'OpenCollection et le lien au contrat se défait depuis l'éditeur.

## 3. Modèle et règles

| Arnod                                  | OpenCollection 1.0.0                                                                                                                                                               |
| -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Manifeste `arnod.yml`                  | `opencollection.yml` : `opencollection: "1.0.0"`, `info.name`, `extensions.arnod`                                                                                                  |
| Identifiant du projet                  | `extensions.arnod.id` (racine ; `extensions` est le point d'extension du schéma)                                                                                                   |
| Ordre et protection des environnements | `extensions.arnod.environments.order` et `.protected`                                                                                                                              |
| Requête                                | `info` (`name`, `type: http`, `tags`), `http`, `runtime.assertions`, `settings`                                                                                                    |
| Paramètre d'adresse                    | `http.params[]` : `name`, `value`, `type: query` (ou `path`), `disabled`                                                                                                           |
| En-tête, champ de formulaire           | `name`, `value`, `disabled`                                                                                                                                                        |
| Corps JSON, texte, XML, formulaire     | `http.body` : `type` `json`, `text`, `xml`, `form-urlencoded` ; `data`                                                                                                             |
| Authentification                       | `http.auth` : `basic`, `bearer`, `apikey` (`key`, `value`, `placement`), `inherit` ; absente = aucune                                                                              |
| Délai                                  | `settings.timeout` (millisecondes), absent = 30 s                                                                                                                                  |
| Vérification                           | `runtime.assertions[]` : `expression` (`res.status`, `res.body.a[0]`, `res.headers["x"]`, `res.responseTime`), `operator` (`eq`, `lt`, `isDefined`…), `value` en texte, `disabled` |
| Lien au contrat                        | étiquette `info.tags` : `contract:<fichier>#<opération>`                                                                                                                           |
| Conformité au contrat                  | déduite du lien : vérifiée pour toute requête liée, sans vérification écrite                                                                                                       |
| Environnement                          | `environments/<nom>.yml` : `name`, `color`, `variables` ; variable locale = `secret: true`                                                                                         |

Les règles vivent dans le cœur Rust, module `opencollection/`, avec au moins un test par règle.

| ID  | Règle                                                                                                                                                                                     |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Tout fichier écrit par Arnod (racine, requête, dossier, environnement) est valide pour le schéma OpenCollection 1.0.0 ; les tests le vérifient avec le schéma officiel.                   |
| R2  | Un fichier lu qui n'est pas valide pour le schéma est listé comme illisible, avec la raison, et n'est jamais réécrit.                                                                     |
| R3  | Enregistrer une requête ne change que ce qu'Arnod gère ; le reste du fichier (scripts, variables, exemples, docs, description, autres authentifications et corps, réglages) est conservé. |
| R4  | Les opérateurs de vérification sont ceux d'OpenCollection et Bruno ; Arnod évalue tous ceux qui ne demandent pas d'expression régulière ou de script.                                     |
| R5  | Un dossier est un projet s'il contient `opencollection.yml` avec sa version ; une version de majeure supérieure à 1, ou une collection regroupée, est refusée sans écriture.              |
| R6  | Un projet au format précédent (`arnod.yml`) est converti à l'ouverture : racine, requêtes et environnements ; les fichiers propres à Arnod (`arnod.yml`, `_order.yml`) sont retirés.      |
| R7  | `opencollection.yml`, `folder.yml` et les dossiers réservés (`environments/`, `contracts/`) ne sont jamais lus comme des requêtes.                                                        |
| R8  | Une requête liée à un contrat est vérifiée contre son opération sans vérification écrite ; « Délier » retire l'étiquette.                                                                 |

## 4. Cas d'usage

| ID  | Cas d'usage                                                | Comportement attendu                                                                                               | Priorité |
| --- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | -------- |
| S1  | Créer un projet                                            | `opencollection.yml` et `environments/local.yml` valides ; plus d'`arnod.yml`.                                     | Must     |
| S2  | Créer, modifier, renommer, dupliquer, déplacer une requête | Fichiers OpenCollection valides ; le contenu non géré est conservé (R3).                                           | Must     |
| S3  | Vérifications                                              | Opérateurs OpenCollection dans l'éditeur ; valeurs écrites en texte ; vérification désactivée ignorée.             | Must     |
| S4  | Environnements                                             | Fichiers d'environnement valides ; variables locales en variables secrètes ; ordre et protection à la racine.      | Must     |
| S5  | Ajouter une collection existante                           | Un dossier OpenCollection (Bruno 3) s'ajoute comme projet existant ; son identifiant Arnod est ajouté à la racine. | Must     |
| S6  | Conversion                                                 | Un projet `arnod.yml` est converti à l'ouverture, sans perte (R6).                                                 | Must     |
| S7  | Contrats                                                   | Lien au contrat en étiquette ; conformité déduite ; « Délier » dans l'éditeur.                                     | Must     |
| S8  | Paramètres de chemin                                       | Un paramètre `type: path` remplace `:nom` dans l'adresse à l'envoi.                                                | Should   |

## 5. Hors périmètre

- Exécution des scripts, variables de requête et actions (`runtime.scripts`, `variables`,
  `actions`) : conservés, non exécutés.
- Authentifications autres que basique, porteur et clé d'API (digest, OAuth, AWS…) et corps
  multipart, fichier ou SPARQL : conservés, non envoyés (signalés dans l'éditeur).
- Collection regroupée en un seul fichier (`bundled: true`) et requêtes GraphQL, gRPC, WebSocket :
  listées comme non prises en charge.
- Opérateurs `matches` et `notMatches` : conservés, signalés comme non évalués (pas de moteur
  d'expressions régulières).

## 6. Architecture

### 6.1 Formats sur disque

```yaml
# opencollection.yml
opencollection: "1.0.0"
info:
  name: boutique-tests
extensions:
  arnod:
    version: 1
    id: 6c1f4e0a-…
    environments:
      order: [Local, Recette]
      protected: [Recette]
```

```yaml
# Clients/lister.yml
info:
  name: Lister
  type: http
  tags:
    - "contract:contracts/petstore.yaml#GET /pets"
http:
  method: GET
  url: "{{baseUrl}}/clients"
  params:
    - name: page
      value: "1"
      type: query
      disabled: true
runtime:
  assertions:
    - expression: res.responseTime
      operator: lt
      value: "500"
    - expression: res.status
      operator: eq
      value: "200"
```

```yaml
# environments/local.yml
name: Local
color: gray
variables:
  - name: baseUrl
    value: http://localhost:8080
  - name: token
    secret: true
```

### 6.2 Cœur Rust

- `src-tauri/src/opencollection/` : `schema` (schéma embarqué, validation par définition),
  `yaml` (lecture, écriture atomique, blocs littéraux pour les textes sur plusieurs lignes),
  `request` (requête ↔ `HttpRequest`, fusion avec le fichier existant, expressions et opérateurs),
  `collection` (racine et `extensions.arnod`), `migrate` (format précédent).
- `request/file.rs`, `environment.rs`, `manifest.rs` et `tree/` passent par ce module.
- Validation : le validateur de schéma des contrats (`contract::conformance`) sert aussi ici ;
  aucune dépendance ajoutée.

### 6.3 Commandes IPC

Aucune commande ajoutée. Les types changent : `RequestDoc` perd `version` et `id` ; `Assertion`
gagne `enabled` ; `AssertionOperator` reprend les opérateurs d'OpenCollection ; `AuthKind` gagne
`inherit` et `other` ; `BodyKind` gagne `xml` et `other` ; `Pair` gagne `kind` (paramètre de
chemin).

### 6.4 Frontend

Éditeur : liste des opérateurs, vérification activée ou non, authentification et corps conservés
signalés, « Délier » le contrat. Plus de libellé d'origine pour une vérification relue.

## 7. Impacts sur l'existant

| Fichier     | Changement                                                |
| ----------- | --------------------------------------------------------- |
| `CLAUDE.md` | Section « Format OpenCollection », arborescence, P12      |
| `README.md` | Statut, format des fichiers                               |
| `e2e/`      | Vérifications sur disque : `opencollection.yml`, requêtes |
| Installeur  | Schéma embarqué (≈ 70 Ko), aucune dépendance ajoutée      |

## 8. Plan de tâches pour Claude Code

**Lot A - Format OpenCollection strict**

- [x] **T1** Plan, schéma 1.0.0 dans `docs/opencollection/`, `CLAUDE.md`.
- [x] **T2** Module `opencollection/` : schéma, YAML, requêtes, racine ; tests par règle.
- [x] **T3** Requêtes, arborescence, environnements et manifeste sur le nouveau format.
- [x] **T4** Conversion du format précédent (R6) et ajout d'une collection existante (S5).
- [x] **T5** Contrats : lien en étiquette, conformité déduite (R8).
- [x] **T6** Interface : opérateurs, vérification désactivée, contenu conservé, « Délier ».
- [x] **T7** Tests Vitest et e2e ; README, `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Chaque fichier écrit pendant le parcours e2e est valide pour le schéma 1.0.0 (garanti à
      l'écriture : un contenu non conforme n'est jamais écrit).
- [x] Une requête au format Bruno 3 avec script et exemples garde ses scripts et exemples après
      enregistrement dans Arnod.
- [x] Un projet au format précédent s'ouvre, converti, sans perte de requête ni d'environnement.

## 10. Risques

| Risque                                         | Impact                         | Parade                                                                                  |
| ---------------------------------------------- | ------------------------------ | --------------------------------------------------------------------------------------- |
| Le schéma évolue (1.x)                         | Fichiers jugés invalides       | Schéma versionné dans le dépôt ; 1.x accepté en lecture                                 |
| Bruno lit `contracts/*.yaml`                   | Erreurs dans Bruno             | Extension `.yaml` ou `.json`, jamais `.yml`                                             |
| Sémantique des expressions différente de Bruno | Vérification évaluée autrement | Sous-ensemble documenté : `res.status`, `res.body…`, `res.headers…`, `res.responseTime` |

## 11. Décisions

| ID  | Question                                   | Décision (2026-11-09)                                                                                   |
| --- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Q1  | Où ranger ce qui est propre à Arnod ?      | Seulement là où le schéma le permet : `extensions` de la racine, étiquettes des requêtes. Rien d'autre. |
| Q2  | Identifiant et origine des vérifications ? | Non écrits : identifiants recalculés à la lecture, origine connue seulement pendant la session.         |
| Q3  | Vérification de conformité au contrat ?    | Déduite du lien (étiquette) : un opérateur inventé serait valide pour le schéma mais faux pour Bruno.   |
| Q4  | Projet existant sans identifiant Arnod ?   | Arnod ajoute `extensions.arnod.id` à `opencollection.yml` (seule écriture à l'ajout).                   |
| Q5  | Format précédent ?                         | Converti à l'ouverture (R6), une fois.                                                                  |
