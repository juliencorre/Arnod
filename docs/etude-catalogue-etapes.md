# Étude - Catalogue des étapes

**Version** 0.1 · **Date** 2026-11-10 · **Statut** Étude, décisions prises (§ 9) · **Base** version 0.0.2

Comment ajouter des natures d'étape à Arnod (Kafka, SQL, batch, gRPC, SFTP, IHM…) **sans publier
une nouvelle version de l'application**, à la manière d'un antivirus qui récupère ses définitions.
L'étude fixe le modèle que la première nature, **REST**, suit dès maintenant ; la synchronisation
n'est pas implémentée dans ce lot (voir [`plan-accueil-non-classes.md`](plan-accueil-non-classes.md)).

Source : maquette « Arnod - Catalogue des étapes » (catalogue signé, natures du projet, mises à jour
proposées) et demande : « on puisse rajouter des étapes sans télécharger une nouvelle version de
l'application, mais une définition des étapes, comme un antivirus ».

## 1. Principe

Une nature d'étape a deux parties qui n'évoluent pas au même rythme :

| Partie         | Ce qu'elle porte                                                                                              | Mise à jour                                                          |
| -------------- | ------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Définition** | Des données : nom, famille, formulaire, reconnaissance de ce que l'on colle, verbe de l'action, vérifications | Par le catalogue, à tout moment, sans nouvelle version d'Arnod       |
| **Moteur**     | Le code qui exécute l'étape et rend un résultat                                                               | Intégré : avec Arnod. Isolé (WebAssembly) : livré avec sa définition |

L'antivirus fait pareil : le moteur d'analyse change rarement, les définitions chaque jour. Pour
Arnod :

- un **moteur intégré** vit dans le cœur Rust (`http` pour REST et SOAP, plus tard `process` pour
  le batch). Une définition peut changer son formulaire, ses textes ou sa reconnaissance sans
  nouvelle version, tant qu'elle demande un moteur qu'Arnod connaît (`engine.id` et `engine.abi`) ;
- un **moteur isolé** est un module WebAssembly signé, livré avec la définition (Kafka, SQL, gRPC,
  SFTP). Il tourne dans un bac à sable : pas de disque, réseau limité aux hôtes de l'environnement
  du projet, secrets fournis par le cœur depuis le coffre du système. C'est ce qui permet d'ajouter
  un protocole entier par le catalogue ;
- tout moteur rend le **même résultat** : un statut, une durée et des données JSON (réponse HTTP,
  lignes SQL, message Kafka, code de sortie…). Les vérifications sans code (cible, opérateur, valeur)
  restent donc communes à toutes les natures.

## 2. Définition d'une nature

Un fichier JSON, validé contre un schéma publié avec le catalogue (`nature.schema.json`). Exemple :
la définition REST, embarquée dans Arnod (`src-tauri/steps/rest.json`).

```json
{
  "format": 1,
  "id": "rest",
  "version": "1.0.0",
  "name": "REST",
  "title": "Une requête HTTP",
  "description": "Appelle une API par HTTP : méthode, adresse, corps ; vérifie le statut, les en-têtes et le corps.",
  "family": "calls",
  "publisher": "Arnod",
  "engine": { "kind": "builtin", "id": "http", "abi": 1 },
  "action": "Envoyer",
  "subject": { "key": "url", "label": "Adresse", "placeholder": "https://…" },
  "detect": [{ "kind": "url", "schemes": ["http", "https"], "methodPrefix": true }],
  "form": [
    {
      "key": "method",
      "label": "Méthode",
      "kind": "choice",
      "default": "GET",
      "options": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
    },
    {
      "key": "contentType",
      "label": "Type du corps",
      "kind": "choice",
      "default": "application/json",
      "options": ["application/json", "application/xml", "text/plain"],
      "visibleWhen": { "key": "method", "in": ["POST", "PUT", "PATCH"] }
    },
    {
      "key": "body",
      "label": "Corps",
      "kind": "multiline",
      "placeholder": "{ … }",
      "visibleWhen": { "key": "method", "in": ["POST", "PUT", "PATCH"] }
    }
  ]
}
```

