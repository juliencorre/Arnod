Arnod est une application de bureau de test d'API (Windows et Linux, sans droits administrateur),
pour des testeurs et des développeurs qui passent la journée dedans. L'interface est dense, calme
et entièrement utilisable au clavier : des fonds neutres chauds, une seule couleur d'action, du
texte en police système. Ce design system est tiré du code d'Arnod ; la page 0.5 de la maquette
l'affiche.

## Principes

- **Le contenu d'abord.** Les requêtes, réponses et résultats occupent la place ; l'interface reste
  en retrait (`color-bg`, `color-surface`, `color-sidebar`, bordures `color-border`).
- **Une seule couleur d'action.** `color-accent` ne sert qu'au bouton principal et à l'interrupteur
  allumé ; tout le reste se dit par le texte, les bordures et les états.
- **Jamais la couleur seule.** Un statut porte toujours une forme ou un mot : ✓ réussi, ✕ en échec,
  « réussies / exécutées » ; une pastille de workspace est toujours à côté du nom.
- **Tout au clavier.** Chaque action glissée ou survolée a son équivalent au clavier ou dans un
  menu ; le focus est toujours visible (`color-focus`).
- **Rien à confirmer quand rien n'est perdu.** Une suppression se fait tout de suite et se rattrape
  par la notification (« Annuler ») ; une modification s'enregistre seule.

## Contenu et ton

- Tout est en français, phrases courtes, sans jargon interne : on parle de requête, contrat,
  endpoint, test, étape, environnement, projet, workspace.
- Un bouton dit ce qu'il fait, à l'infinitif : « Envoyer », « Ajouter un contrat », « Annuler ».
  Une ligne d'ajout commence par « + » : « + Nouvelle requête », « + Ajouter un projet ».
- Les noms cités sont entre guillemets français : n requêtes créées sous le contrat « Boutique ».
- Un état tient en peu de mots : « ✓ Enregistré », « Non enregistré · Réessayer ».
- Une erreur dit la cause et ce qui la règle, sans excuse.
- Pas d'emoji. Glyphes autorisés : ✓ ✕ ▸ ⋯ ⤢ · « ».
- Les valeurs techniques (méthodes, chemins, raccourcis, compteurs) sont en police à chasse fixe.

## Couleurs

Deux thèmes, `light` (Clair) et `dark` (Sombre), qui suivent le système. Chaque jeton existe dans
les deux thèmes ; les contrastes sont vérifiés par `tests/theme.spec.ts` (texte 4,5:1, indications
et focus 3:1). Un composant n'écrit jamais une couleur en dur : il prend un jeton.

- **Fonds.** `color-bg` pour la zone principale, `color-surface` pour ce qui est posé dessus
  (cartes, champs, menus, fenêtres), `color-sidebar` pour la barre latérale et les barres
  d'onglets, `color-chip` pour les badges neutres, `color-accent-soft` pour la ligne sélectionnée.
- **Textes.** `color-text` pour le texte principal, `color-muted` pour le secondaire,
  `color-subtle` seulement pour les indications et l'état désactivé (3:1, jamais pour un texte à
  lire), `color-link` pour les liens et actions textuelles.
- **États.** `color-success` et `color-danger`, avec leurs fonds `color-success-soft` et
  `color-danger-soft` pour une ligne ou une vérification. Toujours accompagnés de ✓ ou ✕.
- **Action.** `color-accent` en fond, `color-accent-text` dessus (foncé dans les deux thèmes,
  7,2:1 en clair), `color-accent-hover` au survol.
- **Notifications.** `color-toast-bg`, `color-toast-text`, `color-toast-action`.
- **Workspaces.** `color-ws-gray` à `color-ws-teal` : pastilles décoratives (`ColorDot`).
- **Familles d'étapes.** `color-family-<famille>-bg`, `-border` et `-text` pour les pastilles de
  nature (`NaturePill`) : Appels, Données, Messages, Traitements, Interface, Code, Fichiers.
- **Méthodes HTTP** dans l'arborescence : GET en `color-success`, POST, PUT et PATCH en
  `color-link`, DELETE en `color-danger`, les autres en `color-muted` ; style `mono-caption`.

## Typographie

Police système uniquement (`font-sans`) : la politique de sécurité de l'application interdit les
polices distantes. Les données (méthodes, chemins, raccourcis, compteurs, pastilles) sont en
`font-mono`. Hauteur de ligne 1,5.

