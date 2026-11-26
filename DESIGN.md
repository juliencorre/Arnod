---
name: Arnod
description: "Application de bureau de test d'API (remplaçant de SoapUI et Bruno) : interface dense, en mode Operate."
colors:
  bg: "#fbfaf8"
  surface: "#ffffff"
  sidebar: "#f4f2ee"
  chip: "#efebe4"
  accent-soft: "#fdf1e1"
  border: "#e6e2db"
  border-strong: "#cfc9bf"
  border-control: "#8f8a83"
  text: "#1d1b18"
  muted: "#6b665e"
  subtle: "#8b857b"
  link: "#9a5a0c"
  success: "#2f7a4b"
  success-soft: "#e7f4ec"
  danger: "#b42318"
  danger-soft: "#fdeceb"
  accent: "#e8962e"
  accent-hover: "#d9822a"
  accent-text: "#1d1b18"
  focus: "#9a5a0c"
  toast-bg: "#1d1b18"
  toast-text: "#fbfaf8"
  toast-action: "#f0a54a"
  toast-danger: "#ff8a80"
  ws-gray: "#a39d93"
  ws-orange: "#e8962e"
  ws-blue: "#3f7fd6"
  ws-green: "#3f9a5a"
  ws-purple: "#8a63d2"
  ws-red: "#d6564b"
  ws-yellow: "#d4ad2a"
  ws-teal: "#2f9c95"
  family-calls-bg: "#ebf7ff"
  family-calls-border: "#a7c8ea"
  family-calls-text: "#194f81"
  family-data-bg: "#e7faf9"
  family-data-border: "#96d1d0"
  family-data-text: "#005d5e"
  family-messages-bg: "#f7f2ff"
  family-messages-border: "#cabce6"
  family-messages-text: "#563e7b"
  family-jobs-bg: "#fef3e7"
  family-jobs-border: "#debe9a"
  family-jobs-text: "#6f4100"
  family-interface-bg: "#fff0f9"
  family-interface-border: "#e0b6d0"
  family-interface-text: "#70355e"
  family-code-bg: "#fff1ed"
  family-code-border: "#e9b6ad"
  family-code-text: "#7a342b"
  family-files-bg: "#f6f7e7"
  family-files-border: "#c6c89b"
  family-files-text: "#515100"
  code-string: "#2a6e44"
  code-number: "#1f5fa8"
  code-keyword: "#7a3fb0"
  code-tag: "#a1361d"
  code-attr: "#875300"
  match: "#fcebc6"
  backdrop: "rgba(29, 27, 24, 0.32)"
typography:
  text-display:
    fontFamily: "system-ui, -apple-system, \"Segoe UI\", Roboto, Ubuntu, Cantarell, \"Noto Sans\", \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "24px"
    fontWeight: 600
    lineHeight: 1.5
  text-title:
    fontFamily: "system-ui, -apple-system, \"Segoe UI\", Roboto, Ubuntu, Cantarell, \"Noto Sans\", \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.5
  text-body-large:
    fontFamily: "system-ui, -apple-system, \"Segoe UI\", Roboto, Ubuntu, Cantarell, \"Noto Sans\", \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.5
  text-body:
    fontFamily: "system-ui, -apple-system, \"Segoe UI\", Roboto, Ubuntu, Cantarell, \"Noto Sans\", \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
  text-small:
    fontFamily: "system-ui, -apple-system, \"Segoe UI\", Roboto, Ubuntu, Cantarell, \"Noto Sans\", \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
  text-caption:
    fontFamily: "system-ui, -apple-system, \"Segoe UI\", Roboto, Ubuntu, Cantarell, \"Noto Sans\", \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1.5
  mono-caption:
    fontFamily: "ui-monospace, \"Cascadia Mono\", Consolas, \"DejaVu Sans Mono\", monospace"
    fontSize: "11px"
    fontWeight: 700
    lineHeight: 1.5
  mono-small:
    fontFamily: "ui-monospace, \"Cascadia Mono\", Consolas, \"DejaVu Sans Mono\", monospace"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
rounded:
  sm: "2px"
  md: "6px"
  lg: "10px"
  pill: "999px"
