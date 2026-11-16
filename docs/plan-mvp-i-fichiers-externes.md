# Plan - MVP, lot I : fichiers modifiés hors d'Arnod

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** version 0.0.2 (Develop @6417638)

Plan du lot I de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : un projet est un dossier
suivi par Git, modifié aussi hors d'Arnod (`git pull`, éditeur). Écran `4.1-e` (page 4.1, fichier
maquette « Arnod - Référentiel de tests »). Il respecte [`CLAUDE.md`](../CLAUDE.md) et les règles
communes R1 à R10 du plan d'ensemble (citées « MVP-R… »).

## 1. Objectif

À la fin de ce lot, Arnod :

- surveille le dossier de chaque projet ouvert et recharge seul arborescence, contrats,
  environnements et onglets après une modification faite hors d'Arnod ;
- ignore ses propres écritures, `.git/`, les fichiers temporaires et les données du poste ;
- ferme l'onglet d'une requête supprimée (notification), suit une requête renommée ;
- ne perd jamais un brouillon : s'il est en conflit avec le disque, le conflit se règle élément par
  élément (« disque » ou « brouillon »), l'enregistrement automatique étant suspendu d'ici là ;
- ne réécrit jamais un fichier devenu illisible.

## 2. Maquettes

| Repère  | Capture (aperçu)                                                                   | Dans ce plan |
| ------- | ---------------------------------------------------------------------------------- | ------------ |
| `4.1-e` | `node scripts/apercu.mjs 4.1-e --maquette 4.1-e` (et `4.1-e-supprime`, `--sombre`) | FE1 à FE7    |

## 3. Modèle et règles

Le **registre** (`src-tauri/src/watch/mod.rs`) garde l'état connu de chaque projet surveillé :
empreinte du contenu de chaque fichier suivi (`.yml`, `.yaml`, `.json`), présence de chaque
dossier. Il garde aussi l'empreinte de la version de chaque requête qu'Arnod a lue ou écrite en
dernier. Un **élément** d'une requête est un champ simple (nom, méthode, adresse, corps,
authentification, réglages, docs, réponses attendues, rang) ou une ligne d'une liste (paramètre,
en-tête, vérification, extraction), reconnue par sa clé et son rang parmi les lignes de même clé.

