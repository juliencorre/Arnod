# Plan - Configuration d'un endpoint

**Version** 0.1 · **Date** 2026-11-09 · **Statut** Livré · **Base** version 0.0.2

Plan d'implémentation de la configuration d'un endpoint de contrat, telle que la maquette
« Arnod - Contrats de service » la décrit (options a et a2). Il part de la version 0.0.2 (contrats
de services et format OpenCollection strict livrés) et respecte les règles de
[`CLAUDE.md`](../CLAUDE.md).

Source : demande « Mets à jour le programme Arnod avec les mises à jour de la configuration d'un
endpoint de contrat présent dans l'artefact ». Les écrans sont capturés dans
[`docs/maquettes/`](maquettes/).

## 1. Objectif

Dans Arnod, un endpoint est une requête du projet, liée ou non à une opération de contrat. À la
fin de ce plan, l'éditeur d'une requête :

- se modifie entièrement sur place et s'enregistre seul : méthode, adresse, nom, description,
  étiquettes ;
- règle la requête par onglets, comme dans Bruno : Paramètres (de requête et de chemin, avec
  description), Corps et ses variantes nommées, En-têtes (et ceux hérités), Auth (dont « Héritée »),
  Docs (Markdown), Réglages (délai, redirections, encodage, hérités ou propres) ;
- tient les **réponses attendues** de l'endpoint : exemples complets (code, texte du statut, nom,
  description, en-têtes, corps) à ajouter, dupliquer, supprimer avec « Annuler », modifier, ou
  reprendre de la dernière réponse reçue ;
- reçoit, pour une requête créée depuis un contrat OpenAPI, une réponse attendue par code
  documenté et une adresse à paramètres de chemin (`/pets/:petId`).

Tout s'écrit dans le fichier OpenCollection de la requête, sans champ inventé.

## 2. Maquettes