spacing:
  space-2: "2px"
  space-4: "4px"
  space-6: "6px"
  space-8: "8px"
  space-10: "10px"
  space-12: "12px"
  space-16: "16px"
  space-20: "20px"
  space-24: "24px"
  space-32: "32px"
  tree-pad: "6px"
  tree-indent: "20px"
  tree-lead: "40px"
  tree-gap: "8px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-text}"
    typography: "{typography.text-body}"
    rounded: "{rounded.md}"
    padding: "0 12px"
    height: "32px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    padding: "0 12px"
    height: "32px"
  button-link:
    textColor: "{colors.link}"
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    typography: "{typography.text-body}"
    rounded: "{rounded.md}"
    padding: "0 12px"
    height: "32px"
  address-bar:
    typography: "{typography.text-body-large}"
    height: "40px"
  chip:
    backgroundColor: "{colors.chip}"
    textColor: "{colors.muted}"
    typography: "{typography.text-small}"
    rounded: "{rounded.pill}"
  row-selected:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.text}"
  toast:
    backgroundColor: "{colors.toast-bg}"
    textColor: "{colors.toast-text}"
    rounded: "{rounded.lg}"
---

<!-- Généré par scripts/design-system.mjs depuis src/styles.css : ne pas modifier à la main. -->

# Arnod

## Overview

Outil de test d'API utilisé des heures durant, clavier en main : le mode est **Operate**. La
lisibilité, la constance et la densité priment sur l'expression ; la marque tient dans les
détails précis (accent orange, statuts ✓ / ✕, grille de la barre latérale). La maquette
(Claude Design) et `docs/design-system/` sont la référence ; ce fichier les résume pour les
agents. Thème clair et sombre suivant le système, police système (la CSP interdit toute police
distante).

## Colors

Une seule couleur d'action, `accent` (bouton principal, interrupteur allumé). Texte à 4,5:1,
bords de contrôle et focus à 3:1 dans les deux thèmes, vérifiés par `tests/theme.spec.ts`. Un
statut ne passe jamais par la couleur seule.

