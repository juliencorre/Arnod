# Plan - Version 1 : apparence, raccourcis et historique des envois

**Version** 0.1 · **Date** 2026-11-27 · **Statut** Livré · **Base** version 0.0.4 (Develop @8a3d9aa)

Plan d'implémentation des écrans que la maquette range en **V1** : Préférences → Apparence
(`1.6-f`), Préférences → Raccourcis (`1.6-f bis`) et l'historique des envois d'une requête
(`5.1-c bis`). Il part de la version 0.0.4 (fin du MVP et de l'alignement sur la maquette) et
respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et le design system (`docs/design-system/`).

Source : maquette (pages 1.6 « Préférences » et 5.1 « Envoi et réponse »), `docs/maquette-etat.yml`.
Les écrans sont cités par leur repère (voir [`plan-suivi-maquette.md`](plan-suivi-maquette.md)).

## 1. Objectif

À la fin de ce plan, Arnod :

- suit le thème du système ou le force en clair ou en sombre, règle le zoom de l'interface et la
  police des éditeurs, pour ce poste ;
- liste chaque action et sa combinaison, par usage ; un clic la change, un conflit est signalé
  avant d'être remplacé, « Rétablir » revient aux raccourcis par défaut ; la nouvelle
  combinaison agit à la place de l'ancienne ;
- garde les 20 derniers envois de chaque requête sur le poste, les liste dans l'onglet
  « Historique » de la réponse, déplie chacun sur place et le compare au précédent.

## 2. Maquettes

| Repère      | Écran                                               | Dans ce plan |
| ----------- | --------------------------------------------------- | ------------ |
| `1.6-f`     | Préférences → Apparence : thème, zoom, police       | AP1 à AP4    |
| `1.6-f bis` | Préférences → Raccourcis : liste, conflit, rétablir | RC1 à RC6    |
| `5.1-c bis` | Historique : onglet, envoi déplié, comparaison      | H1 à H6      |

## 3. Modèle et règles

**Préférences d'interface** : `interface.json` dans le dossier de données (à côté d'`arnod.json`),
propres au poste, jamais dans un projet. **Action** : une commande de l'interface qui a un
raccourci (identifiant, libellé, groupe, portée, combinaisons par défaut, modifiable ou fixe).
**Combinaison** canonique : `Ctrl+Alt+Shift+Touche` ; lettres et chiffres d'après la touche
physique (AZERTY), signes d'après le caractère tapé ; Cmd vaut Ctrl. **Portée** : globale, ou
locale (éditeur, réponse, barre latérale, contrat). **Envoi gardé** : réponse reçue par un envoi
depuis l'éditeur (statut, en-têtes, corps coupé à 256 Ko, assertions, ce qui est parti, masqué).

Règles dans le cœur, modules purs `src-tauri/src/interface.rs` et `src-tauri/src/history.rs`, un
test par règle (`interface/tests.rs`, `history/tests.rs`).