| Écran | Capture                                      | Dans ce plan                                          |
| ----- | -------------------------------------------- | ----------------------------------------------------- |
| a     | `maquettes/5a-endpoint.png`                  | EP1 à EP7, EP9 (en-tête, onglets, réponses attendues) |
| a2    | `maquettes/5b-endpoint-reponse-attendue.png` | EP2, EP8 (paramètres, détail d'une réponse attendue)  |

## 3. Modèle et règles

| Arnod              | OpenCollection 1.0.0                                                                         |
| ------------------ | -------------------------------------------------------------------------------------------- |
| Nom, description   | `info.name`, `info.description`                                                              |
| Étiquettes         | `info.tags` (hors étiquette `contract:`, jamais montrée)                                     |
| Paramètre, en-tête | `http.params[]`, `http.headers[]` : `name`, `value`, `description`, `disabled`, `type`       |
| Corps              | `http.body` : un corps, ou un tableau de variantes `{ title, selected, body }`               |
| Documentation      | `docs` (Markdown)                                                                            |
| Réglages           | `settings` : `timeout`, `followRedirects`, `maxRedirects`, `encodeUrl` ; absent = hérité     |
| Valeurs héritées   | `request` de `folder.yml` (dossier, puis parents), puis de `opencollection.yml`              |
| Réponse attendue   | `examples[]` : `name`, `description`, `response` (`status`, `statusText`, `headers`, `body`) |

Les règles vivent dans le cœur Rust, avec au moins un test par règle.

| ID  | Règle                                                                                                                                                                                                                                                                  |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Les paramètres de chemin sont les segments `:nom` ou `{nom}` de l'adresse, dans leur ordre ; à l'enregistrement, ceux qui n'y figurent plus sont retirés et ceux qui manquent sont ajoutés vides.                                                                      |
| R2  | Un corps sans variante nommée s'écrit en corps simple ; sinon en tableau de variantes, la sélectionnée marquée `selected`. Seule la variante sélectionnée part à l'envoi.                                                                                              |
| R3  | Un réglage absent (ou `inherit`) est hérité : `folder.yml` du dossier de la requête, puis de ses parents, puis `opencollection.yml` (`request.settings.http`), puis le défaut d'Arnod (30 s, redirections suivies, 5 au plus, adresse encodée).                        |
| R4  | Une authentification `inherit` reprend la première authentification définie (`request.auth`) sur la même chaîne ; aucune trouvée : la requête part sans authentification. Les en-têtes `request.headers` de la chaîne s'ajoutent à l'envoi, sauf s'ils sont redéfinis. |
| R5  | Délai 0 : aucun délai. Adresse non encodée : les paramètres partent tels quels.                                                                                                                                                                                        |
| R6  | Une réponse attendue est un exemple OpenCollection ; la partie `request` d'un exemple existant est conservée.                                                                                                                                                          |
| R7  | « Depuis la dernière réponse reçue » reprend statut, texte, en-têtes et corps (masqués comme la réponse), le type du corps déduit du `Content-Type`.                                                                                                                   |
| R8  | Une requête créée depuis une opération OpenAPI reçoit une réponse attendue par code documenté (nom et description du contrat, `Content-Type`, corps d'exemple du schéma) ; ses paramètres de chemin restent dans l'adresse (`:nom`).                                   |
| R9  | Description, étiquettes, documentation, descriptions des lignes, variantes, réglages et exemples sont gérés par Arnod : ils ne figurent plus dans le contenu conservé (`preserved`).                                                                                   |

## 4. Cas d'usage

| ID   | Cas d'usage                      | Écrans | Comportement attendu                                                                                                                                | Priorité |
| ---- | -------------------------------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| EP1  | Modifier sur place               | a      | Méthode, adresse, nom, description, étiquettes (« + Étiquette », × pour retirer). Enregistrement 600 ms après la dernière frappe, « ✓ Enregistré ». | Must     |
| EP2  | Paramètres                       | a2     | Paramètres de requête (case, nom, valeur d'exemple, description) ; paramètres de chemin tirés de l'adresse (R1), valeur et description.             | Must     |
| EP3  | Corps et variantes               | a      | Type (Aucun, JSON, XML, Texte, Formulaire), variantes nommées, « + Variante » copie la sélectionnée ; « Cette requête part sans corps ».            | Must     |
| EP4  | En-têtes                         | a      | Case, nom, valeur, description ; « Hérités » liste les en-têtes de la chaîne (R4) avec leur source.                                                 | Should   |
| EP5  | Auth                             | a      | Héritée, Aucune, Basic, Bearer, Clé d'API ; une note dit ce qui part, l'authentification héritée et sa source comprises.                            | Must     |
| EP6  | Docs                             | a      | Documentation en Markdown, éditée sur place.                                                                                                        | Must     |
| EP7  | Réglages                         | a      | Délai (Hérité · valeur, 5 s, 10 s, 60 s, Aucun), suivre les redirections, redirections au plus, encoder l'adresse ; « Hérité » montre la valeur.    | Must     |
| EP8  | Réponses attendues               | a, a2  | Liste (code, nom, texte, description), ⧉ dupliquer, ✕ supprimer avec « Annuler », › modifier : code, texte, nom, description, en-têtes, corps.      | Must     |
| EP9  | Depuis la dernière réponse reçue | a      | Bouton actif après un envoi : la réponse devient une réponse attendue, ouverte en modification.                                                     | Must     |
| EP10 | Endpoint créé depuis un contrat  | -      | Réponses attendues et paramètres de chemin remplis depuis l'opération OpenAPI (R8).                                                                 | Should   |

## 5. Hors périmètre

- Onglet « Tests et templates » et bouton « + Test » : le référentiel de tests et les templates
  viendront dans leur propre lot.
- Comparaison du lancement d'un contrat aux réponses attendues (codes non documentés, en-têtes et
  corps) : la conformité reste celle du schéma OpenAPI (lot contrats de services).
- Édition des réglages, en-têtes et authentification d'un dossier ou de la collection : ils sont
  lus (chaîne d'héritage), pas encore modifiables dans Arnod.
- Version propre d'un contrat créé à la main (« contrat de service, un dossier des endpoints ») :
  un contrat reste un fichier OpenAPI.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

```yaml
# Animalerie/animaux/trouver-un-animal.yml
info:
  name: Trouver un animal
  description: Renvoie un animal par son identifiant.
  type: http
  tags:
    - lecture
    - "contract:contracts/animalerie.yaml#GET /pets/{petId}"
http:
  method: GET
  url: "{{baseUrl}}/pets/:petId"
  params:
    - name: petId
      value: "1"
      type: path
      description: Identifiant de l'animal
  body:
    - title: Complet
      selected: true
      body: { type: json, data: '{"name": "Rex"}' }
    - title: Minimal
      body: { type: json, data: "{}" }
settings:
  timeout: 10000
  followRedirects: false
docs: |-
  ## Trouver un animal
examples:
  - name: Animal trouvé
    description: L'animal existe.
    response:
      status: 200
      statusText: OK
      headers:
        - name: Content-Type
          value: application/json
      body:
        type: json
        data: |-
          { "id": 1, "name": "Rex" }
```

Aucun changement de version : tout est déjà dans OpenCollection 1.0.0, et chaque écriture reste
validée contre le schéma.

### 6.2 Cœur Rust

- `request/model.rs` : `Request` gagne `description`, `tags`, `docs`, `settings` (`Settings` :
  options = hérité), `examples` (`Example`, `ExampleBodyKind`), `inherited` (calculé à la lecture,
  jamais écrit) ; `Pair` gagne `description` ; `Body` gagne `variants` et `selected`.
- `opencollection/request.rs` : correspondance des nouveaux champs, fusion conservant la partie
  `request` des exemples ; `normalize_path_params` (R1).
- `request/inherit.rs` (nouveau, pur sur le disque) : chaîne `folder.yml` → `opencollection.yml`,
  `Inherited` (authentification, en-têtes, réglages et leur source), `apply` avant l'envoi (R3, R4).
- `request/resolve.rs` et `http.rs` : variante sélectionnée, délai optionnel, politique de
  redirection, encodage (R2, R5).
- `request/example.rs` (nouveau) : exemple depuis une réponse reçue (R7).
- `contract/generate.rs` : réponses attendues et paramètres de chemin (R8).

### 6.3 Commandes IPC

| Commande                        | Paramètres                             | Réponse                       | Erreurs (`kind`)             |
| ------------------------------- | -------------------------------------- | ----------------------------- | ---------------------------- |
| `request_read` (modifiée)       | `project`, `path`                      | `RequestDoc` avec `inherited` | `notFound`, `invalidRequest` |
| `request_send` (modifiée)       | + `path` (pour l'héritage), facultatif | `SendOutcome`                 | inchangées                   |
| `request_example_from_response` | `response`, `name`                     | `Example`                     | -                            |

### 6.4 Frontend

- `RequestEditor.vue` : en-tête (fil du contrat, méthode, adresse, Envoyer, état d'enregistrement),
  nom, description, étiquettes ; onglets Paramètres, Corps, En-têtes, Auth, Docs, Réglages,
  Vérifications.
- `ExpectedResponses.vue` (nouveau) : liste et détail des réponses attendues, dans le panneau de
  droite à côté de la réponse reçue (onglets « Réponse » et « Réponses attendues »).
- Store `workbench` : enregistrement automatique (600 ms), état « Enregistrement… », « ✓ Enregistré »,
  « Non enregistré · Réessayer ».

### 6.5 Design

Palette existante (`var(--color-*)`), aucun jeton ajouté. Contrôles natifs (`select`, cases,
champs) ; boutons d'icône avec `aria-label`. Onglets de l'éditeur au clavier comme aujourd'hui.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                    |
| ------------------------------------- | ------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Configuration d'un endpoint », travail en cours     |
| `README.md`                           | Tableau des fonctions                                         |
| `src-tauri/capabilities/default.json` | Inchangé                                                      |
| `e2e/`                                | Endpoint du contrat : réglages, réponses attendues sur disque |
| Installeur                            | Aucune dépendance ajoutée                                     |

## 8. Plan de tâches pour Claude Code

**Lot A - Configuration d'un endpoint**

- [x] **T1** Plan, captures de la maquette.
- [x] **T2** Modèle et correspondance OpenCollection : description, étiquettes, docs, descriptions,
      variantes, réglages, exemples ; tests R1, R2, R6, R9.
- [x] **T3** Héritage (`inherit.rs`), envoi (variante, délai, redirections, encodage) ; tests R3 à R5.
- [x] **T4** Exemple depuis une réponse (R7) et requêtes créées depuis un contrat (R8).
- [x] **T5** Éditeur : en-tête, onglets, enregistrement automatique.
- [x] **T6** Réponses attendues : liste, détail, dupliquer, supprimer avec « Annuler », depuis la
      dernière réponse.
- [x] **T7** Vitest (clavier compris), e2e ; README, `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Une réponse attendue ajoutée dans l'éditeur se retrouve dans `examples` du fichier, valide
      pour le schéma.
- [x] Un endpoint créé depuis un contrat a ses réponses attendues et `:petId` dans son adresse.
- [x] Un délai réglé à 5 s s'applique à l'envoi ; « Hérité » reprend celui du dossier.

## 10. Risques

| Risque                                           | Impact                        | Parade                                                       |
| ------------------------------------------------ | ----------------------------- | ------------------------------------------------------------ |
| Enregistrement automatique pendant une frappe    | Écritures fréquentes          | 600 ms d'attente ; rien n'est réécrit si le contenu est égal |
| Variantes de types différents (fichier Bruno)    | Type unique de l'éditeur faux | Corps « autre » conservé tel quel, signalé                   |
| Adresse `{id}` (OpenAPI) et `:id` (Bruno) mêlées | Paramètre non remplacé        | Les deux formes sont reconnues (R1)                          |

## 11. Décisions

| ID  | Question                                  | Décision (2026-11-09)                                                                                              |
| --- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Q1  | Où afficher les réponses attendues ?      | Dans le panneau de résultat, onglet « Réponses attendues » à côté de « Réponse » : la réponse reçue reste visible. |
| Q2  | Enregistrement explicite ou automatique ? | Automatique après 600 ms (maquette, et Q7 du premier usage) ; « ✓ Enregistré » remplace le bouton.                 |
| Q3  | Type de corps par variante ou commun ?    | Commun, comme la maquette : le type s'applique à toutes les variantes.                                             |
| Q4  | Forme des paramètres de chemin écrits ?   | `:nom`, la forme de Bruno ; `{nom}` reste reconnue à la lecture et à l'envoi.                                      |