| Jeton | Clair | Sombre | Emploi |
| --- | --- | --- | --- |
| `--color-bg` | `#fbfaf8` | `#1a1917` | Fond de la zone principale. Texte : color-text, color-muted. |
| `--color-surface` | `#ffffff` | `#23211e` | Cartes, champs, menus, fenêtres modales, éléments posés sur color-bg. Texte : color-text, color-muted. |
| `--color-sidebar` | `#f4f2ee` | `#201e1b` | Barre latérale et barres d'onglets. Texte : color-text, color-muted. |
| `--color-chip` | `#efebe4` | `#2e2b27` | Badges et pastilles neutres (« par défaut »), fond de l'interrupteur éteint. |
| `--color-accent-soft` | `#fdf1e1` | `#3a2c1a` | Ligne sélectionnée ou mise en évidence, élément actif d'un menu. Texte : color-text. |
| `--color-border` | `#e6e2db` | `#36322d` | Bordure des champs, cartes, menus et fenêtres ; séparateurs. |
| `--color-border-strong` | `#cfc9bf` | `#4a453e` | Séparateurs marqués, bord d'un champ de recherche, survol d'un bouton. |
| `--color-border-control` | `#8f8a83` | `#7d7871` | Bord des cases à cocher, des boutons radio et de l'interrupteur éteint : 3:1 au moins sur color-bg, color-surface et color-sidebar (WCAG 1.4.11, vérifié par tests/theme.spec.ts). |
| `--color-text` | `#1d1b18` | `#ede9e3` | Texte principal, 4,5:1 sur color-bg, color-surface, color-sidebar, color-chip et color-accent-soft (vérifié par tests/theme.spec.ts). |
| `--color-muted` | `#6b665e` | `#b0a99e` | Texte secondaire (descriptions, compteurs, en-têtes de menu, icônes des sections), 4,5:1 sur les mêmes fonds que color-text. |
| `--color-subtle` | `#8b857b` | `#8a8378` | Indications, textes désactivés, texte indicatif des champs : 3:1 au moins sur color-bg, color-surface et color-sidebar. |
| `--color-link` | `#9a5a0c` | `#f0b46a` | Liens et actions textuelles (« + Ajouter un projet »), méthodes POST, PUT et PATCH, mentions de menu : 4,5:1 sur color-bg, color-surface, color-sidebar. |
| `--color-success` | `#2f7a4b` | `#6cc28b` | Réussite : ✓ d'un statut, méthode GET, vérification réussie ; 4,5:1 sur color-bg, color-surface, color-sidebar et color-success-soft. |
| `--color-success-soft` | `#e7f4ec` | `#1f3327` | Fond d'une vérification ou d'une ligne de run réussie. |
| `--color-danger` | `#b42318` | `#ff8a80` | Échec, erreur, action destructrice, méthode DELETE ; 4,5:1 sur color-bg, color-surface, color-sidebar et color-danger-soft. |
| `--color-danger-soft` | `#fdeceb` | `#3d2321` | Fond d'une vérification ou d'une ligne de run en échec. |
| `--color-accent` | `#e8962e` | `#eda24c` | Seule couleur d'action : fond du bouton principal, interrupteur allumé. Texte posé dessus : color-accent-text. |
| `--color-accent-hover` | `#d9822a` | `#f3b46a` | Bouton principal survolé. |
| `--color-accent-text` | `#1d1b18` | `#1d1b18` | Texte sur color-accent et color-accent-hover, 4,5:1 dans les deux thèmes. |
| `--color-focus` | `#9a5a0c` | `#f0b46a` | Contour de focus au clavier : 2 px plein, décalé de 2 px ; 3:1 sur color-bg, color-surface et color-sidebar. |
| `--color-toast-bg` | `#1d1b18` | `#3a3631` | Fond des notifications (sombre en thème clair, relevé en thème sombre). |
| `--color-toast-text` | `#fbfaf8` | `#ede9e3` | Texte des notifications, 4,5:1 sur color-toast-bg. |
| `--color-toast-action` | `#f0a54a` | `#f0b46a` | Action d'une notification (« Annuler »), 4,5:1 sur color-toast-bg. |
| `--color-toast-danger` | `#ff8a80` | `#ff8a80` | Action destructrice posée sur color-toast-bg (« Supprimer » de la barre de sélection), 4,5:1 sur ce fond. |
| `--color-ws-gray` | `#a39d93` | `#8f887e` | Pastille du workspace « Gris » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-orange` | `#e8962e` | `#eda24c` | Pastille du workspace « Orange » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-blue` | `#3f7fd6` | `#6b9be6` | Pastille du workspace « Bleu » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-green` | `#3f9a5a` | `#5cb878` | Pastille du workspace « Vert » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-purple` | `#8a63d2` | `#a585e3` | Pastille du workspace « Violet » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-red` | `#d6564b` | `#e8766c` | Pastille du workspace « Rouge » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-yellow` | `#d4ad2a` | `#e2c04a` | Pastille du workspace « Jaune » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-ws-teal` | `#2f9c95` | `#4bbab2` | Pastille du workspace « Turquoise » (ColorDot, menus) : décorative, toujours à côté du nom du workspace. |
| `--color-family-calls-bg` | `#ebf7ff` | `#1e2f41` | Famille « Appels » : fond de la pastille de nature (NaturePill). |
| `--color-family-calls-border` | `#a7c8ea` | `#3e6084` | Famille « Appels » : bord de la pastille de nature. |
| `--color-family-calls-text` | `#194f81` | `#a9d6ff` | Famille « Appels » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-family-data-bg` | `#e7faf9` | `#113434` | Famille « Données » : fond de la pastille de nature (NaturePill). |
| `--color-family-data-border` | `#96d1d0` | `#216a6a` | Famille « Données » : bord de la pastille de nature. |
| `--color-family-data-text` | `#005d5e` | `#90e2e1` | Famille « Données » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-family-messages-bg` | `#f7f2ff` | `#31293f` | Famille « Messages » : fond de la pastille de nature (NaturePill). |
| `--color-family-messages-border` | `#cabce6` | `#64547f` | Famille « Messages » : bord de la pastille de nature. |
| `--color-family-messages-text` | `#563e7b` | `#d9c6fe` | Famille « Messages » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-family-jobs-bg` | `#fef3e7` | `#3b2a17` | Famille « Traitements » : fond de la pastille de nature (NaturePill). |
| `--color-family-jobs-border` | `#debe9a` | `#77562f` | Famille « Traitements » : bord de la pastille de nature. |
| `--color-family-jobs-text` | `#6f4100` | `#f3c898` | Famille « Traitements » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-family-interface-bg` | `#fff0f9` | `#3b2634` | Famille « Interface » : fond de la pastille de nature (NaturePill). |
| `--color-family-interface-border` | `#e0b6d0` | `#784e6a` | Famille « Interface » : bord de la pastille de nature. |
| `--color-family-interface-text` | `#70355e` | `#f6bde2` | Famille « Interface » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-family-code-bg` | `#fff1ed` | `#402622` | Famille « Code » : fond de la pastille de nature (NaturePill). |
| `--color-family-code-border` | `#e9b6ad` | `#814e46` | Famille « Code » : bord de la pastille de nature. |
| `--color-family-code-text` | `#7a342b` | `#ffbeb2` | Famille « Code » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-family-files-bg` | `#f6f7e7` | `#2f3017` | Famille « Fichiers » : fond de la pastille de nature (NaturePill). |
| `--color-family-files-border` | `#c6c89b` | `#606130` | Famille « Fichiers » : bord de la pastille de nature. |
| `--color-family-files-text` | `#515100` | `#d4d699` | Famille « Fichiers » : texte de la pastille, 4,5:1 sur son fond et sur color-surface. |
| `--color-code-string` | `#2a6e44` | `#6cc28b` | Corps de réponse : chaînes JSON et valeurs d'attribut, 4,5:1 sur color-surface, color-bg et color-match. |
| `--color-code-number` | `#1f5fa8` | `#82b4f2` | Corps de réponse : nombres JSON, 4,5:1 sur les mêmes fonds. |
| `--color-code-keyword` | `#7a3fb0` | `#c9a8f7` | Corps de réponse : true, false et null, 4,5:1 sur les mêmes fonds. |
| `--color-code-tag` | `#a1361d` | `#f2a08d` | Corps de réponse : noms de balise XML et HTML, 4,5:1 sur les mêmes fonds. |
| `--color-code-attr` | `#875300` | `#e2c04a` | Corps de réponse : noms d'attribut XML et HTML, 4,5:1 sur les mêmes fonds. |
| `--color-match` | `#fcebc6` | `#45341a` | Valeur trouvée par le filtre JSONPath ; la correspondance courante ajoute un contour color-accent. |
| `--backdrop` | `rgba(29, 27, 24, 0.32)` | `rgba(0, 0, 0, 0.55)` | Voile derrière une fenêtre modale (::backdrop de BaseDialog). |

