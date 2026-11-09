# Plan - Fenêtres et zones

**Version** 0.1 · **Date** 2026-11-09 · **Statut** Livré · **Base** lots « premier usage » et
« workspaces en onglets »

Plan d'implémentation de la gestion des fenêtres : zones à onglets, séparations redimensionnables,
division d'une fenêtre, fenêtres détachées et interconnectées. Il respecte les règles de
[`CLAUDE.md`](../CLAUDE.md).

Source : maquette Claude Design, page « Fenêtres et zones » (a à h).

## 1. Objectif

À la fin de ce plan, Arnod :

- ouvre chaque requête dans un onglet de contenu, rangé dans une **zone** ; une fenêtre compte au
  plus 4 zones, divisées à droite ou en dessous ;
- se redimensionne par des **séparations** (barre latérale, zones) à la souris ou au clavier,
  avec la taille affichée pendant le glisser ; les tailles sont mémorisées ;
- détache une zone, un onglet ou la barre latérale dans une **fenêtre** du système, déplaçable
  sur un autre écran, puis la **rattache** ;
- garde les fenêtres **interconnectées** : une modification faite dans une fenêtre se voit dans
  toutes ; une fenêtre de contenu suit la sélection de la barre latérale, ou reste épinglée ;
- rouvre au démarrage ses zones, ses onglets et ses fenêtres détachées.

## 2. Maquettes

| Écran | Maquette                                                 | Dans ce plan |
| ----- | -------------------------------------------------------- | ------------ |
| a     | Diviser : glisser un onglet sur un bord                  | F2, F3       |
| b     | Redimensionner : séparation, taille affichée             | F4, F5       |
| c     | Test + mock : résultat dessous, journal qui suit         | F6, F10      |
| d     | Dispositions (menu Fenêtre)                              | F7           |
| e     | Détacher la barre latérale (icône au survol)             | F9           |
| f     | Rendre une zone autonome (⧉, clic droit, glisser dehors) | F8           |
| g     | Fenêtre épinglée sur l'écran 2                           | F10          |
| h     | Plusieurs écrans : barre latérale pilote                 | F9, F10, F11 |

## 3. Modèle

- **Onglet de contenu** : une requête (`projectId`, chemin) ou le run d'un projet. Un même
  contenu n'est ouvert qu'une fois par fenêtre.
- **Zone** : une liste d'onglets, un onglet actif, la place du résultat (à droite, dessous,
  automatique).
- **Disposition** : arbre binaire de zones et de séparations (`row` : côte à côte, `column` :
  l'une sous l'autre), chaque séparation avec sa proportion.
- **Fenêtre** : `main` (onglets des workspaces, barre latérale, zones), `content` (zones seules,
  suit la sélection ou épinglée), `sidebar` (barre latérale détachée, « pilote »).