| ID  | Règle                                                                                                                                                                                                                               |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Ignorés : tout élément caché (`.git/`, fichiers d'échange), `*.tmp` (écritures atomiques), `*~`, et tout fichier qui n'est ni YAML ni JSON. Les données du poste (`local.json`, statuts) sont hors du projet.                       |
| R2  | Toute écriture, création, renommage ou suppression d'Arnod met à jour le registre (`record_write`, `settle`) : l'événement du système qui suit ne change rien et n'est pas diffusé.                                                 |
| R3  | Les événements d'un projet sont regroupés 300 ms après le dernier ; le disque est comparé au registre : requête modifiée, ajoutée, supprimée ; portées `tree`, `environments` (`environments/`, `opencollection.yml`), `contracts`. |
| R4  | Requête supprimée : son onglet se ferme (notification), sauf s'il a un brouillon : « Restaurer » le réécrit, « Fermer l'onglet » l'abandonne.                                                                                       |
| R5  | Un fichier supprimé et un seul ajouté de même empreinte dans le même lot sont un renommage : onglets, brouillons, derniers résultats et statuts suivent.                                                                            |
| R6  | Le lot est diffusé par `arnod://fichiers` (`FilesChange`), puis par `arnod://projet` pour chaque portée et chaque requête modifiée (origine vide), comme l'interconnexion entre fenêtres.                                           |
| R7  | La surveillance démarre à la lecture de l'arborescence d'un projet (`tree_load`) et s'arrête quand le projet est oublié ou que son dossier disparaît.                                                                               |
| R8  | `request_save` refuse (`changedOnDisk`) d'écrire sur un fichier modifié depuis la version lue ou écrite par Arnod : rien n'est écrasé, même si l'événement n'est pas encore arrivé.                                                 |
| R9  | Requête modifiée sur disque et brouillon dans l'onglet : conflit. `request_conflict` liste les éléments qui diffèrent, puis ceux changés de la même façon des deux côtés (identiques). Pas d'enregistrement automatique d'ici là.   |
| R10 | Chaque élément qui diffère propose la version qui seule a changé depuis la version de départ du brouillon ; changé des deux côtés, aucun choix n'est fait d'avance et « Appliquer » attend.                                         |
| R11 | `request_conflict_resolve` part du brouillon, prend du disque les éléments choisis « disque » et écrit le fichier une seule fois ; un élément qui diffère sans choix (le fichier a encore changé) refuse la fusion.                 |
| R12 | Un fichier de requête illisible (YAML cassé, non conforme) est listé illisible et n'est jamais réécrit, même par l'enregistrement d'un onglet.                                                                                      |

## 4. Cas d'usage

| ID  | Cas d'usage                            | Écran   | Comportement attendu                                                                                                                                                                   | Priorité |
| --- | -------------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| FE1 | `git pull`, onglet sans brouillon      | `4.1-e` | L'onglet est relu ; « ⟳ Rechargé depuis le disque » quelques secondes à la place de « ✓ Enregistré ».                                                                                  | Must     |
| FE2 | Arborescence, contrats, environnements | `4.1-e` | Rechargés seuls ; la fenêtre principale résume le lot (« Rechargé depuis le disque : 4 requêtes modifiées, 1 ajoutée »).                                                               | Must     |
| FE3 | Requête supprimée                      | `4.1-e` | Onglet fermé avec une notification ; avec un brouillon : « Restaurer » ou « Fermer l'onglet ».                                                                                         | Must     |
| FE4 | Requête renommée                       | —       | Onglet, brouillon, résultat et statut suivent le nouveau chemin.                                                                                                                       | Should   |
| FE5 | Brouillon en conflit                   | `4.1-e` | Tableau élément / disque / brouillon / garder ; « Appliquer et enregistrer », « Tout prendre du disque », « Tout garder de mon brouillon » ; clavier (Tab, flèches des boutons radio). | Must     |
| FE6 | Écho d'une écriture d'Arnod            | —       | Rien ne se recharge, rien n'est annoncé.                                                                                                                                               | Must     |
| FE7 | Fichier devenu illisible               | —       | Listé illisible ; l'onglet ne le réécrit pas (erreur d'enregistrement affichée).                                                                                                       | Must     |

## 5. Hors périmètre

- La ligne fautive d'un YAML illisible ouverte « en lecture » (maquette) : l'erreur du cœur la cite.
- Les contrats modifiés ici et sur le disque à la fois : le contrat ouvert garde sa source modifiée
  (comportement existant du store `contracts`).
- La colonne « Sans conflit, rien à faire » de la maquette : note de conception, pas d'interface.

## 6. Architecture

### 6.1 Formats sur disque

Aucun changement. Rien n'est écrit dans le projet ni sur le poste par la surveillance.

### 6.2 Cœur Rust

- `watch/mod.rs` : registre (pur sauf la lecture des fichiers), `ignored`, `refresh`, `classify`,
  `apply`, `acknowledge`, `check_unchanged`. Nature d'erreur ajoutée : `changedOnDisk`.
- `watch/service.rs` : `Watchers` (un observateur `notify` par projet, un fil de regroupement de
  300 ms, trait `Sink`). Crate `notify` 8 justifiée dans `Cargo.toml`.
- `watch/app.rs` : `AppSink`, diffusion aux fenêtres (`arnod://fichiers`, `arnod://projet`), onglets,
  résultats et statuts qui suivent les renommages.
- `watch/conflict.rs` : `compare` et `merge`, purs.
- Écritures signalées : `opencollection::yaml::write`, `contract::file::write_text`, suppressions et
  renommages de `tree`, `environment`, `contract::file`.

