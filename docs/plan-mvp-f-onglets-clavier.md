# Plan - MVP, lot F : onglets et clavier

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** Develop @6417638

Plan du lot F de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : onglet d'aperçu et
exécution en cours, onglets en trop, Ctrl+Tab dans l'ordre d'utilisation, Ctrl+J et Ctrl+\. Il
prolonge [`plan-fenetres-zones.md`](plan-fenetres-zones.md) et
[`plan-resultat-detachable.md`](plan-resultat-detachable.md), respecte les règles de
[`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan d'ensemble (ajouts en fin de
liste dans les fichiers partagés, R5 ; aperçu comparé à la maquette, R6).

Source : maquette au 13 novembre 2026, page 1.3 « Onglets » (fichier « Arnod - Récents ») et
page 1.4 « Fenêtres et zones », écran j.

## 1. Objectif

À la fin de ce plan, Arnod :

- ouvre d'un simple clic dans la barre latérale un onglet d'**aperçu** (titre en italique),
  remplacé par l'aperçu suivant ; double-clic, modification ou glisser le gardent (1.3-a) ;
- montre ◐ dans l'onglet d'un élément qui s'exécute (envoi, run du test, du dossier, du projet) ;
- range les onglets qui ne tiennent pas dans la barre d'une zone sous « +N », qui propose aussi
  « Fermer les autres onglets » et « Fermer tous les onglets » (1.3-b) ;
- passe d'un onglet à l'autre par Ctrl+Tab et Ctrl+Maj+Tab dans l'ordre d'utilisation, liste
  affichée tant que Ctrl est tenu (1.3-c) ;
- met le résultat dessous ou à côté par Ctrl+J et masque la barre latérale par Ctrl+\, aussi
  dans le menu Fenêtre et sur l'icône de l'en-tête du résultat ; le choix est gardé par fenêtre
  (1.4-j).

## 2. Maquettes

| Repère | Écran                                    | Dans ce plan |
| ------ | ---------------------------------------- | ------------ |
| 1.3-a  | Onglets : aperçu en italique, ◐ en cours | AP1 à AP4    |
| 1.3-b  | Trop d'onglets : « +3 »                  | TO1, TO2     |
| 1.3-c  | Ctrl+Tab dans l'ordre d'utilisation      | CT1, CT2     |
| 1.4-j  | Au clavier : Ctrl+J, Ctrl+\              | CL1 à CL3    |

## 3. Modèle et règles

- **Onglet d'aperçu** : `Tab.preview` (`windows.json`, `false` par défaut). Au plus un par zone.
- **Ordre d'utilisation** : `Zone.recent`, identifiants des onglets de la zone, le plus récent
  d'abord ; tenu par le cœur à chaque activation, ouverture, fermeture et fusion.
- **Barre latérale masquée** : `WindowState.sidebarHidden` de la fenêtre principale.

| ID  | Règle                                                                                                                                                                                             |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un contenu ouvert en aperçu remplace sur place l'onglet d'aperçu de sa zone ; déjà ouvert, il est seulement activé (Z3).                                                                          |
| R2  | Au plus un onglet d'aperçu par zone : une fusion (disposition, rattachement) ou un fichier lu en garde un seul.                                                                                   |
| R3  | Un onglet devient permanent quand on l'ouvre sans aperçu (double-clic), qu'on le modifie, qu'on le glisse, le déplace, le divise, le détache ou le rattache.                                      |
| R4  | La sélection de la barre latérale s'ouvre en aperçu dans la fenêtre qui la suit (Z5) ; les ouvertures par une action (créer, ouvrir la requête d'origine, envoyer un essai…) restent permanentes. |
| R5  | `Zone.recent` contient exactement les onglets de la zone, sans doublon ; l'onglet actif est en tête. Ctrl+Tab parcourt cet ordre dans la zone active ; relâcher Ctrl active l'onglet choisi.      |
| R6  | « Fermer les autres » et « Fermer tout » enregistrent d'abord les requêtes et contrats modifiés ; un onglet inconnu refuse tout ; une fenêtre de contenu vidée se ferme.                          |
| R7  | Ctrl+J, Ctrl+\ et leurs entrées du menu Fenêtre font la même chose ; les touches sont lues par leur code (`KeyJ`, `Backslash`), le menu n'a pas d'accélérateur natif et émet `arnod://raccourci`. |
| R8  | La place du résultat reste celle de la zone (F6) ; Ctrl+J passe de l'autre côté, ou revient à l'automatique si c'est le côté qu'il donnerait. La barre masquée est gardée dans `windows.json`.    |

## 4. Cas d'usage

| ID  | Cas d'usage             | Écrans | Comportement attendu                                                                                                 | Priorité |
| --- | ----------------------- | ------ | -------------------------------------------------------------------------------------------------------------------- | -------- |
| AP1 | Aperçu                  | 1.3-a  | Un clic sur une requête, un test, un contrat ou une opération l'ouvre en italique, « (aperçu) » lu.                  | Must     |
| AP2 | Aperçu remplacé         | 1.3-a  | Le clic suivant remplace l'aperçu au même endroit de la barre.                                                       | Must     |
| AP3 | Garder l'onglet         | 1.3-a  | Double-clic (barre latérale ou onglet), modification ou glisser : l'onglet reste.                                    | Must     |
| AP4 | En cours                | 1.3-a  | ◐ (« en cours ») dans l'onglet pendant l'envoi de la requête, le run de son test, dossier ou projet.                 | Should   |
| TO1 | Onglets en trop         | 1.3-b  | « +N » liste les onglets cachés (méthode, nom, dossier) ; l'onglet actif reste visible.                              | Must     |
| TO2 | Fermer les autres, tout | 1.3-b  | Dans « +N » et dans le menu de l'onglet ; au clavier par les menus.                                                  | Must     |
| CT1 | Ctrl+Tab                | 1.3-c  | Liste « Onglets · dans l'ordre d'utilisation » sur la zone active, l'onglet d'avant choisi.                          | Must     |
| CT2 | Relâcher, autre sens    | 1.3-c  | Ctrl relâché, Entrée ou clic : ouvre ; Ctrl+Maj+Tab recule ; Échap ou perte du focus : rien ne change.               | Must     |
| CL1 | Ctrl+J                  | 1.4-j  | Le résultat de la zone active passe dessous ou à côté ; l'icône de l'en-tête du résultat fait de même.               | Must     |
| CL2 | Ctrl+\                  | 1.4-j  | La barre latérale se masque ; notification « Afficher » ; un bouton en haut la rouvre.                               | Must     |
| CL3 | Menu Fenêtre, gardé     | 1.4-j  | « Résultat dessous ou à côté (Ctrl+J) », « Masquer ou afficher la barre latérale (Ctrl+\) » ; redémarré : même état. | Should   |

## 5. Hors périmètre

- La barre de titre intégrée de la maquette (logo, sélecteur, « +3 » dans la barre du système) :
  écart transverse, la barre du système reste.
- « hier », ✗ et l'étoile ☆ de la maquette (date d'ouverture, statut, favori) dans les listes.
- Onglets d'exigences, d'anomalies, de campagnes : ces contenus n'existent pas encore.
- Raccourcis modifiables (V1).

