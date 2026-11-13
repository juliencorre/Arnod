# NaturePill

Pastille d'une nature d'étape (REST, puis les suivantes), aux couleurs de sa famille.

Source : `src/components/steps/NaturePill.vue` (aperçu statique recopié à la main, styles du code).

## Quand l'employer

Dans le catalogue des étapes, dans « Essayer une étape » et partout où une étape est nommée par sa
nature. Une famille a trois jetons : `color-family-<famille>-bg`, `-border` et `-text` (texte à
4,5:1 sur son fond et sur `color-surface`).

## Ce que fournit l'appelant

- `family`, la famille de la nature (libellé en police à chasse fixe) : `calls` (Appels), `data`
  (Données), `messages` (Messages), `jobs` (Traitements), `interface` (Interface), `code` (Code),
  `files` (Fichiers).
- `name` : le nom de la nature (« REST ») ; `size` : `sm` (20 px de haut, par défaut) ou `md`
  (24 px).

## À éviter

Une couleur de famille hors d'une pastille de nature : elles ne disent rien d'autre.