| ID  | Règle                                                                                                                                                                              |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Z1  | Au plus 4 zones par fenêtre. Diviser au-delà est refusé avec un message.                                                                                                           |
| Z2  | Une zone vidée (dernier onglet fermé ou déplacé) disparaît, sauf la dernière de la fenêtre ; sa voisine prend sa place.                                                            |
| Z3  | Ouvrir un contenu déjà ouvert dans la fenêtre l'active, où qu'il soit.                                                                                                             |
| Z4  | Une proportion reste entre 10 % et 90 % ; l'interface impose en plus 280 px de large et 160 px de haut par zone.                                                                   |
| Z5  | Une sélection faite dans la barre latérale s'ouvre dans la dernière fenêtre utilisée qui la suit : la principale, ou une fenêtre de contenu non épinglée.                          |
| Z6  | Fermer une fenêtre de contenu la rattache : ses onglets reviennent dans la zone active de la fenêtre principale (rien n'est perdu). Fermer la barre latérale détachée la rattache. |
| Z7  | Fermer la fenêtre principale quitte Arnod.                                                                                                                                         |
| Z8  | Les dispositions et fenêtres sont enregistrées dans `windows.json` (données locales) et restaurées au démarrage.                                                                   |

## 4. Cas d'usage

| ID  | Cas d'usage                  | Comportement attendu                                                                                                                                             | Priorité |
| --- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| F1  | Onglets de contenu           | Une requête choisie dans la barre latérale s'ouvre en onglet ; ● si modifiée ; × ou Ctrl+W ferme.                                                                | Must     |
| F2  | Diviser en glissant          | Glisser un onglet sur une zone : bord droit, bord bas ou centre ; la zone de dépôt s'affiche avec son libellé.                                                   | Must     |
| F3  | Diviser par le menu          | Clic droit sur un onglet, ou ◫ : « Diviser à droite », « Diviser en dessous », « Déplacer vers », « Fermer ».                                                    | Must     |
| F4  | Redimensionner               | Glisser une séparation ; la taille des deux côtés s'affiche ; double-clic : parts égales ; Alt+Maj+flèches au clavier.                                           | Must     |
| F5  | Barre latérale               | Largeur réglable de la même façon, mémorisée.                                                                                                                    | Must     |
| F6  | Place du résultat            | « À droite » ou « Dessous · auto » : en automatique, le résultat passe dessous quand la zone est étroite.                                                        | Should   |
| F7  | Dispositions                 | Menu Fenêtre › Dispositions : 1 zone, 2 colonnes, 2 lignes, 3 zones, 4 zones ; les onglets sont conservés.                                                       | Should   |
| F8  | Zone ou onglet en fenêtre    | ⧉ ou Ctrl+Maj+O : la zone devient une fenêtre ; « Ouvrir cet onglet seul dans une fenêtre » ; glisser un onglet hors de la fenêtre l'ouvre à l'endroit du dépôt. | Must     |
| F9  | Détacher la barre latérale   | Icône au survol de sa séparation, ou Ctrl+Maj+D : la barre devient une fenêtre pilote ; « Rattacher ».                                                           | Must     |
| F10 | Suit la sélection / épinglée | Une fenêtre de contenu suit la sélection (Z5) ou reste épinglée ; bascule dans son en-tête.                                                                      | Must     |
| F11 | Interconnexion               | Projets, workspaces, requêtes enregistrées, arborescence et environnements se mettent à jour dans toutes les fenêtres.                                           | Must     |
| F12 | Rattacher                    | « Rattacher » dans une fenêtre, ou « Rattacher toutes les fenêtres » dans le menu Fenêtre (Z6).                                                                  | Must     |
| F13 | Restauration                 | Au démarrage : zones, onglets, tailles et fenêtres détachées comme à la fermeture (Z8).                                                                          | Should   |
| F14 | Agrandir une zone            | ⤢ : la zone occupe toute la fenêtre, ⤢ de nouveau la remet en place.                                                                                             | Could    |

## 5. Hors périmètre

- « Mes dispositions » (enregistrer une disposition nommée, Ctrl+Alt+1…) : lot suivant.
- Onglets d'autres contenus (mocks, campagnes, exigences, tableau de bord) : ils n'existent pas
  encore ; le modèle d'onglet est prévu pour eux.
- Glisser un onglet d'une fenêtre à une autre : « Déplacer vers » le fait au clavier et à la souris.
- Barre de titre personnalisée : barre native conservée (voir `plan-workspaces-onglets.md`, Q1).

## 6. Architecture

### 6.1 Cœur Rust

- `layout/` (pur, testé) : onglets, zones, arbre de disposition, fenêtres ; règles Z1 à Z6.
- `windows.rs` : état des fenêtres dans `AppState`, `windows.json` (écriture atomique), création
  des fenêtres (`WebviewWindowBuilder`, depuis des commandes asynchrones pour éviter l'interblocage
  sous Windows), suivi du focus (Z5) et des fermetures (Z6, Z7).
- Événements émis vers les fenêtres :

| Événement           | Contenu                                  | Émis après                                        |
| ------------------- | ---------------------------------------- | ------------------------------------------------- |
| `arnod://fenetres`  | `WindowsSnapshot` (toutes les fenêtres)  | toute modification de disposition                 |
| `arnod://catalogue` | `CatalogSnapshot`                        | toute modification du catalogue                   |
| `arnod://projet`    | `ProjectChange` (projet, portée, chemin) | arborescence, requête enregistrée, environnements |
| `arnod://fermer`    | aucun                                    | demande de fermeture d'une fenêtre détachée       |

### 6.2 Commandes IPC

`windows_load`, `layout_open`, `layout_open_selection`, `layout_activate`, `layout_close`,
`layout_move`, `layout_split`, `layout_resize`, `layout_preset`, `layout_set_placement`,
`layout_set_follow`, `layout_set_sidebar_width`, `window_detach_zone`, `window_detach_tab`,
`window_detach_sidebar`, `window_reattach`, `window_reattach_all`.

### 6.3 Permissions

La capacité `default` s'applique aussi aux fenêtres détachées (`"windows": ["main", "fenetre-*"]`)
: même `core:default`, aucune permission ajoutée. Les fenêtres sont créées par Rust. Le menu natif
gagne un menu « Fenêtre » (dispositions, Ctrl+Maj+O, Ctrl+Maj+D, rattacher toutes).

### 6.4 Frontend

- Store `windows` : instantané des fenêtres, étiquette de la fenêtre courante, opérations.
- Store `workbench` : données par projet (arborescence, environnements, run) et par onglet
  (brouillon, réponse) ; écoute `arnod://projet`.
- `components/zones/` : `ZoneArea` (zones de la fenêtre, raccourcis), `ZoneLayout` (arbre),
  `ZoneView` (onglets, outils, dépôt, menu), `SplitHandle` (séparation accessible :
  `role="separator"`), `RequestTab` (éditeur et résultat), `ProjectWelcome` ; un run s'affiche
  dans `RunPanel`. Les actions sur les onglets sont dans `composables/useTabActions.ts`.
- Fenêtres : la principale garde sa coquille ; `ContentWindow` (en-tête, suit la sélection,
  rattacher) et `SidebarWindow` (onglets des workspaces, barre latérale, pilote, rattacher).
- La barre latérale montre, sous chaque projet ouvert, son environnement et son arborescence.

## 7. Plan de tâches

**Lot A - Cœur**

- [x] **T1** Module `layout` : modèle, opérations, règles Z1 à Z6, tests.
- [x] **T2** `windows.json`, état des fenêtres, restauration (Z8), tests sur dossier temporaire.
- [x] **T3** Commandes, création et fermeture des fenêtres, focus, menu Fenêtre, événements.
- [x] **T4** Événements d'interconnexion (catalogue, projet) dans les commandes existantes.

**Lot B - Interface**

- [x] **T5** Store `windows`, store `workbench` par projet et par onglet, écoute des événements.
- [x] **T6** Zones : onglets, séparations, dépôt, menus, place du résultat, agrandir.
- [x] **T7** Barre latérale : arborescences des projets, largeur, détachement.
- [x] **T8** Fenêtres de contenu et barre latérale détachée.

**Lot C - Vérification**

- [x] **T9** Tests Vitest (clavier compris) ; e2e multi-fenêtres sur l'application réelle.
- [x] **T10** Captures, README et `CLAUDE.md`.

## 8. Critères d'acceptation

- [x] Une requête glissée sur le bord droit d'une zone ouvre une deuxième zone ; une cinquième est
      refusée.
- [x] Une séparation se règle à la souris et au clavier ; la taille revient après redémarrage.
- [x] Une zone détachée devient une fenêtre ; « Rattacher » ramène ses onglets.
- [x] Une requête enregistrée dans une fenêtre se met à jour dans l'autre.
- [x] La barre latérale détachée ouvre la sélection dans la fenêtre qui la suit, pas dans une
      fenêtre épinglée.

## 9. Décisions

| ID  | Question                                          | Décision (2026-11-09)                                                                                             |
| --- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Q1  | Où vit la disposition ?                           | Dans le cœur (règles testées, partagée entre fenêtres, persistée) ; l'interface ne garde que le glisser en cours. |
| Q2  | Fermer une fenêtre détachée perd-il ses onglets ? | Non : elle se rattache (Z6), après avoir enregistré ses requêtes modifiées.                                       |
| Q3  | Navigation des workspaces dans la barre détachée  | Chaque fenêtre a ses onglets de workspaces ; le dernier utilisé reste mémorisé (R7).                              |
| Q4  | Raccourcis Ctrl+Maj+O et Ctrl+Maj+D               | Accélérateurs du menu natif « Fenêtre », valables dans toutes les fenêtres.                                       |
