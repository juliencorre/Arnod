# Menu

Menu déroulant accessible au clavier (motif WAI-ARIA « menu »), avec en-tête, choix, nuanciers,
mentions, raccourcis, sous-menus et séparateurs.

Source : `src/components/ui/BaseMenu.vue` et `menu.ts` (aperçu statique recopié à la main, styles
du code).

## Quand l'employer

Toutes les actions secondaires d'un élément (« ⋯ », clic droit, Maj+F10) et les choix d'une liste
courte (environnement, couleur). Panneau `color-surface`, bord `color-border`, rayon 10 px, ombre
`shadow-popover` ; ligne active en `color-accent-soft`.

## Ce que fournit l'appelant

Une liste d'entrées : `label`, et selon le besoin `checked` (choix, ✓), `swatch` (couleur),
`tag` (mention, « protégé »), `detail` (valeur en police à chasse fixe), `hint` (seconde ligne),
`shortcut`, `tone: "danger"`, `disabled`, `separatorBefore`, `children` (sous-menu). Le
déclencheur est fourni par l'emplacement `trigger`, avec ses attributs.

## Clavier

Flèches haut et bas pour se déplacer, Début et Fin pour aller aux extrémités, Entrée ou Espace
pour choisir, flèche droite pour ouvrir un sous-menu et gauche pour en sortir, Échap pour fermer
et rendre le focus au déclencheur.

## À éviter

Un menu pour une seule action : un bouton suffit.