## 6. Architecture

### 6.1 Formats sur disque

`windows.json` gagne trois champs facultatifs, lus à `false` ou vides dans un fichier plus
ancien : `tabs[].preview`, `zone.recent`, `windows[].sidebarHidden`. `Windows::from_file` les
répare (R2, R5). Rien n'est écrit dans un projet.

### 6.2 Cœur Rust

- `layout/mod.rs` : `Layout::open_tab` (aperçu, R1, R3), `Layout::pin`, `Zone::touch` et
  `Zone::repair` (R2, R5) ; `move_tab`, `split`, `absorb` rendent l'onglet permanent.
- `layout/windows.rs` : `open_selection(content, preview)`, `close_tabs` (R6),
  `set_sidebar_hidden` (R8).
- `menu.rs` : deux entrées sans accélérateur (`resultat-place`, `barre-masquer`) qui émettent
  `arnod://raccourci` (`Shortcut` : `placement` vers la dernière fenêtre à zones, `sidebar` vers la
  principale).

### 6.3 Commandes IPC

| Commande                    | Paramètres            | Réponse       | Erreurs (`kind`) |
| --------------------------- | --------------------- | ------------- | ---------------- |
| `layout_open_selection`     | `content`, `preview?` | `WindowsFile` | `notFound`       |
| `layout_pin`                | `tab`                 | `WindowsFile` | `notFound`       |
| `layout_close_tabs`         | `tabs`                | `WindowsFile` | `notFound`       |
| `layout_set_sidebar_hidden` | `hidden`              | `WindowsFile` | `invalidRequest` |

