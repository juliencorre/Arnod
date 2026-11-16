# Tests e2e

Parcours de bout en bout sur **l'application réelle** : binaire debug d'Arnod (cœur Rust,
interface embarquée), piloté par WebDriver via [`tauri-driver`](https://v2.tauri.app/develop/tests/webdriver/)
et `WebKitWebDriver`. Rien n'est simulé : les projets sont créés sur disque et l'état est
écrit dans un vrai `arnod.json`.

## Lancer

Linux uniquement (`tauri-driver` ne gère pas macOS ; sous Windows, `dirs` ignore les
variables d'environnement qui isolent le bac à sable).

```sh
sudo apt install webkit2gtk-driver xvfb   # pilote WebKit et serveur X virtuel
cargo install tauri-driver --locked
npm run build:e2e                         # tauri build --debug --no-bundle
xvfb-run -a npm run test:e2e              # thème clair
ARNOD_E2E_THEME=dark xvfb-run -a npm run test:e2e   # thème sombre (GTK_THEME=Adwaita:dark)
```

Après une modification de l'interface, relancer `npm run build:e2e` : l'interface est
embarquée dans le binaire.

`tauri-driver` est lancé par son chemin absolu, sans recherche dans `PATH` : celui de
`cargo install` (`$CARGO_HOME/bin`, par défaut `~/.cargo/bin`). Pour un binaire installé
ailleurs : `ARNOD_E2E_TAURI_DRIVER=/chemin/vers/tauri-driver`.

## Fonctionnement

- `app.ts` crée un bac à sable (dossier personnel `HOME` et données `XDG_DATA_HOME`
  temporaires), lance `tauri-driver` avec cet environnement, puis ouvre l'application par
  une session WebDriver classique (WebKitWebDriver ne gère pas WebDriver BiDi).
- `arnod.e2e.ts` enchaîne les étapes d'un utilisateur : premier lancement (accueil, formulaire
  REST), création de projets, workspaces, ajout et retrait avec « Annuler », gestion, référentiel, redémarrage
  (W2), dossier supprimé hors d'Arnod (P10), dossier projet existant (P12), projet oublié puis
  rétabli (P11) ; puis zones et fenêtres : onglets, division, séparation au clavier, zone ouverte
  dans sa fenêtre puis rattachée, barre latérale détachée qui pilote la fenêtre principale,
  restauration au redémarrage ; contrats de services contre un serveur HTTP local ; catalogue
  des étapes et essais REST gardés dans « Non classés » sous leur contrat déduit ; enfin le premier
  usage jusqu'au run d'un dossier : vérification en échec ignorée, résultat par dossier, statuts de
  la barre latérale relus au redémarrage. Chaque étape
  vérifie aussi le disque (`opencollection.yml`, `arnod.json`,
  `windows.json`).
- Plusieurs fenêtres : `getWindowHandles()` les liste et `switchToWindow()` passe de l'une à
  l'autre ; un bouton qui n'apparaît qu'au survol est cliqué depuis la page (`clickHidden`).
- Les sélecteurs sont stricts (WebdriverIO 10) : `$()` échoue s'il trouve plusieurs éléments,
  y compris des éléments masqués. Les fenêtres (`<dialog>`) restent dans le DOM une fois
  fermées : viser `main`, `aside` ou `dialog[open]` plutôt qu'un sélecteur global.
- Les captures sont enregistrées dans `e2e/artifacts/` (ignoré par Git) et publiées par la CI.

## Limites

Le sélecteur de dossier natif (« Changer », « Importer un projet existant… ») ne se pilote
pas par WebDriver. Le choix de l'emplacement est couvert par les tests Vitest ; l'inscription
d'un dossier existant est testée en appelant la vraie commande Rust depuis la page.
