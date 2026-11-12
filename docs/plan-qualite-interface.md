# Plan - Qualité visuelle et cohérence de l'interface

**Version** 1.0 · **Date** 2026-11-12 · **Statut** Livré · **Base** version 0.0.2

Plan d'un lot de design : appliquer le design system « Arnod » à l'application et à la maquette,
sans toucher aux fonctions. Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et suit
[`plan-suivi-maquette.md`](plan-suivi-maquette.md).

Source : revue de l'application réelle (46 captures e2e, thèmes clair et sombre), de la maquette
(écrans 3.2-a, 5.1-b, 5.2-d rendus) et du design system ([`design-system/`](design-system/README.md),
page 0.5 de la maquette).

## 1. Objectif

À la fin de ce plan :

- l'application n'a qu'une échelle de texte, de hauteurs de contrôle et de rayons, écrite en
  variables dans `src/styles.css` et lue par le design system ;
- le bouton principal est le même dans la maquette et dans l'application ;
- aucun contrôle natif ne sort de la palette ;
- l'éditeur de requête ne cache plus d'onglet.

Public : testeurs et développeurs, plusieurs heures par jour, écran de bureau d'au moins 1280 px,
clavier. Tâche principale : écrire une requête, l'envoyer, lire la réponse et ses vérifications,
puis exécuter un dossier.

## 2. Constats

| #   | Élément                 | Constat                                                                                                                   | Correction                                                                                 | Niveau              | Priorité |
| --- | ----------------------- | ------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------- | -------- |
| C1  | Taille du texte         | La zone principale héritait de 16 px, la barre latérale était à 13 px ; 12 tailles dans le code.                          | Texte de base à 13 px ; six tailles en variables `--text-*`.                               | Règle globale       | Haute    |
| C2  | Boutons, champs, listes | « Envoyer » faisait 48 px de haut, les champs 38 à 46 px ; 32 px dans la maquette.                                        | Trois hauteurs `--control-*` : 28, 32, et 40 px pour la seule barre d'adresse.             | Composants partagés | Haute    |
| C3  | Onglets de l'éditeur    | « Vérifications » était coupé à 1280 px en vue partagée, la barre de défilement masquée.                                  | Onglets plus serrés, sur une ligne à 1280 px ; ils passent à la ligne en zone étroite.     | Composant           | Haute    |
| C4  | Bouton principal        | Rouille et texte blanc dans la maquette, orange et texte foncé dans l'application ; un bouton plein noir (« + Réponse »). | Un seul bouton plein, orange ; « + Réponse » secondaire ; la maquette alignée.             | Règle globale       | Haute    |
| C5  | Rayons                  | 3, 4, 5, 6, 8, 10, 12 et 14 px dans le code.                                                                              | Variables `--radius-*` : 2, 6, 10 px et forme ronde.                                       | Règle globale       | Moyenne  |
| C6  | Cases et listes         | Cases à cocher en bleu du système, listes déroulantes au style natif gris.                                                | Case à la couleur d'accent, coche foncée ; liste au gabarit des champs.                    | Règle globale       | Moyenne  |
| C7  | Libellés et actions     | Dans « Environnements », libellés en 16 px gras, plus forts que les valeurs ; liens d'action en 16 px gras.               | Libellés en `--text-small` et `--color-muted` ; actions textuelles à 13 px.                | Composition         | Moyenne  |
| C8  | Couleurs de la maquette | 220 teintes écrites à la main, la moitié des emplois sur des jetons.                                                      | Teintes à moins de 6 d'écart RVB remplacées par le jeton (`scripts/maquette-couleurs.py`). | Règle (maquette)    | Moyenne  |

C1 à C6 et C8 se mesurent dans le code ou les captures ; C7 est en partie un jugement de
hiérarchie.

## 3. Direction

« Établi calme » : un outil de bureau dense, où le contenu technique passe devant l'interface.