| Champ           | Rôle                                                                                                                                                                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `format`        | Version du format de définition. Arnod ignore une définition d'un format plus récent et le signale dans le catalogue.                                                                                                                      |
| `id`, `version` | Identité stable (`kafka`) et version sémantique. Deux versions d'une même nature peuvent coexister sur le poste.                                                                                                                           |
| `family`        | Famille fermée, connue d'Arnod : `calls`, `data`, `messages`, `jobs`, `interface`, `code`, `files`. Elle donne la couleur.                                                                                                                 |
| `engine`        | Moteur demandé : `builtin` (`id`, `abi`) ou `wasm` (`module`, empreinte `sha256`, `abi`).                                                                                                                                                  |
| `action`        | Verbe du bouton : « Envoyer », « Exécuter », « Attendre », « Lancer ».                                                                                                                                                                     |
| `subject`       | Champ principal, saisi dans l'entrée de l'accueil : adresse, requête SQL, topic, commande.                                                                                                                                                 |
| `detect`        | Règles déclaratives pour reconnaître ce que l'on colle : `url` (schémas, méthode en tête), `keywords` (premier mot), `prefix`, `suffix`. Pas d'expressions régulières : Arnod les évalue lui-même, sans code venu du catalogue.            |
| `form`          | Champs déclaratifs : `choice`, `text`, `multiline`, `secret` (va au coffre), `pairs` ; `default`, `placeholder`, `hint`, `visibleWhen`. L'interface les affiche sans les connaître : une nature nouvelle n'a besoin d'aucun écran nouveau. |

Le formulaire décrit le **premier essai** (accueil, ajout d'une étape) ; l'éditeur complet d'une
nature intégrée reste écrit dans l'application (l'éditeur REST existant).

## 3. Distribution et synchronisation

Le catalogue est un **site statique HTTPS** (dépôt public `arnod-steps` publié tel quel, par exemple
GitHub Pages ou un CDN) : pas de serveur ni d'API à maintenir, et un miroir d'entreprise n'est qu'une
copie des fichiers.

```
https://steps.arnod.dev/v1/
  index.json            liste des natures publiées
  index.json.minisig    signature Ed25519 de index.json
  natures/kafka/1.0.2/nature.json
  natures/kafka/1.0.2/engine.wasm
```

`index.json` porte une **séquence** croissante, la date de publication et, pour chaque nature :
`id`, `version`, `family`, `publisher`, `status` (`stable`, `beta`, `retired`), `arnod` (version
minimale d'Arnod), et l'empreinte `sha256` de chaque fichier.

Synchronisation, côté cœur Rust uniquement (aucune permission WebView) :

1. Au démarrage puis toutes les 24 h, ou sur « Vérifier maintenant », en tâche de fond : requête
   conditionnelle (`If-None-Match`) sur `index.json` et sa signature.
2. Vérification de la signature avec la clé publique embarquée dans Arnod (une clé de secours
   aussi). Signature invalide, ou séquence plus ancienne que celle déjà connue : refus, rien ne
   change (pas de retour en arrière forcé).
3. Téléchargement des seules natures nouvelles ou mises à jour, vérification de chaque `sha256`,
   écriture atomique dans le cache `app_local_data_dir()/steps/`.
4. Hors ligne, proxy fermé, erreur : le catalogue reste celui du cache, et l'accueil comme les
   projets fonctionnent. L'adresse du catalogue et la clé d'un miroir d'entreprise se règlent dans
   les préférences.

La barre d'état signale les nouveautés (« Catalogue · 3 nouveautés »), jamais par une fenêtre.

## 4. Dans un projet

- **Natures intégrées** (REST) : toujours disponibles ; leur version suit celle d'Arnod.
- **Natures du catalogue** : « Ajouter au projet » **fixe leur version** dans `opencollection.yml`,
  dans la seule zone propre à Arnod autorisée par le format strict :

  ```yaml
  extensions:
    arnod:
      steps:
        kafka: 1.0.2
        sql: 0.8.1
  ```

  Les collègues et la commande `arnod` utilisent exactement cette version : le poste qui ne l'a pas
  encore la télécharge depuis le catalogue (même vérification). Une nouvelle version est
  **proposée**, jamais imposée. Une nature absente ou retirée laisse ses étapes lisibles mais non
  exécutables, avec un message clair.

- **Stockage d'une étape** : REST, GraphQL, gRPC et WebSocket ont un type d'élément OpenCollection et
  y restent. Les autres natures (SQL, Kafka, batch, SFTP, IHM) n'en ont pas : leurs étapes vont dans
  un fichier `<nom>.step` (YAML, schéma `arnod-step/1` : nature, version, valeurs du formulaire,
  vérifications), que les autres outils OpenCollection ignorent. Le format strict des fichiers
  OpenCollection n'est pas touché.

## 5. Premier essai sans projet

L'accueil propose les natures disponibles sur le poste (intégrées et en cache). L'essai d'une
nature du catalogue l'ajoute au projet « Non classés », à sa version du moment (même règle qu'au
§ 4). Arnod reconnaît la nature de ce que l'on colle en évaluant, dans le cœur, les règles `detect`
de chaque définition ; à égalité, la nature intégrée l'emporte, puis la plus précise
(`keywords` avant `url`).

