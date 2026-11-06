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

## Fonctionnement

- `app.ts` crée un bac à sable (dossier personnel `HOME` et données `XDG_DATA_HOME`
  temporaires), lance `tauri-driver` avec cet environnement, puis ouvre l'application par
  une session WebDriver classique (WebKitWebDriver ne gère pas WebDriver BiDi).
- `arnod.e2e.ts` enchaîne les étapes d'un utilisateur : premier lancement, création de
  projets, workspaces, ajout et retrait avec « Annuler », gestion, référentiel, redémarrage
  (W2), dossier supprimé hors d'Arnod (P10), dossier projet existant (P12). Chaque étape
  vérifie aussi le disque (`arnod.yml`, `arnod.json`).
- Les captures sont enregistrées dans `e2e/artifacts/` (ignoré par Git) et publiées par la CI.

## Limites

Le sélecteur de dossier natif (« Changer », « Ajouter un dossier existant… ») ne se pilote
pas par WebDriver. Le choix de l'emplacement est couvert par les tests Vitest ; l'inscription
d'un dossier existant est testée en appelant la vraie commande Rust depuis la page.