## Typography

Police système (`system-ui, -apple-system, "Segoe UI", Roboto, Ubuntu, Cantarell, "Noto Sans", "Helvetica Neue", Arial, sans-serif`) ; données, méthodes et chemins en chasse fixe (`ui-monospace, "Cascadia Mono", Consolas, "DejaVu Sans Mono", monospace`).
Texte courant à 13 px, graisses 400 et 600 seulement.

| Style | Taille | Graisse | Emploi |
| --- | --- | --- | --- |
| `text-display` | 24px | 600 | Titre d'une vue entière : Tous les projets, accueil, run. |
| `text-title` | 18px | 600 | Titre d'un onglet (contrat, catalogue) ou d'une fenêtre modale. |
| `text-body-large` | 15px | 400 | Champ principal (adresse d'une requête), titre d'un bloc. |
| `text-body` | 13px | 400 | Texte courant de l'interface : arborescence, champs, boutons, menus, panneaux. |
| `text-small` | 12px | 400 | Texte secondaire : pastilles, statuts, détails, légendes. |
| `text-caption` | 11px | 600 | Plus petite taille : compteurs, en-têtes de section, méthodes HTTP. |
| `mono-caption` | 11px | 700 | Méthodes HTTP de l'arborescence, compteurs, en-têtes de menu (capitales, espacement 0,05em). |
| `mono-small` | 12px | 400 | Raccourcis clavier et détails d'un menu, pastilles de nature, chemins et valeurs. |

## Layout

Barre latérale de projets, zones d'onglets redimensionnables (éditeur et résultat côte à côte
dès 1280 × 775), fenêtres détachées. Toute ligne de la barre latérale suit la grille
`--tree-*` : une colonne de tête de largeur fixe, le libellé après.