### 6.3 Commandes IPC

| Commande                   | Paramètres                            | Réponse           | Erreurs (`kind`)                              |
| -------------------------- | ------------------------------------- | ----------------- | --------------------------------------------- |
| `request_conflict`         | `project`, `path`, `base`, `draft`    | `RequestConflict` | `notFound`, `invalidRequest`                  |
| `request_conflict_resolve` | `project`, `path`, `draft`, `choices` | `RequestDoc`      | `changedOnDisk`, `notFound`, `invalidRequest` |

`request_save` peut renvoyer `changedOnDisk`. Événement ajouté : `arnod://fichiers` (`FilesChange`).

### 6.4 Frontend

Store `workbench` : `Editor.conflict`, `choices`, `removedOnDisk`, `reloaded`, `draftSince` ;
`detectConflict`, `chooseConflict`, `conflictReady`, `resolveConflict`, `restoreRemoved`,
`discardRemoved`, `onFiles`, `filesSummary`. Composant `ConflictPanel.vue` montré par `RequestTab`
à la place de l'éditeur tant que le conflit dure. L'ancien bandeau « Modifiée dans une autre
fenêtre » disparaît : le même conflit vaut pour une autre fenêtre.

### 6.5 Design

Aucun jeton ajouté (`--color-danger`, `--color-danger-soft`, `--color-success`). Un seul bouton
plein (« Appliquer et enregistrer » ou « Restaurer »). Les choix sont des boutons radio natifs
groupés par élément (`role="radiogroup"`, flèches).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                     |
| ------------------------------------- | -------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Fichiers modifiés hors d'Arnod ».                    |
| `src-tauri/capabilities/default.json` | Inchangé.                                                      |
| `e2e/arnod.e2e.ts`                    | `describe` du lot I : fichier modifié sur disque, onglet relu. |
| Installeur                            | `notify` et ses dépendances : quelques centaines de Ko.        |

## 8. Tâches

- [x] **T1** Registre, regroupement, classification et conflit dans le cœur, testés (`watch/tests.rs`).
- [x] **T2** Commandes `request_conflict`, `request_conflict_resolve`, garde de `request_save`.
- [x] **T3** Store `workbench`, `ConflictPanel`, Vitest au clavier (`tests/workbench/fichiers-externes.spec.ts`).
- [x] **T4** Aperçu `4.1-e` et `4.1-e-supprime`, e2e, `maquette-etat.yml`, `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Un fichier `.yml` modifié sur disque pendant que l'application tourne met l'onglet à jour (e2e).
- [x] Une écriture d'Arnod ne recharge rien (`cargo test`, surveillant réel).
- [x] Un brouillon en conflit n'est jamais écrasé et se règle élément par élément (Vitest, `cargo test`).

## 10. Risques

| Risque                                      | Impact                       | Parade                                                            |
| ------------------------------------------- | ---------------------------- | ----------------------------------------------------------------- |
| Écho d'une écriture pris pour un changement | Rechargement en boucle       | Empreinte du contenu au moment de l'écriture (R2).                |
| Événement manqué ou en retard               | Brouillon écrasant le disque | Garde de `request_save` sur l'empreinte connue (R8).              |
| Gros dépôt (milliers de fichiers)           | Lecture lente au démarrage   | Seuls YAML et JSON sont lus ; `.git/` et cachés jamais parcourus. |

## 11. Décisions

| ID  | Question                                     | Décision                                                                                 |
| --- | -------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Q1  | Crate de regroupement (`notify-debouncer`) ? | 2026-11-13 : non, regroupement écrit dans `service.rs` (un fil, 300 ms).                 |
| Q2  | Écho : date ou empreinte ?                   | 2026-11-13 : empreinte du contenu (la date ne distingue pas deux écritures rapprochées). |
| Q3  | Vocabulaire « Ce test a changé »             | 2026-11-13 : « Cette requête a changé » (MVP-R4) ; le lot K renomme.                     |
