# ColorDot

Pastille de couleur d'un workspace, décorative.

Source : `src/components/ui/ColorDot.vue` (aperçu statique recopié à la main, styles du code).

## Quand l'employer

À côté du nom d'un workspace : sélecteur, menus, gestion des workspaces. Toujours avec le nom :
la couleur seule ne désigne rien (`aria-hidden`).

## Ce que fournit l'appelant

- `color` : `gray`, `orange`, `blue`, `green`, `purple`, `red`, `yellow` ou `teal` (jetons
  `color-ws-*`, libellés Gris, Orange, Bleu, Vert, Violet, Rouge, Jaune, Turquoise).
- `size` : `md` (14 px, coins de 4 px, par défaut) ou `sm` (10 px, coins de 3 px).
