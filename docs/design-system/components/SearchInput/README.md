# SearchInput

Champ de recherche qui filtre une liste pendant la frappe.

Source : `src/components/ui/SearchInput.vue` (aperçu statique recopié à la main, styles du code).

## Quand l'employer

Filtrer l'arborescence, une liste de projets ou de workspaces. Fond `color-surface`, bord
`color-border`, texte indicatif `color-subtle`.

## Ce que fournit l'appelant

- `v-model` : le texte cherché.
- `label` : le nom accessible, repris comme texte indicatif si `placeholder` manque
  (« Filtrer les requêtes »).

## Clavier

Échap vide le champ ; s'il est déjà vide, Échap continue son chemin (il ferme, par exemple, la
fenêtre qui contient le champ). Focus : contour `color-focus` à l'intérieur du bord.