| Style             | Taille | Graisse | Usage                                       |
| ----------------- | ------ | ------- | ------------------------------------------- |
| `text-display`    | 24 px  | 600     | Titre d'une vue entière                     |
| `text-title`      | 18 px  | 600     | Titre d'un onglet ou d'une fenêtre modale   |
| `text-body-large` | 15 px  | 400     | Champ principal (adresse d'une requête)     |
| `text-body`       | 13 px  | 400     | Texte courant de l'interface                |
| `text-small`      | 12 px  | 400     | Texte secondaire, pastilles, statuts        |
| `text-caption`    | 11 px  | 600     | Compteurs, en-têtes de section              |
| `mono-caption`    | 11 px  | 700     | Méthodes HTTP, en-têtes de menu (capitales) |
| `mono-small`      | 12 px  | 400     | Raccourcis clavier, chemins, valeurs        |

Graisses : 400 pour le texte, 600 pour les titres, libellés de bouton et en-têtes, 700 pour les
méthodes et les marques d'état.

## Espacements, rayons et ombres

- Pas d'espacement : `space-2` à `space-32`. `space-8` est le plus employé (rembourrage vertical
  des boutons et des champs), `space-24` rembourre une fenêtre modale.
- Barre latérale : toute ligne suit la grille `tree-pad`, `tree-indent`, `tree-lead`, `tree-gap`
  (colonne de tête de largeur fixe, libellé après) au lieu de marges propres.
- Rayons : `radius-sm` (2 px) pour les petites marques, `radius-md` (6 px) pour les boutons,
  champs et lignes, `radius-lg` (10 px) pour les menus, notifications, cartes et fenêtres,
  `radius-pill` pour les formes rondes.
- Ombres : `shadow-popover` pour les menus et notifications, `shadow-dialog` pour les fenêtres
  modales, posées sur le voile `backdrop`. Les autres éléments n'ont pas d'ombre : une bordure
  `color-border` les sépare.

## États et interactions

- Focus au clavier : contour de 2 px plein en `color-focus`, décalé de 2 px (vers l'intérieur
  dans un menu ou un champ).
- Désactivé : opacité 0,55 et curseur interdit ; un élément de menu désactivé passe en
  `color-subtle`.
- Survol : bouton principal en `color-accent-hover`, bouton secondaire bordé de
  `color-border-strong`, lien souligné.
- Transitions de 120 ms sur les fonds et bordures ; rien d'autre ne bouge.

## Iconographie

Icônes de trait, 16 px, trait de 1,4 px, bouts et angles arrondis, en couleur du texte
(`currentColor`, `color-muted` dans les en-têtes de section) ; décoratives (`aria-hidden`), toujours
à côté d'un libellé. Le groupe « Icônes » en donne les fichiers. Les glyphes ✓ ✕ ▸ servent de
marques d'état et de flèches de sous-menu.

## Composants

Les aperçus sont des rendus statiques : les composants sont écrits en Vue, et leurs styles sont
recopiés tels quels dans `components/bundle.css`. Dans l'application, réutiliser ces composants au
lieu d'en écrire de nouveaux.

- **Button** (`BaseButton`) : principal, secondaire, danger, lien.
- **Toggle** (`ToggleSwitch`) : interrupteur `role="switch"` avec son libellé.
- **SearchInput** : champ de recherche, Échap le vide.
- **Menu** (`BaseMenu`) : menu accessible au clavier, avec choix, nuanciers, mentions, raccourcis.
- **Dialog** (`BaseDialog`) : fenêtre modale sur `<dialog>` natif.
- **Toast** (`ToastHost`) : notification avec une action (« Annuler »).
- **SectionHeading** : en-tête d'une section de la barre latérale (icône, nom, nombre, « + »).
- **ColorDot** : pastille de couleur d'un workspace.
- **StatusMark** : ✓ ou ✕ de la dernière exécution.
- **NaturePill** : pastille de nature d'étape, couleur de sa famille.

## Écarts à résorber

Ce design system est la référence de la maquette et du code. Décisions du 12 novembre 2026 :

- **Couleurs : le code fait foi.** Bouton principal orange `color-accent`, texte foncé. La maquette
  dessine encore son bouton principal en rouille `#b4561d`, texte blanc, et écrit ses couleurs à
  la main, la moitié seulement sur des valeurs de jetons : elle passe aux jetons. La page 0.5 de
  la maquette compte ces teintes et donne le jeton qui remplace chacune.
- **Tailles de texte : l'échelle de la maquette fait foi** (11, 12, 13, 15, 18, 24 px). Le code
  emploie aussi 10, 14 (49 fois), 16, 17, 20 et 28 px : il s'aligne.
- **Rayons : l'échelle de la maquette fait foi** (2, 6, 10 px et formes rondes). Le code emploie
  aussi 8 px (boutons, champs) et 3, 4, 5, 12, 14 px : il s'aligne.

Ces écarts sont suivis dans `docs/maquette-etat.yml` (écarts transverses).

## Tenue à jour

Les jetons, les styles des composants et les icônes sont générés depuis le code par
`node scripts/design-system.mjs` ; `npm run check` échoue tant que `docs/design-system/` n'est pas
à jour. Un jeton ajouté à `src/styles.css` doit recevoir sa note d'usage dans ce script. Ce texte,
les guides et les aperçus des composants s'écrivent à la main dans `docs/design-system/`.

Non synchronisé : aucune police à fournir (police système) ; composants non exécutables (Vue,
aperçus statiques) ; les tailles et rayons hors échelle restent dans le code jusqu'à leur
alignement.
