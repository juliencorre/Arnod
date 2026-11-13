# Toggle

Interrupteur (`role="switch"`) pour un réglage qui s'applique tout de suite, avec son libellé.

Source : `src/components/ui/ToggleSwitch.vue` (aperçu statique recopié à la main, styles du code).

## Quand l'employer

Un réglage oui / non qui prend effet sans bouton « Enregistrer ». Allumé : fond `color-accent`.
Éteint : fond `color-chip`, bord `color-border-strong`. Pour un choix dans une liste, préférer un
menu à choix unique.

## Ce que fournit l'appelant

- `v-model` : l'état (booléen).
- Le libellé dans le contenu : ce qui est activé, sans « Activer ».
- `disabled` : un interrupteur allumé et non modifiable reste lisible (opacité 0,8).

## Clavier et accessibilité

Vrai `<button>` : Espace et Entrée basculent. Le libellé est relié par `for` (un clic sur le texte
bascule) et nomme le bouton par `aria-labelledby`.