| Jeton | Valeur | Emploi |
| --- | --- | --- |
| `space-2` | 2px | Décalage du contour de focus ; marges fines. |
| `space-4` | 4px | Petits écarts : bouton lien, marge au-dessus d'un en-tête de section. |
| `space-6` | 6px | Écart icône-libellé d'un bouton, rembourrage des menus et de leurs lignes. |
| `space-8` | 8px | Rembourrage vertical des boutons et des champs ; écart le plus employé. |
| `space-10` | 10px | Écart icône-libellé des menus et des interrupteurs ; rembourrage horizontal d'une ligne de menu. |
| `space-12` | 12px | Rembourrage horizontal des champs ; écart des boutons d'un pied de fenêtre modale. |
| `space-16` | 16px | Rembourrage horizontal des boutons ; écart entre les blocs d'une fenêtre modale. |
| `space-20` | 20px | Écart entre le message et l'action d'une notification. |
| `space-24` | 24px | Rembourrage des fenêtres modales. |
| `space-32` | 32px | Marges des vues entières (Tous les projets, accueil d'un workspace). |
| `tree-pad` | 6px | Barre latérale : marge des lignes, à gauche et à droite. |
| `tree-indent` | 20px | Barre latérale : retrait d'un niveau de l'arborescence. |
| `tree-lead` | 40px | Barre latérale : colonne de tête (chevron, pastille ou icône), de largeur fixe. |
| `tree-gap` | 8px | Barre latérale : écart entre la colonne de tête et le libellé. |

## Elevation & Depth

À plat ; seules les surfaces posées au-dessus du contenu portent une ombre.

| Jeton | Emploi |
| --- | --- |
| `--shadow-popover` | Menus, sous-menus et notifications. |
| `--shadow-dialog` | Fenêtres modales (BaseDialog). |

## Shapes

| Jeton | Valeur | Emploi |
| --- | --- | --- |
| `--radius-sm` | 2px | Petites marques : pastille de couleur, nuancier d'un menu. |
| `--radius-md` | 6px | Boutons, champs, listes, lignes de menu et d'arborescence. |
| `--radius-lg` | 10px | Menus, notifications, cartes et fenêtres modales. |
| `--radius-pill` | 999px | Formes rondes : interrupteur, pastilles de nature et d'état. |
| `--control-sm` | 28px | Compact : barres d'outils, lignes de tableau, pastilles d'action. |
| `--control-md` | 32px | Courant : boutons, champs et listes d'un formulaire. |
| `--control-lg` | 40px | Barre d'adresse d'une requête et son bouton « Envoyer ». |

## Components

Réutiliser les composants de base plutôt que d'en écrire de nouveaux :

- Button : `src/components/ui/BaseButton.vue`
- Toggle : `src/components/ui/ToggleSwitch.vue`
- SearchInput : `src/components/ui/SearchInput.vue`
- Menu : `src/components/ui/BaseMenu.vue`
- Dialog : `src/components/ui/BaseDialog.vue`
- Toast : `src/components/ui/ToastHost.vue`
- SectionHeading : `src/components/ui/SectionHeading.vue`
- SectionIcon : `src/components/ui/SectionIcon.vue`
- ColorDot : `src/components/ui/ColorDot.vue`
- StatusMark : `src/components/workbench/StatusMark.vue`
- NaturePill : `src/components/steps/NaturePill.vue`

## Do's and Don'ts

### Do:

- Prendre tailles de texte, rayons et hauteurs dans `--text-*`, `--radius-*`, `--control-*`.
- Écrire chaque couleur en `var(--color-*)` ; un nouveau jeton est défini pour les deux thèmes,
  en `#rrggbb`, avec sa note d'usage dans `scripts/design-system.mjs`.
- Écrire un même état d'une seule façon : compteur en pastille, sélection en fond `accent-soft`.
- Donner à toute action glissée son équivalent au clavier ; focus visible en `--color-focus`.

### Don't:

- Plus d'un bouton plein par vue.
- Une police, une image ou une ressource distante (CSP stricte).
- `--color-accent-text` hors d'un fond `accent` ; `--color-subtle` pour un texte à lire.
- Une couleur, une taille ou un rayon écrits en dur dans un composant (sauf les glyphes ▾ ● ×).
