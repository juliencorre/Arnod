# StatusMark

Marque de la dernière exécution d'une requête : ✓ réussie, ✕ en échec, rien si elle n'a jamais
été exécutée.

Source : `src/components/workbench/StatusMark.vue` (aperçu statique recopié à la main, styles du
code). L'aperçu montre aussi les méthodes HTTP de l'arborescence (`TreeRow.vue`).

## Quand l'employer

En fin de ligne d'une requête dans la barre latérale et dans les listes de résultats. ✓ en
`color-success`, ✕ en `color-danger` : la forme porte le sens, la couleur le renforce.

## Ce que fournit l'appelant

`status` : le statut de la dernière exécution (réussie ou non, avec sa date pour l'info-bulle).
Les lecteurs d'écran entendent « réussie » ou « en échec ». Le décompte d'un dossier ou d'un
projet passe par `StatusCount` (« réussies / exécutées »).
