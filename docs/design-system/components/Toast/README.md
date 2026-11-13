# Toast

Notification en bas de la fenêtre, avec au plus une action : c'est elle qui rend une suppression
réversible (« Annuler »).

Source : `src/components/ui/ToastHost.vue` et le store `toasts` (aperçu statique recopié à la
main, styles du code).

## Quand l'employer

Annoncer le résultat d'une action qui ne se voit pas déjà à l'écran, ou offrir de la défaire. Fond
`color-toast-bg`, texte `color-toast-text`, action `color-toast-action`. Une erreur (`tone`
`error`) porte un filet `color-danger` à gauche.

## Ce que fournit l'appelant

Passer par le store `toasts` : un message (une phrase, les noms entre guillemets français), un ton,
et au plus une action avec son libellé. Une seule notification à la fois.

## Clavier et accessibilité

Zone `role="status"` (`aria-live="polite"`). Le survol ou le focus suspend la disparition ;
l'action est un vrai bouton.