| ID  | Règle                                                                                                                                                                     |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Six tailles de texte (11, 12, 13, 15, 18, 24 px), 13 px par défaut partout ; les glyphes ▾ ● × gardent leur taille propre.                                                |
| R2  | Trois hauteurs de contrôle : 28 px (compact), 32 px (formulaire), 40 px (barre d'adresse d'une requête et « Envoyer », le champ principal).                               |
| R3  | Rayons 2, 6, 10 px et forme ronde, rien d'autre.                                                                                                                          |
| R4  | Graisses 400 et 600 ; 700 pour les seules méthodes et marques d'état en chasse fixe.                                                                                      |
| R5  | Un seul bouton plein par vue, à la couleur d'accent ; l'accent ne sert qu'à l'action principale, à l'interrupteur, à la case cochée et au soulignement de l'onglet actif. |
| R6  | Un libellé de champ n'est jamais plus fort que sa valeur : `--text-small`, `--color-muted`.                                                                               |
| R7  | Des bordures plutôt que des ombres ; aucune carte, pastille ou décoration ajoutée.                                                                                        |
| R8  | Les tailles, rayons et hauteurs sont des variables de `src/styles.css`, avec leur usage en commentaire ; le design system les lit (`scripts/design-system.mjs`).          |

## 4. Hors périmètre

- Les fonctions, les libellés et la navigation : rien ne change.
- Les espacements : pas de variables dans ce lot (103 valeurs à ramener à une dizaine de pas).
- Les teintes de la maquette éloignées de tout jeton (texte moyen `#3a3732` et `#4a463f`, bordures
  `#ddd6cb`, bleus des contrats) : à revoir une à une, listées en 0.5-e.
- Les écrans de la maquette ne sont pas redessinés un par un.

## 5. Impacts

| Fichier                                         | Changement                                                                                     |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `src/styles.css`                                | Variables `--text-*`, `--radius-*`, `--control-*` ; texte à 13 px ; listes et cases habillées. |
| `src/components/**`                             | Tailles et rayons en variables ; hauteurs des boutons et des champs ; graisses 400 et 600.     |
| `scripts/design-system.mjs`                     | Échelles lues dans `styles.css` ; règles de base recopiées dans `components/bundle.css`.       |
| `docs/design-system/`                           | Composant `Field`, hauteurs, écarts résorbés.                                                  |
| `scripts/maquette-couleurs.py`                  | Aligne le bouton principal et les teintes proches des écrans de la maquette.                   |
| `scripts/maquette-sync.py`, `maquette-etat.yml` | Page 0.5 : hauteurs, champs ; écarts transverses mis à jour.                                   |
| `src-tauri/`, capacités, installeur             | Inchangés.                                                                                     |

## 6. Tâches

- [x] **T1** — Variables d'échelle dans `src/styles.css` ; texte de base à 13 px.
- [x] **T2** — Tailles, rayons et graisses des composants ramenés à l'échelle (56 fichiers).
- [x] **T3** — Hauteurs : `BaseButton`, `SearchInput`, `BaseMenu`, champs des écrans ; barre
      d'adresse à 40 px.
- [x] **T4** — Listes déroulantes et cases à cocher aux couleurs du design system.
- [x] **T5** — Onglets de l'éditeur visibles à 1280 px ; « + Réponse » en bouton secondaire ;
      libellés d'« Environnements ».
- [x] **T6** — Design system : échelles lues dans le code, composant `Field`, page 0.5.
- [x] **T7** — Maquette : bouton principal et teintes proches alignés dans les 38 pages d'écrans.

## 7. Vérifications

- [x] `npm run check`.
- [x] Tests e2e sur l'application réelle, thèmes clair et sombre : 36 tests sur 36.
- [x] Captures avant et après comparées, fenêtre de 1280 × 775 : accueil, Tous les projets,
      éditeur de requête (vue partagée), vérification en échec, environnements, run, contrat,
      catalogue, gestion des workspaces, menu d'environnement.
- [ ] À vérifier sous Windows (WebView2) : flèche des listes et coche des cases, dessinées en CSS.
- [ ] Zone étroite (280 px, Z4) : les onglets de l'éditeur passent à la ligne ; à regarder à
      l'écran.
