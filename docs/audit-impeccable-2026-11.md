# Audit de design avec Impeccable

**Date** 2026-11-26 · **Lot** I du [plan de qualité](plan-qualite-mvp.md) (T36) · **Outil**
[Impeccable](https://github.com/pbakaus/impeccable) 4.11.1 (Apache 2.0), moteur 0.1.31

Impeccable est une skill de design pour agents de code : un vocabulaire de commandes (`audit`,
`critique`, `polish`…) et un détecteur de 62 règles écrites, sans IA. Il est installé dans le dépôt
(`.claude/skills/impeccable/`) et lit `PRODUCT.md` et `DESIGN.md`. Arnod est une interface d'outil :
le mode retenu est **Operate** (lisibilité, constance et densité avant l'expression).

## 1. Méthode

| Passage              | Portée                                       | Comment                                                  |
| -------------------- | -------------------------------------------- | -------------------------------------------------------- |
| Sources              | 136 fichiers `.vue`, `.ts`, `.css` de `src/` | `npm run design:detect` (aussi dans `npm run check`, CI) |
| Aperçu rendu, clair  | 54 scénarios de `tests/apercu/`, 1280 × 775  | `node scripts/impeccable-apercu.mjs`                     |
| Aperçu rendu, sombre | les mêmes, thème sombre                      | `node scripts/impeccable-apercu.mjs --sombre`            |
| Constats de l'audit  | DE1 à DE10, AC5, AC6, ER3, ER4 du lot I      | [`audit-2026-11-mvp.md`](audit-2026-11-mvp.md)           |

Le passage sur les sources ne voit que les règles écrites (barre de sélection sur le côté, taille
hors de l'échelle de `DESIGN.md`…) ; le contraste réel, les titres sautés ou le texte recouvert
demandent une page rendue, d'où le passage sur l'aperçu, hors CI (il faut Chromium).

## 2. Résultat

| Dimension (barème d'Impeccable, 0 à 4, estimé) | Avant | Après | Ce qui a changé                                                                                      |
| ---------------------------------------------- | ----- | ----- | ---------------------------------------------------------------------------------------------------- |
| Accessibilité                                  | 2     | 3     | Textes et bords de contrôle à leur seuil, titres sans niveau sauté, pas de contrôle dans un contrôle |
| Thème (clair et sombre)                        | 2     | 4     | Plus aucun texte sous son seuil en thème sombre ; jetons employés à leur place, vérifiés             |
| Densité à 1280 × 775                           | 2     | 3     | Onglets et barre des étapes sur une ligne, adresse de 274 à 307 px                                   |
| Anti-patterns                                  | 3     | 4     | Plus de barre de sélection sur le côté ni de couleur d'erreur pour un message neutre                 |
| Performance                                    | —     | —     | Hors de ce passage : bancs des lots F et G                                                           |

Passage sur l'aperçu, constats bloquants (hors avis) : 87 avant, 74 après en thème clair ; la
plupart des restants sont le texte indicatif des champs (DI7, à trancher) et des titres sautés
dans des scénarios capturés avant leur correction (DI5).

## 3. Constats d'Impeccable

En plus des constats du lot I, le détecteur en a relevé sept. Les six premiers sont corrigés dans le
lot I, chacun avec son test ; le septième attend une décision.

| ID  | Constat                                                                                                                                    | Règle           | Correction                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------ | --------------- | ------------------------------------------------------------------------------------------- |
| DI1 | Barre de la sélection multiple illisible en thème sombre : « Supprimer » à 1,8:1 (fond `--color-text`, texte accent)                       | low-contrast    | Jetons des notifications `--color-toast-*`, nouveau `--color-toast-danger` (7,5:1 et 5,3:1) |
| DI2 | Méthode d'une opération sans requête estompée par l'opacité : 2,8:1                                                                        | low-contrast    | Estompée par `--color-muted`, plus d'opacité sur du texte                                   |
| DI3 | Texte indicatif au gris propre au moteur (#757575, 3,5:1 en thème sombre) dans quatre champs                                               | low-contrast    | Règle `::placeholder` commune dans `styles.css`                                             |
| DI4 | Sélection dessinée par une barre orange sur le côté (sélection multiple, « Copier ou déplacer ») ; réponse attendue ouverte en fond neutre | side-tab        | Fond `--color-accent-soft`, la forme unique d'une sélection (DE7)                           |
| DI5 | Niveau de titre sauté (h1 puis h3) dans l'onglet d'une requête : 24 scénarios                                                              | skipped-heading | Titres h2 réservés aux lecteurs d'écran : « Requête … » et « Résultat »                     |
| DI6 | Explication d'un conflit en lignes de 117 caractères                                                                                       | line-length     | 72 caractères au plus                                                                       |
| DI7 | Texte indicatif des champs et numéros de ligne en `--color-subtle` : 3,7:1 en clair, 4,3:1 en sombre, sous les 4,5:1 du texte              | low-contrast    | Q12 du plan (lot J) : foncer `--color-subtle` ou passer ces textes en `--color-muted`       |

## 4. Écartés, avec leur raison

| Règle                          | Où                                     | Raison                                                                     |
| ------------------------------ | -------------------------------------- | -------------------------------------------------------------------------- |
| text-occlusion                 | 1.3-c (Ctrl+Tab)                       | La fenêtre des onglets récents est posée au-dessus de la page, c'est voulu |
| nested-cards                   | Accueil, champ « Dossier »             | Un champ avec son bouton « Changer », pas une carte                        |
| design-system-font-size        | Glyphes ● ▾ ×, racine à 16 px          | Exceptions des échelles (CLAUDE.md), déclarées à côté de la valeur         |
| straggler, flat-type-hierarchy | Plusieurs paragraphes, titres de 11 px | Avis, sans effet sur la lecture d'un outil dense                           |

## 5. Ce qui reste, et où

- Onglets de l'éditeur et de la réponse aux flèches, cibles de 24 px, zoom, mouvement réduit :
  AC3, AC4, AC7, AC8, lot J.
- Erreur d'envoi avec son action sous la réponse : ER6, lot J.
- Texte indicatif à 4,5:1 : DI7, décision Q12.
- Contraste réel de chaque écran, titres et texte recouvert : relancer
  `node scripts/impeccable-apercu.mjs` (deux thèmes) à chaque lot qui touche l'interface.
