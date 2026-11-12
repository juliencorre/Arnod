# Dialog

Fenêtre modale sur l'élément `<dialog>` natif : titre, description, corps qui défile, pied avec
les boutons.

Source : `src/components/ui/BaseDialog.vue` (aperçu statique recopié à la main, styles du code).

## Quand l'employer

Une saisie courte qui interrompt le travail : nommer un projet, gérer les environnements,
confirmer l'envoi vers un environnement protégé. Panneau `color-surface`, ombre `shadow-dialog`
sur le voile `backdrop`. Largeur 36 rem, 46 rem avec `wide`.

## Ce que fournit l'appelant

- `open`, `title` (obligatoires) et `description` (une phrase, en `color-muted`).
- Le contenu dans l'emplacement par défaut ; les boutons dans l'emplacement `footer`, l'action
  principale à droite.
- L'événement `close` : Échap et le clic sur le voile le déclenchent sans fermer d'eux-mêmes ;
  l'appelant garde la main sur `open`.

## Clavier et accessibilité

Le focus entre dans la fenêtre et y reste ; Échap ferme. Le titre et la description nomment la
fenêtre (`aria-labelledby`, `aria-describedby`). Fenêtre basse : le titre et les boutons restent
visibles, seul le corps défile.

## À éviter

Une fenêtre pour confirmer une action qui se rattrape : une notification avec « Annuler » suffit.
