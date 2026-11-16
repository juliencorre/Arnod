# Plan - Résultat détachable

**Version** 0.1 · **Date** 2026-11-10 · **Statut** Livré · **Base** lots « fenêtres et zones » et
« barre latérale du projet »

Plan d'implémentation de l'écran i de la maquette « Arnod - Fenêtres et zones » : la barre
latérale, la requête et son résultat, chacun dans sa fenêtre. Il respecte les règles de
[`CLAUDE.md`](../CLAUDE.md).

Source : « Je voudrais pouvoir détacher la fenêtre de requête et la fenêtre de résultat, c'est un
gain énorme d'ergonomie. »

## 1. Objectif

Aujourd'hui, l'onglet d'une requête montre toujours son éditeur et son résultat ensemble : on peut
détacher l'onglet, pas séparer les deux. Une réponse n'est connue que de la fenêtre qui l'a
envoyée, et un run lancé depuis la barre latérale détachée s'ouvre vide dans une autre fenêtre. À la
fin de ce plan :

- « ⧉ Ouvrir le résultat dans une fenêtre » (dans l'onglet de la requête) ouvre une **fenêtre de
  résultat** : elle montre le résultat de la **requête active**, celle de la dernière fenêtre
  utilisée ;
- tant que le résultat a sa fenêtre, l'onglet d'une requête ne montre que l'éditeur, en pleine
  largeur ;
- l'envoi fait dans une fenêtre s'affiche dans toutes : le cœur garde le dernier résultat de chaque
  requête et le diffuse ;
- un run s'affiche dans la fenêtre où son onglet s'ouvre, quelle que soit la fenêtre qui l'a lancé.

## 2. Maquettes

| Écran | Capture                                       | Dans ce plan    |
| ----- | --------------------------------------------- | --------------- |
| i     | page « Fenêtres et zones », option i          | D1 à D6         |
| h     | page « Fenêtres et zones », option h (le run) | D7 (correction) |

## 3. Modèle et règles

- **Onglet de résultat** : un nouveau contenu, `result`, sans projet ni chemin ; il affiche le
  résultat de la requête active.
- **Requête active** : l'onglet actif de la zone active de la fenêtre utilisée le plus récemment
  dont l'onglet actif est une requête. Le cœur la calcule (ordre du focus, zones) et la diffuse.
- **Résultat d'une requête** : dernier envoi (en cours, réponse ou erreur), gardé en mémoire par le
  cœur pour la session.

| ID  | Règle                                                                                                                                         |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un seul onglet de résultat dans toute l'application : le redemander met sa fenêtre au premier plan.                                           |
| R2  | La fenêtre de résultat ne reçoit pas la sélection de la barre latérale (Z5) : elle suit la requête active.                                    |
| R3  | Tant qu'un onglet de résultat existe, quelle que soit sa fenêtre, l'onglet d'une requête ne montre que l'éditeur.                             |
| R4  | Rattacher la fenêtre de résultat (ou toutes les fenêtres) la ferme : le résultat revient dans l'onglet de chaque requête, rien n'est déplacé. |
| R5  | Le cœur garde le dernier résultat de chaque requête envoyée (au plus 50, les plus récents) et le diffuse à chaque début et fin d'envoi.       |
| R6  | Un envoi annulé ou en attente de confirmation (environnement protégé) laisse le résultat précédent.                                           |
| R7  | Un run démarré dans une fenêtre est suivi par toutes : son onglet affiche sa progression où qu'il soit ouvert.                                |

## 4. Cas d'usage

| ID  | Cas d'usage               | Écrans | Comportement attendu                                                                                                                             | Priorité |
| --- | ------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | -------- |
| D1  | Ouvrir le résultat à part | i      | ⧉ dans l'en-tête du résultat d'une requête : une fenêtre « Résultat » s'ouvre ; l'onglet de la requête passe en éditeur seul.                    | Must     |
| D2  | Suivre la requête active  | i      | Changer d'onglet ou de fenêtre de requête : la fenêtre de résultat montre le résultat de la nouvelle requête active (ou l'invite à envoyer).     | Must     |
| D3  | Envoyer, voir ailleurs    | i      | « Envoyer » dans la fenêtre de la requête : « Envoi en cours… » puis la réponse dans la fenêtre de résultat.                                     | Must     |
| D4  | Réponses attendues        | i      | Les deux onglets du résultat (Réponse, Réponses attendues) restent disponibles dans la fenêtre de résultat ; leurs modifications s'enregistrent. | Should   |
| D5  | Rattacher                 | i      | « Rattacher » ou fermer la fenêtre de résultat : le résultat revient dans l'onglet de la requête (R4).                                           | Must     |
| D6  | Trois fenêtres            | i      | Barre latérale détachée, fenêtre principale (la requête) et fenêtre de résultat ; restaurées au démarrage (Z8).                                  | Must     |
| D7  | Run d'une autre fenêtre   | h      | ▶ dans la barre latérale détachée : l'onglet du run s'ouvre dans la fenêtre principale et affiche sa progression (R7).                           | Must     |