| ID  | Règle                                                                                                                                                                                |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | Thème : `system` par défaut, ou `light` / `dark` forcé ; la page et la fenêtre native le suivent.                                                                                    |
| R2  | Zoom parmi 80, 90, 100, 110, 125, 150, 175, 200 % ; appliqué à chaque fenêtre par le cœur.                                                                                           |
| R3  | Police des éditeurs : celle du système ou une police proposée ; taille de 11 à 18 px. Rien d'autre accepté.                                                                          |
| R4  | Une combinaison se note sous forme canonique ; Ctrl+Tab et Ctrl+0…9 sont fixes.                                                                                                      |
| R5  | Sans Ctrl ni Alt, seule une touche de fonction est acceptée ; édition, zoom, onglets, workspaces réservés.                                                                           |
| R6  | Une combinaison tenue par une action de portée voisine (même portée, ou l'une globale) est un conflit : rien ne change sans « Remplacer », qui retire l'autre raccourci.             |
| R7  | Revenir à la combinaison par défaut efface la modification ; « Rétablir » les efface toutes.                                                                                         |
| R8  | Un fichier illisible ou une valeur hors des propositions revient aux valeurs par défaut, sans erreur.                                                                                |
| H1  | Chaque réponse reçue par `request_send` rejoint l'historique de sa requête, 20 au plus, le plus récent d'abord.                                                                      |
| H2  | Chaque envoi se compare au précédent ; le plus ancien n'a rien à quoi se comparer.                                                                                                   |
| H3  | Le résumé dit statut, environnement, en-têtes apparus, disparus ou changés (sauf Date, Age, Expires, X-Request-Id), corps (JSON comparé remis en forme), assertions réussies, durée. |
| H4  | « Voir les différences » : lignes retirées puis ajoutées (statut, en-têtes, corps, assertions), 300 au plus.                                                                         |
| H5  | « Effacer » oublie l'historique de la requête ; « Annuler » le remet après les envois reçus depuis.                                                                                  |
| H6  | L'historique suit renommages, déplacements et suppressions ; un fichier d'une version plus récente n'est jamais réécrit.                                                             |

## 4. Cas d'usage

| ID  | Cas d'usage              | Écrans      | Comportement attendu                                                                                             | Priorité |
| --- | ------------------------ | ----------- | ---------------------------------------------------------------------------------------------------------------- | -------- |
| AP1 | Forcer le thème          | `1.6-f`     | « Comme le système » (clair ou sombre en ce moment), « Clair », « Sombre » ; immédiat, dans toutes les fenêtres. | Must     |
| AP2 | Zoom                     | `1.6-f`     | Liste de zooms ; Ctrl + / Ctrl − / Ctrl 0 restent au clavier.                                                    | Must     |
| AP3 | Police des éditeurs      | `1.6-f`     | Police et taille : corps des requêtes et réponses, contrats, envoyé, historique.                                 | Must     |
| AP4 | Autres fenêtres          | `1.6-f`     | Un changement est diffusé (`arnod://interface`) et appliqué partout.                                             | Must     |
| RC1 | Liste des raccourcis     | `1.6-f bis` | Par usage (Requête et réponse, Fenêtre et onglets, Projet et workspace) ; recherche par action ou par touche.    | Must     |
| RC2 | Changer une combinaison  | `1.6-f bis` | Clic, puis la combinaison ; Échap annule ; un refus du cœur s'affiche sous la ligne.                             | Must     |
| RC3 | Conflit                  | `1.6-f bis` | « Ctrl+J est déjà pris par « … ». Remplacer · Annuler ».                                                         | Must     |
| RC4 | Rétablir                 | `1.6-f bis` | « Rétablir les raccourcis par défaut », inactif tant que rien n'a changé.                                        | Must     |
| RC5 | Le raccourci agit        | -           | Chaque composant demande au store `interface` si la touche est le raccourci de l'action.                         | Must     |
| RC6 | Envoyer au clavier       | `1.6-f bis` | Ctrl+Entrée dans l'éditeur envoie la requête (nouveau raccourci).                                                | Should   |
| H1  | Historique de la requête | `5.1-c bis` | Onglet « Historique N » de la réponse ; « N envois sur ce poste, hors Git ».                                     | Must     |
| H2  | Envoi déplié             | `5.1-c bis` | Le plus récent déplié ; Corps, En-têtes, Envoyé, Assertions.                                                     | Must     |
| H3  | Comparaison              | `5.1-c bis` | « Comparé à hier 17:20 : l'en-tête Location a disparu ; 4 ms de plus. Voir les différences ».                    | Must     |
| H4  | L'horloge                | `5.1-c bis` | L'horloge de la ligne d'état ouvre l'onglet ; sans réponse en session, l'historique reste lisible.               | Must     |
| H5  | Effacer                  | `5.1-c bis` | « Effacer », notification « Annuler ».                                                                           | Must     |

## 5. Hors périmètre

- « Rechercher un réglage » et les sections « Général » des préférences (aucun réglage général à
  ce jour) ; Réseau et Certificats restent une section (1.6-e).
- Action « Rechercher » (Ctrl+K) : la recherche est en V2 (1.5).
- Historique des lignes d'un run : seuls les envois depuis l'éditeur sont gardés.
- Libellés des infobulles et des menus qui citent un raccourci : ils gardent la combinaison par
  défaut.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque (poste seulement)

- `interface.json` : `{ "appearance": { "theme": "dark", "zoom": 110, "editorFont": null,
"editorFontSize": 13 }, "shortcuts": { "send": "Alt+Enter", "togglePlacement": null } }`,
  écriture atomique ; sans version (champs par défaut, R8).
- `projects/<id>/history/<chemin encodé>.json` : `{ "version": 1, "entries": [...] }`, un fichier
  par requête, nom en encodage pour cent réversible ; une version plus récente n'est jamais
  réécrite (H6).

### 6.2 Cœur Rust

`interface.rs` (actions, `normalize`, `assign`, `effective`, `validate`, `load`, `save`),
`history.rs` (`entry_of`, `compare`, `diff`, `view_of`, `record`, `clear`, `restore`, `rename`,
`remove`). `request_send` garde la réponse avant de diffuser le résultat ;
`Workbench::rename_status` / `remove_status` font suivre l'historique. Aucune dépendance ajoutée
(`percent-encoding` y était déjà).

### 6.3 Commandes IPC

| Commande                    | Paramètres                   | Réponse          | Erreurs (`kind`)             |
| --------------------------- | ---------------------------- | ---------------- | ---------------------------- |
| `interface_load`            | -                            | `InterfaceView`  | -                            |
| `interface_save_appearance` | `appearance`                 | `InterfaceView`  | `invalidRequest`, `io`       |
| `interface_set_shortcut`    | `action`, `keys?`, `replace` | `ShortcutChange` | `notFound`, `invalidRequest` |
| `interface_reset_shortcuts` | -                            | `InterfaceView`  | `io`                         |
| `history_load`              | `project`, `path`            | `HistoryView`    | `notFound`                   |
| `history_diff`              | `project`, `path`, `id`      | `HistoryDiff`    | `notFound`                   |
| `history_clear`             | `project`, `path`            | `HistoryView`    | `io`                         |
| `history_restore`           | `project`, `path`, `entries` | `HistoryView`    | `io`                         |

Événement `arnod://interface` (`InterfaceView`) après chaque changement.

### 6.4 Frontend

Store `interface` (vue, `matches(event, action)`, thème `data-theme` et variables
`--editor-font`, `--editor-font-size` sur la racine), `lib/shortcuts.ts` (lecture d'une touche,
affichage « Ctrl+Maj+K », `DEFAULT_SHORTCUTS` vérifiés par un test Rust), store `history`.
Composants : `AppearanceSection`, `ShortcutsSection` (préférences), `HistoryPanel` (réponse).
Les raccourcis de `AppShell`, `AppSidebar`, `WorkspaceSwitcher`, `ZoneArea`, `RequestEditor`,
`ResponseBody`, `ContractTab` et des préférences passent par le store.

### 6.5 Design

Aucun jeton de couleur ajouté. Le thème sombre forcé (`:root[data-theme="dark"]`) reprend chaque
jeton du thème sombre du système, vérifié par `tests/theme.spec.ts`. Capture d'une combinaison au
clavier (Échap annule, Tab sort) ; onglets de l'envoi déplié au motif `tablist`.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                    |
| ------------------------------------- | ------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Apparence, raccourcis et historique », arborescence |
| `src/styles.css`                      | Thème forcé, variables des éditeurs                           |
| `src-tauri/capabilities/default.json` | Inchangé : thème et zoom appliqués depuis Rust                |
| `e2e/`                                | Captures `v1-apparence`, `v1-raccourcis`, `v1-historique`     |
| `docs/maquette-etat.yml`              | `1.6-f`, `1.6-f bis`, `5.1-c bis` livrés                      |

## 8. Plan de tâches

**Lot V1 - Apparence, raccourcis et historique**

- [x] **T1** Cœur : `interface.rs`, commandes `interface_*`, événement `arnod://interface`.
- [x] **T2** Cœur : `history.rs`, commandes `history_*`, envoi gardé, renommages.
- [x] **T3** Interface : store `interface`, thème forcé, sections Apparence et Raccourcis.
- [x] **T4** Interface : raccourcis branchés sur le store, Ctrl+Entrée envoie.
- [x] **T5** Interface : `HistoryPanel`, horloge, « Effacer » et « Annuler ».
- [x] **T6** Tests Vitest (`tests/interface/`), faux cœur (`tests/fakeInterface.ts`), e2e.

## 9. Critères d'acceptation

- [x] `cargo test` : règles R1 à R8 et H1 à H6.
- [x] Vitest : thème forcé, zoom et police, conflit remplacé ou annulé, raccourci changé qui agit,
      historique, comparaison, différences, effacer et annuler.
- [x] `npm run check` passe.

## 10. Risques

| Risque                                       | Impact                      | Parade                                      |
| -------------------------------------------- | --------------------------- | ------------------------------------------- |
| Historique volumineux (corps de 5 Mo)        | Disque du poste             | Corps coupé à 256 Ko, 20 envois par requête |
| Zoom natif non pris en charge par la WebView | Le choix est sans effet     | Erreur ignorée ; Ctrl + / Ctrl − restent    |
| Deux blocs sombres à tenir                   | Jeton oublié en thème forcé | Test d'égalité des deux blocs               |

## 11. Décisions

| ID  | Question                                      | Décision (2026-11-27)                                                                                            |
| --- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Q1  | Thème et zoom par la page ou par la fenêtre ? | Thème : `data-theme` sur la page et thème de la fenêtre native ; zoom : `set_zoom` depuis Rust, sans permission. |
| Q2  | Actions fixes ?                               | Ctrl+Tab (relâcher Ctrl valide) et Ctrl+0…9 (famille) sont fixes.                                                |
| Q3  | Historique des runs ?                         | Non : envois depuis l'éditeur seulement (hors périmètre).                                                        |
