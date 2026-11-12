# Field

Champ de saisie, liste déroulante et case à cocher : trois contrôles natifs, habillés par les
règles de base de `src/styles.css`.

Source : `src/styles.css` (règles `select` et `input[type="checkbox"]`) ; les champs de texte
sont stylés par chaque écran sur le même gabarit (aperçu statique).

## Règles

- **Hauteur.** `control-md` (32 px) dans un formulaire, `control-sm` (28 px) dans une ligne de
  tableau ou une barre d'outils, `control-lg` (40 px) pour la seule barre d'adresse d'une requête.
- **Champ.** Fond `color-surface`, bord `color-border-strong`, rayon `radius-md`, texte
  `text-body`. Texte indicatif en `color-subtle`.
- **Libellé.** Au-dessus du champ, en `text-small` et `color-muted` : le libellé ne doit jamais
  être plus fort que la valeur.
- **Liste déroulante.** Même gabarit que le champ ; la flèche est dessinée en CSS. Un écran qui
  change le fond d'une liste écrit `background-color`, pas `background`, pour garder la flèche.
- **Case à cocher.** 16 px, rayon `radius-sm` ; cochée : fond `color-accent` et coche
  `color-accent-text` (7:1), comme l'interrupteur. Pour un réglage qui s'applique tout de suite,
  préférer `Toggle`.
- **Focus.** Contour de 2 px en `color-focus`, à l'intérieur du bord.

## À éviter

Une case ou une liste au style du système : elles sortent de la palette (bleu, gris) dans les deux
thèmes.