Événement ajouté : `arnod://raccourci` (`MenuShortcut` : `"placement" | "sidebar"`).

### 6.4 Frontend

- `lib/tabs.ts` (pur, testé) : `fitTabs` (onglets qui tiennent), `byUse` (ordre d'utilisation),
  `isRunning` (◐).
- Store `windows` : `pin`, `closeTabs`, `toggleSidebar`, `togglePlacement` (`nextPlacement`), liste
  de Ctrl+Tab (`tabSwitcher`, `stepTabSwitcher`, `commitTabSwitcher`), écoute de
  `arnod://raccourci`.
- `ZoneView.vue` : aperçu, ◐, « +N » (`BaseMenu`, entrées avec `badge`), liste de Ctrl+Tab ;
  `ZoneArea.vue` : Ctrl+Tab et Ctrl+J ; `AppShell.vue` : Ctrl+\ et le bouton qui rouvre ;
  `RequestTab.vue` : l'icône de la place du résultat remplace la bascule « À droite / Dessous ».
- Barre latérale : `TreeRow`, `LinkedRequestRow`, `ContractsSection` ouvrent en aperçu au clic,
  sans aperçu au double-clic.

## 7. Impacts sur l'existant

| Fichier                   | Changement                                                          |
| ------------------------- | ------------------------------------------------------------------- |
| `RequestTab.vue`          | La bascule « À droite · auto / Dessous » devient une icône (1.4-j). |
| `BaseMenu.vue`, `menu.ts` | Entrée avec `badge` (méthode colorée).                              |
| `tests/fakeWindows.ts`    | Aperçu, ordre d'utilisation, nouvelles commandes.                   |
| `e2e/arnod.e2e.ts`        | `describe` « Lot F » en fin de fichier.                             |
| `src-tauri/capabilities/` | Inchangé.                                                           |

## 8. Plan de tâches

- [x] **T1** Cœur : aperçu, ordre d'utilisation, fermer plusieurs, barre masquée ; tests R1 à R8.
- [x] **T2** Menu Fenêtre et `arnod://raccourci` ; IPC et types miroirs.
- [x] **T3** Interface : aperçu, ◐, « +N », Ctrl+Tab, Ctrl+J, Ctrl+\ ; Vitest au clavier.
- [x] **T4** Aperçu Playwright (1.3-a, b, c, 1.4-j ; clair, sombre, 900 px), e2e, maquette-etat,
      `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Trois clics simples dans la barre latérale laissent un seul onglet, en italique.
- [x] Avec plus d'onglets que la barre n'en tient, « +N » les liste et ferme les autres au clavier.
- [x] Ctrl+Tab puis Ctrl relâché ramène à l'onglet utilisé juste avant.
- [x] Ctrl+J et Ctrl+\ marchent en AZERTY ; la barre masquée l'est encore au redémarrage.
- [x] `npm run check` passe.

## 10. Risques

| Risque                                            | Impact                         | Parade                                                                        |
| ------------------------------------------------- | ------------------------------ | ----------------------------------------------------------------------------- |
| Parcours e2e qui comptent sur des onglets cumulés | Onglets remplacés par l'aperçu | Les ouvertures par action restent permanentes (R4) ; tests par double-clic.   |
| Accélérateur natif et touche lue par l'interface  | Action faite deux fois         | Pas d'accélérateur natif pour Ctrl+J et Ctrl+\ (R7).                          |
| Ctrl+Tab capté par le système ou la WebView       | Raccourci sans effet           | `preventDefault` ; les onglets restent accessibles par les flèches et « +N ». |

## 11. Décisions

| ID  | Question                                    | Décision (2026-11-13)                                                                                     |
| --- | ------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Q1  | Où vit l'ordre d'utilisation ?              | Dans le cœur (`Zone.recent`), gardé avec la disposition : il survit à un redémarrage et suit les fusions. |
| Q2  | Bascule « À droite · auto / Dessous »       | Remplacée par l'icône de l'en-tête du résultat, comme la maquette ; l'automatique reste (R8).             |
| Q3  | Accélérateurs du menu pour Ctrl+J et Ctrl+\ | Non : raccourci affiché dans le libellé, la touche est lue par l'interface (R7).                          |
| Q4  | Barre masquée hors fenêtre principale ?     | Non : seule la principale a une barre latérale ; la barre détachée se ferme ou se rattache.               |