## 5. Hors périmètre

- Plusieurs fenêtres de résultat (une par requête épinglée).
- Annuler un envoi depuis la fenêtre de résultat (l'éditeur garde « Annuler »).
- Conserver les résultats d'une session à l'autre.

## 6. Architecture

### 6.1 Formats sur disque

`windows.json` accepte le contenu `{ "kind": "result" }`. Rien d'autre n'est écrit.

### 6.2 Cœur Rust

- `layout` : `Content::Result` ; `project_id()` devient `Option<Uuid>` ; `Windows::active_request`
  (pur) ; `Windows::detach_result` (R1, R2) ; `reattach` écarte les onglets de résultat (R4).
- `results.rs` (nouveau, pur) : `ResultBook`, derniers résultats par requête (R5, R6).
- `windows.rs` : événement `arnod://requete-active` à chaque changement (opération ou focus).
- `commands/requests.rs` : `request_send` note le début et la fin dans le `ResultBook` et émet
  `arnod://resultat` ; `request_result` le relit. `commands/runs.rs` émet `arnod://run` (R7).

### 6.3 Commandes IPC

| Commande               | Paramètres        | Réponse                 | Erreurs (`kind`) |
| ---------------------- | ----------------- | ----------------------- | ---------------- |
| `window_detach_result` | `bounds`          | `WindowsFile`           | `io`             |
| `request_result`       | `project`, `path` | `RequestResult \| null` | aucune           |

`windows_load` renvoie aussi `activeRequest`.

### 6.4 Frontend

- `zones/ResultTab.vue` (nouveau) : en-tête de la requête active, puis `ResultSide`.
- `zones/ResultSide.vue` (nouveau) : onglets Réponse / Réponses attendues, extraits de
  `RequestTab.vue`.
- `RequestTab.vue` : ⧉ dans l'en-tête du résultat ; éditeur seul quand le résultat est détaché.
- `ContentWindow.vue` : pour la fenêtre de résultat, « ⇆ Suit la requête active » à la place de la
  bascule « Suit la sélection ».
- Stores : `windows.activeRequest`, `windows.resultDetached`, `windows.detachResult()` ;
  `workbench` applique `arnod://resultat` et `arnod://run` venus d'une autre fenêtre.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                          |
| ------------------------------------- | --------------------------------------------------- |
| `CLAUDE.md`                           | Section « Fenêtres et zones » : résultat détachable |
| `README.md`                           | Tableau des fonctions                               |
| `src-tauri/capabilities/default.json` | Inchangé                                            |
| `e2e/`                                | Parcours trois fenêtres et run d'une autre fenêtre  |

## 8. Plan de tâches pour Claude Code

- [x] **T1** Cœur : contenu `result`, requête active, fenêtre de résultat, `ResultBook`, événements ;
      tests Rust.
- [x] **T2** Interface : `ResultTab`, `ResultSide`, éditeur seul, en-tête de la fenêtre de résultat,
      événements ; Vitest au clavier.
- [x] **T3** e2e (trois fenêtres, run), README, `CLAUDE.md`, maquette i.

## 9. Critères d'acceptation

`npm run check` passe ; e2e clair et sombre ; envoi depuis la fenêtre principale visible dans la
fenêtre de résultat ; run lancé depuis la barre détachée visible dans la principale.

## 10. Risques

| Risque                                     | Impact                             | Parade                                  |
| ------------------------------------------ | ---------------------------------- | --------------------------------------- |
| Mémoire des corps de réponse (5 Mo chacun) | Croissance pendant la session      | 50 résultats au plus (R5)               |
| Brouillon modifié dans deux fenêtres       | Modification signalée « ailleurs » | Mécanisme existant (`changedElsewhere`) |

## 11. Décisions

| ID  | Question                                  | Décision (2026-11-10)                                                                                                     |
| --- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Q1  | Une fenêtre par requête ou une qui suit ? | Une qui suit la requête active : c'est le gain de l'écran i (D2).                                                         |
| Q2  | La fenêtre 2 de la maquette ?             | La fenêtre principale : elle existe toujours (Z7) ; la maquette i est ajustée.                                            |
| Q3  | Rattacher le résultat dans une zone ?     | Non : il revient dans l'onglet de chaque requête (R4) ; on peut glisser l'onglet de résultat dans une zone si on le veut. |