## 6. Sécurité

- Signature de l'index et empreinte de chaque fichier ; clé publique embarquée, rotation par une
  nouvelle version d'Arnod ou par la clé de secours.
- Une définition n'est que des données : pas de code, pas d'expression régulière.
- Un moteur WebAssembly n'a que les capacités que le cœur lui donne (hôtes de l'environnement,
  délai, mémoire bornée) ; ses secrets passent par le coffre du système, jamais par le projet.
- L'installeur reste sans droits administrateur : le cache vit dans le dossier de données de
  l'utilisateur.

## 7. Ce que ce lot implémente

| Élément                                                         | Ce lot | Plus tard                           |
| --------------------------------------------------------------- | ------ | ----------------------------------- |
| Format de définition (`format: 1`) et sa lecture                | Oui    |                                     |
| Définition REST embarquée, moteur intégré `http`                | Oui    |                                     |
| Reconnaissance déclarative (`url`) dans le cœur                 | Oui    | `keywords`, `prefix`, `suffix`      |
| Formulaire déclaratif rendu par l'interface                     | Oui    | `secret`, `pairs`                   |
| Écran « Catalogue des étapes » en lecture                       | Oui    | Mises à jour, « Ajouter au projet » |
| Index signé, synchronisation, cache                             | Non    | Lot « Catalogue synchronisé »       |
| Moteurs WebAssembly, `extensions.arnod.steps`, fichiers `.step` | Non    | Lots des natures Kafka, SQL…        |

Le code range dès maintenant les définitions dans un module pur `steps` (lecture, validation,
reconnaissance) séparé de leur source : les définitions embarquées aujourd'hui, le cache synchronisé
demain, sans changer l'interface ni les commandes.

## 8. Risques

| Risque                                        | Parade                                                                     |
| --------------------------------------------- | -------------------------------------------------------------------------- |
| Définition malveillante ou altérée            | Signature, empreintes, données seulement (§ 6)                             |
| Nature qui exige un moteur absent de ce poste | `engine.id` et `abi` vérifiés : la nature est listée « Arnod trop ancien » |
| Projet partagé avec une nature non installée  | Version épinglée téléchargée à l'ouverture ; sinon étapes non exécutables  |
| Catalogue injoignable (réseau d'entreprise)   | Cache, miroir interne, préférences                                         |

## 9. Décisions

| ID  | Question                                               | Décision (2026-11-10)                                                                                |
| --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| Q1  | Site statique ou API ?                                 | Site statique signé : même format qu'une API, sans serveur ; une API pourra le servir à l'identique. |
| Q2  | Où la version d'une nature est-elle fixée ?            | `extensions.arnod.steps` de `opencollection.yml`, seule zone propre à Arnod du format strict.        |
| Q3  | Comment stocker les natures sans type OpenCollection ? | Fichiers `<nom>.step` à part ; les fichiers OpenCollection restent stricts.                          |
| Q4  | Formulaire écrit dans l'interface ou déclaratif ?      | Déclaratif : une nouvelle nature n'exige aucun écran nouveau.                                        |
| Q5  | Reconnaissance par expressions régulières ?            | Non : règles déclaratives évaluées par le cœur (sûreté, prévisibilité).                              |
