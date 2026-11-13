# Button

Bouton de l'application, en quatre variantes : principal, secondaire, danger et lien.

Source : `src/components/ui/BaseButton.vue` (aperçu statique recopié à la main, styles du code).

## Quand l'employer

- `primary` : l'action principale d'une vue ou d'une fenêtre, une seule à la fois (« Envoyer »,
  « Créer »). Fond `color-accent`, texte `color-accent-text`.
- `secondary` (par défaut) : les autres actions (« Annuler », « Importer »).
- `danger` : une action destructrice ; texte `color-danger`, bord rouge au survol.
- `link` : une action textuelle dans une liste ou une section (« + Ajouter un projet »).

## Ce que fournit l'appelant

- Le libellé dans le contenu : un verbe à l'infinitif, sans point final.
- `variant`, `type` (`button` ou `submit`) et `disabled`.

## Clavier et accessibilité

Vrai `<button>` : Entrée et Espace l'activent ; contour de focus `color-focus`. Un bouton désactivé
reste visible (opacité 0,55). Un bouton fait d'une seule icône porte un `aria-label`.

## À éviter

- Deux boutons principaux côte à côte.
- Une confirmation quand rien n'est perdu : préférer une notification avec « Annuler ».
