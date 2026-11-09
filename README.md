# Arnod

[![Build](https://github.com/juliencorre/Arnod/actions/workflows/build.yml/badge.svg?branch=Develop)](https://github.com/juliencorre/Arnod/actions/workflows/build.yml?query=branch%3ADevelop)
[![Quality Gate](https://sonarcloud.io/api/project_badges/measure?project=juliencorre_Arnod&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=juliencorre_Arnod)
[![Sécurité](https://sonarcloud.io/api/project_badges/measure?project=juliencorre_Arnod&metric=security_rating)](https://sonarcloud.io/summary/overall?id=juliencorre_Arnod)
[![Fiabilité](https://sonarcloud.io/api/project_badges/measure?project=juliencorre_Arnod&metric=reliability_rating)](https://sonarcloud.io/summary/overall?id=juliencorre_Arnod)
[![Maintenabilité](https://sonarcloud.io/api/project_badges/measure?project=juliencorre_Arnod&metric=sqale_rating)](https://sonarcloud.io/summary/overall?id=juliencorre_Arnod)

Application de bureau de test d'API, destinée à remplacer SoapUI et Bruno.

> **Statut : projets et workspaces en onglets, fenêtres et zones.** Sur le squelette technique
> ([`PRD-squelette.md`](PRD-squelette.md)), Arnod crée et gère des projets et des
> workspaces selon [`docs/plan-projets-workspaces.md`](docs/plan-projets-workspaces.md), révisé
> par [`docs/plan-workspaces-onglets.md`](docs/plan-workspaces-onglets.md) : les workspaces sont
> des onglets, après l'onglet épinglé « Tous les projets ». Les requêtes s'ouvrent en onglets,
> dans des zones redimensionnables et des fenêtres interconnectées
> ([`docs/plan-fenetres-zones.md`](docs/plan-fenetres-zones.md)).

| Thème clair                                      | Thème sombre                                           |
| ------------------------------------------------ | ------------------------------------------------------ |
| ![Arnod, thème clair](docs/screenshot-linux.png) | ![Arnod, thème sombre](docs/screenshot-linux-dark.png) |

_Vue d'un workspace, capturée sur l'application réelle (WebKitGTK) par les tests e2e. Le thème
suit la préférence du système._

## Ce que fait Arnod aujourd'hui

Cas d'usage détaillés dans [`docs/plan-projets-workspaces.md`](docs/plan-projets-workspaces.md)
et [`docs/plan-workspaces-onglets.md`](docs/plan-workspaces-onglets.md), maquettes dans
[`docs/maquettes/`](docs/maquettes/).

| ID           | Comportement                                                                                                                               |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| P1, O8       | Premier lancement : seul l'onglet « Tous les projets » ; Arnod propose de créer le premier projet (`~/arnod/<nom>`), workspace facultatif. |
| P2, O8       | Nouveau projet depuis un workspace : rangé dans ce seul workspace, toujours visible dans « Tous les projets ».                             |
| P5, P10      | Ouverture d'un projet ; un dossier déplacé ou supprimé hors d'Arnod est signalé.                                                           |
| P6, P7       | Ajout d'un projet à d'autres workspaces, retrait d'un workspace avec « Annuler ».                                                          |
| O1, O12, P11 | « Tous les projets » : chaque projet une fois, dernière exécution, « Oublier » avec « Annuler », import d'un projet (`arnod.yml`).         |
| O2, R7       | Workspaces en onglets ; Arnod rouvre le dernier onglet utilisé, conservé dans `arnod.json`.                                                |
| O3–O6        | « + » crée un onglet ; clic droit (ou Maj+F10) : renommer, couleur, gérer, supprimer avec « Annuler ».                                     |
| O7           | Glisser une ligne de « Tous les projets » sur un onglet ajoute le projet à ce workspace.                                                   |
| W6, O10      | « Ajouter un projet » à un workspace : créer, importer, ou reprendre des projets existants.                                                |
| O11          | Gestion des workspaces : nom, couleur, projets, suppression avec « Annuler ».                                                              |
| F4           | Fermeture propre par le menu **Fichier › Quitter** (`Ctrl+Q`) ou la croix de la fenêtre.                                                   |
| N7           | Thème clair ou sombre selon le système, police système.                                                                                    |

### Fenêtres et zones

Cas F1 à F14 de [`docs/plan-fenetres-zones.md`](docs/plan-fenetres-zones.md) (maquette « Fenêtres
et zones »).

| Deux zones côte à côte                                      | Zone ouverte dans sa fenêtre                                          |
| ----------------------------------------------------------- | --------------------------------------------------------------------- |
| ![Deux zones, thème clair](docs/screenshot-zones.png)       | ![Fenêtre de contenu, thème clair](docs/screenshot-fenetre.png)       |
| ![Deux zones, thème sombre](docs/screenshot-zones-dark.png) | ![Fenêtre de contenu, thème sombre](docs/screenshot-fenetre-dark.png) |

| ID       | Comportement                                                                                                                           |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| F1       | Une requête choisie dans la barre latérale s'ouvre en onglet, une seule fois par fenêtre ; ×, Suppr ou `Ctrl+W` la ferme, enregistrée. |
| F2, F3   | Diviser : glisser un onglet sur le bord droit ou bas d'une zone, ◫, ou le menu de l'onglet (clic droit, Maj+F10) ; au plus 4 zones.    |
| F4, F5   | Séparations et barre latérale redimensionnables : taille affichée, double-clic pour des parts égales, flèches ou `Alt+Maj+flèches`.    |
| F6       | Résultat d'une requête à droite ou dessous ; en automatique, il passe dessous quand la zone est étroite.                               |
| F7       | Menu **Fenêtre › Dispositions** : 1 zone, 2 colonnes, 2 lignes, 3 ou 4 zones, onglets conservés.                                       |
| F8, F12  | ⧉ ou `Ctrl+Maj+O` ouvre la zone dans sa fenêtre ; un onglet glissé hors de la fenêtre s'ouvre là ; « Rattacher » le ramène.            |
| F9, F10  | Barre latérale détachable (`Ctrl+Maj+D`) : elle pilote les fenêtres qui suivent la sélection ; une fenêtre peut être épinglée.         |
| F11      | Catalogue, arborescences, requêtes enregistrées et environnements se mettent à jour dans toutes les fenêtres.                          |
| F13, F14 | Zones, tailles et fenêtres détachées rouvertes au démarrage (`windows.json`) ; ⤢ agrandit une zone.                                    |

Pile : [Tauri 2](https://v2.tauri.app) (cœur Rust, WebView2 sous Windows, WebKitGTK sous
Linux), Vue 3, TypeScript strict, Vite, Pinia. Les règles d'architecture et les conventions
sont dans [`CLAUDE.md`](CLAUDE.md).

## Prérequis

- Node.js 22 ou plus récent.
- Rust stable 1.90 ou plus récent (`rustup`).
- **Windows** : Microsoft C++ Build Tools et WebView2 (présent sur Windows 10 22H2 et 11).
- **Linux** (Debian/Ubuntu) :

  ```sh
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libxdo-dev libssl-dev librsvg2-dev
  ```

## Développement

```sh
npm ci
npm run tauri dev
```

| Commande                                    | Rôle                                                              |
| ------------------------------------------- | ----------------------------------------------------------------- |
| `npm run check`                             | Toutes les vérifications de la CI hors build, avant chaque commit |
| `npm test`                                  | Tests frontend (Vitest)                                           |
| `npm run lint` / `npm run typecheck`        | ESLint / vue-tsc                                                  |
| `npm run format` / `npm run format:check`   | Prettier                                                          |
| `cargo test` (dans `src-tauri/`)            | Tests du cœur Rust                                                |
| `cargo clippy --all-targets -- -D warnings` | Lint Rust, zéro avertissement                                     |

## Build

```sh
npm run tauri build
```

| Système | Paquet produit                                                            |
| ------- | ------------------------------------------------------------------------- |
| Windows | `src-tauri/target/release/bundle/nsis/Arnod_<version>_x64-setup.exe`      |
| Linux   | `src-tauri/target/release/bundle/appimage/Arnod_<version>_amd64.AppImage` |

La CI construit les deux paquets à chaque pull request et les publie comme artefacts
(onglet **Actions** › exécution › **Artifacts**).

### Taille des paquets (cible N3 : installeur < 15 Mo)

| Paquet                    | Taille                         | Mesure                           |
| ------------------------- | ------------------------------ | -------------------------------- |
| Installeur Windows (NSIS) | **1,17 Mo** (1 167 097 octets) | CI `windows-latest`, v0.0.1      |
| AppImage Linux            | 80,4 Mo (76,6 Mio)             | Ubuntu 24.04, build local v0.0.1 |

1 Mo = 1 000 000 octets. La CI recalcule ces tailles à chaque build (résumé de
l'exécution) et échoue si l'installeur Windows dépasse 15 Mo (`scripts/bundle-size.mjs`).
L'AppImage embarque WebKitGTK et ses dépendances, d'où sa taille ; la cible N3 vise
l'installeur Windows, qui s'appuie sur le WebView2 du système.

## Publier une version

Les versions suivent [SemVer](https://semver.org/lang/fr/). `Develop` reçoit les pull requests,
`main` porte les versions publiées.

1. **Passer à la nouvelle version** par une pull request vers `Develop` (`chore: version X.Y.Z`) :
   `npm version X.Y.Z --no-git-tag-version` (`package.json` et `package-lock.json`), puis
   `version` dans `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json`, puis
   `cargo update -p arnod --offline` dans `src-tauri/` (`Cargo.lock`).
   `node scripts/check-version.mjs` vérifie que les six sources concordent ; `npm run check` et
   la CI le lancent aussi.
2. **Fusionner `Develop` dans `main`** avec un commit de merge.
3. **Poser le tag sur `main`** et le pousser :
   `git tag -a vX.Y.Z -m "Arnod X.Y.Z" && git push origin vX.Y.Z`.
4. **La CI publie la release** : build, tests, contrôle tag = version, puis release GitHub avec
   l'installeur Windows, l'AppImage et des notes générées à partir des pull requests. Ne pas
   créer la release à la main ; si elle existe déjà, la CI y ajoute les paquets.

## Installation

### Windows, sans droits administrateur

1. Télécharger `Arnod_<version>_x64-setup.exe` depuis les
   [releases](https://github.com/juliencorre/Arnod/releases) et le lancer depuis un compte
   standard. L'installeur est en mode `currentUser` : il ne demande pas d'élévation UAC.
2. SmartScreen peut afficher « Windows a protégé votre ordinateur », l'exécutable n'étant pas
   signé : **Informations complémentaires › Exécuter quand même**.
3. Arnod est installé dans **`%LOCALAPPDATA%\Arnod`** avec un raccourci dans le menu Démarrer.
   (Le PRD, N2, visait `%LOCALAPPDATA%\Programs\Arnod` : le modèle NSIS de Tauri 2 en mode
   `currentUser` impose `%LOCALAPPDATA%\<productName>` et n'offre aucune option pour le
   changer, hormis un modèle `.nsi` complet personnalisé.)
4. Désinstallation : **Paramètres › Applications › Arnod › Désinstaller** (sans droits
   administrateur, l'entrée est enregistrée dans `HKCU`).

WebView2 : s'il est absent, l'installeur télécharge son bootstrapper (connexion requise).
Pour un poste hors ligne, passer `bundle.windows.webviewInstallMode` à `embedBootstrapper`
dans `src-tauri/tauri.conf.json` (+1,8 Mo).

Installation silencieuse : `Arnod_<version>_x64-setup.exe /S`.

### Linux (AppImage)

Télécharger `Arnod_<version>_amd64.AppImage` depuis les
[releases](https://github.com/juliencorre/Arnod/releases), puis :

```sh
chmod +x Arnod_<version>_amd64.AppImage
./Arnod_<version>_amd64.AppImage
```

Aucun droit root n'est nécessaire. Sans FUSE (conteneurs, certaines distributions),
lancer avec `APPIMAGE_EXTRACT_AND_RUN=1`.

## Vérifications

La CI (`.github/workflows/build.yml`, Windows et Linux) exécute lint, format, typecheck,
Vitest, clippy, `cargo test`, le build puis un test de fumée :

- **Windows** (`scripts/smoke-windows.ps1`) : manifeste de l'installeur en `asInvoker`
  (aucune élévation demandée), installation silencieuse dans `%LOCALAPPDATA%\Arnod`,
  raccourci du menu Démarrer, entrée de désinstallation dans `HKCU`, lancement, attente de
  « Créez votre premier projet » dans la page (UI Automation), capture publiée en artefact,
  fermeture par la croix (code 0), désinstallation silencieuse.
- **Linux** (`scripts/smoke-linux.sh`) : AppImage construit sur Ubuntu 22.04, lancé sous
  Xvfb, capture d'écran publiée en artefact, fermeture par `Ctrl+Q` (code 0).
- **E2E** (`e2e/`, voir [`e2e/README.md`](e2e/README.md)) : parcours complets sur
  l'application réelle (binaire debug, cœur Rust, vrai disque) via `tauri-driver`, sous Xvfb,
  en thème clair puis sombre ; captures publiées en artefact.

Lancer les tests e2e en local (Linux) :

```sh
sudo apt install webkit2gtk-driver xvfb
cargo install tauri-driver --locked
npm run build:e2e                      # binaire debug avec l'interface embarquée
xvfb-run -a npm run test:e2e           # ARNOD_E2E_THEME=dark pour le thème sombre
```

Restent à vérifier à la main (le runner Windows CI est administrateur) :

- [ ] Installation et désinstallation sur un **compte Windows standard**, sans invite UAC.
- [ ] Installation sur un Windows 10 sans WebView2.
- [ ] Démarrage à froid < 2 s (N4) et mémoire au repos < 150 Mo (N5) sur le poste de référence.

## Licence

[MIT](LICENSE)
