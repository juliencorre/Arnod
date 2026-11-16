# SectionHeading

En-tête d'une section d'un projet dans la barre latérale : icône, nom, nombre d'éléments, puis
ses actions.

Source : `src/components/ui/SectionHeading.vue` et `SectionIcon.vue` (aperçu statique recopié à
la main, styles du code).

## Quand l'employer

Chaque section d'un projet : « Contrats de service », « Référentiel de tests », puis les sections
à venir (tableau de bord, produits, exigences, mocks, templates, suites, campagnes, anomalies),
dont les icônes existent déjà (groupe « Icônes »). L'icône occupe la colonne de tête de la grille
`tree-*`, le nom s'aligne sur celui du projet.

## Ce que fournit l'appelant

- `icon` : le nom d'une icône de section ; `title` : le nom de la section ; `count` : le nombre
  d'éléments, masqué à zéro.
- Les actions dans l'emplacement par défaut, en général un « + » nommé par `aria-label`.

## À éviter

Un texte d'aide sous une section vide : une ligne d'ajout (« + Ajouter un contrat ») suffit.
