# Plan - MVP, lot L : mises à jour installées par Arnod

**Version** 0.1 · **Date** 2026-11-16 · **Statut** En cours · **Base** version 0.0.2 (Develop @512df34)

Plan du lot L de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md), après le lot L allégé
(PR #63 : vérification, canal, « Voir la version »). Arnod télécharge sa nouvelle version, en
vérifie la signature et l'installe sans droits administrateur. Écran `1.6-g` (Préférences → Mises à
jour). Il respecte [`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan d'ensemble
(citées « MVP-R… ») ; il tranche la question Q1 de ce plan.

Source : document « Arnod - Mise à jour automatique (tauri-plugin-updater) » (claude.ai, 2026-11-16)
et maquette `1.6-g`.

## 1. Objectif

À la fin de ce lot, Arnod :

- propose « Installer » quand une version plus récente est publiée et installable, dans la
  notification du démarrage comme dans Préférences → Mises à jour ;
- la télécharge par le réseau du poste (proxy, autorités internes), avec sa progression, et vérifie
  sa signature avant de la garder ;
- l'installe sur « Redémarrer maintenant » (puis redémarre) ou « Au prochain lancement » (à la
  fermeture d'Arnod), sans droits administrateur ;
- garde « Voir la version » quand l'installation n'est pas possible ;
- est publié par une CI qui signe ses paquets et joint `latest.json` à chaque release.

## 2. Maquettes

| Repère  | Capture (aperçu)                                                                  | Dans ce plan |
| ------- | --------------------------------------------------------------------------------- | ------------ |
| `1.6-g` | `node scripts/apercu.mjs mises-a-jour --maquette 1.6-g` (et `mises-a-jour-prete`) | MJ1 à MJ6    |

## 3. Modèle et règles

Une **version publiée** est une release GitHub trouvée par le lot L allégé (`updates::pick`, canal
stable ou préversions). Elle est **installable** quand elle porte un **manifeste** `latest.json`
(format du plugin : version, notes, date, une entrée `url` + `signature` par plateforme) et que ce
poste sait la remplacer. Le **paquet** est l'installeur NSIS (Windows) ou l'AppImage (Linux) que
désigne le manifeste ; sa **signature** (minisign, `.sig`) est produite par la CI avec la clé
privée et vérifiée avec la clé publique embarquée dans `tauri.conf.json`.

Le plugin `tauri-plugin-updater` n'est appelé que depuis Rust, sans ses options par défaut (ni
client TLS, ni zip) : Arnod lit le manifeste et télécharge le paquet par `network::Network` ;
le plugin reprend le manifeste (`Updater::restore_update`), vérifie le paquet (`Update::verify`) et
l'installe (`Update::install`).

| ID  | Règle                                                                                                                                                                                                                                                                          |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | Installable si et seulement si : la release porte `latest.json`, Arnod embarque une clé publique non vide, et le poste est installé par un paquet que le plugin remplace (Windows : NSIS ; Linux : AppImage lancée comme telle). Sinon : « Voir la version ».                  |
| R2  | Le manifeste doit annoncer la version de la release trouvée ; sinon il est refusé avant tout téléchargement du paquet.                                                                                                                                                         |
| R3  | Le paquet est vérifié dès la fin du téléchargement, avec la clé publique embarquée, et sa signature doit porter la version annoncée (`requireSignedVersion`). Un paquet refusé n'est ni gardé, ni écrit sur disque, ni lancé ; il est vérifié de nouveau avant l'installation. |
| R4  | Manifeste et paquet passent par le réseau du poste (`network::Network` : proxy, autorités internes, délai de connexion). Le paquet est limité à 200 Mo ; un téléchargement plus long est refusé.                                                                               |
| R5  | Un seul téléchargement à la fois. Annuler, changer de canal ou trouver une autre version oublie le paquet téléchargé.                                                                                                                                                          |
| R6  | Avant d'installer, la fenêtre enregistre ses requêtes et contrats modifiés ; un échec d'enregistrement arrête l'installation.                                                                                                                                                  |
| R7  | Installation sans droits administrateur : l'installeur NSIS reste `currentUser`, lancé en mode passif (`/P /UPDATE`, barre de progression, aucune question). Sous Linux, l'AppImage est remplacée sur place, l'ancienne gardée jusqu'à la réussite.                            |
| R8  | L'état de l'installation (aucune, téléchargement, prête, échec) vit dans le cœur et se diffuse à toutes les fenêtres par `arnod://mise-a-jour`, au plus une fois par pour cent téléchargé.                                                                                     |
| R9  | « Au prochain lancement » installe à la sortie d'Arnod (fenêtre principale fermée ou Fichier > Quitter), sans relancer Arnod ; le paquet reste en mémoire, il est perdu si Arnod s'arrête autrement.                                                                           |

## 4. Cas d'usage

| ID  | Cas d'usage                    | Écrans | Comportement attendu                                                                                                                                                                                    | Priorité |
| --- | ------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| MJ1 | Installer                      | 1.6-g  | Notification du démarrage « Arnod X est disponible. » avec « Installer » (R1), ou bouton « Installer » de la section : téléchargement avec barre de progression (« 3,1 Mo sur 9,8 Mo ») et « Annuler ». | Must     |
| MJ2 | Redémarrer maintenant          | 1.6-g  | Paquet vérifié : « Arnod X est prête » (notification et section). « Redémarrer maintenant » enregistre (R6), installe (R7) et relance Arnod, qui rouvre projets, onglets et brouillons.                 | Must     |
| MJ3 | Au prochain lancement          | 1.6-g  | L'installation attend la fermeture d'Arnod (R9) ; la section le dit et propose encore « Redémarrer maintenant ».                                                                                        | Should   |
| MJ4 | Signature ou manifeste refusés | 1.6-g  | Message clair (« La signature … ne correspond pas : rien n'a été installé. ») ; « Réessayer » et « Voir la version » restent proposés.                                                                  | Must     |
| MJ5 | Réseau d'entreprise            | 1.6-g  | Le téléchargement suit « Réseau et certificats » ; un échec dit sa cause (proxy, certificat) comme un envoi.                                                                                            | Must     |
| MJ6 | Installation impossible        | 1.6-g  | Release sans manifeste, clé absente, Linux hors AppImage : « Voir la version », comme au lot L allégé.                                                                                                  | Must     |
| MJ7 | Publication signée             | -      | Un tag `vX.Y.Z` produit des paquets signés et un `latest.json` joint à la release ; un tag avec suffixe (`-beta.1`) publie une préversion. Sans clé privée, la release est refusée.                     | Must     |

## 5. Hors périmètre

- Signature de code Windows (Authenticode) : certificat payant, hors périmètre de `CLAUDE.md`.
  SmartScreen peut avertir au premier téléchargement manuel, pas aux mises à jour suivantes.
- Pastille « mise à jour » en bas de la barre latérale (texte du lot L) : la notification et la
  section suffisent ; à reprendre avec la barre d'état.
- Stratégie de groupe pour couper les vérifications (écart de 1.6-g) : V2.
- Paquet gardé sur disque entre deux lancements ; reprise d'un téléchargement interrompu.
- macOS et paquets `.deb` / `.rpm` : non publiés.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

- `updates.json` (préférences du poste) : inchangé.
- `latest.json`, joint à chaque release par la CI (format du plugin) :

  ```json
  {
    "version": "0.1.0",
    "notes": "Arnod 0.1.0",
    "pub_date": "2026-11-20T10:00:00Z",
    "platforms": {
      "windows-x86_64": {
        "url": "https://github.com/…/Arnod_0.1.0_x64-setup.exe",
        "signature": "…"
      },
      "linux-x86_64": { "url": "https://github.com/…/Arnod_0.1.0_amd64.AppImage", "signature": "…" }
    }
  }
  ```

  Les clés `windows-x86_64-nsis` et `linux-x86_64-appimage` reprennent les mêmes entrées.

- `tauri.conf.json` : `plugins.updater` (`pubkey`, `requireSignedVersion: true`,
  `windows.installMode: "passive"`, aucune adresse : le manifeste est celui de la release trouvée).

### 6.2 Cœur Rust

- `updates.rs` (pur) : actifs d'une release (`GhRelease.assets`), adresse du manifeste
  (`Release.manifest`, non sérialisée), contrôle du manifeste (R2), état `InstallState`, pas de
  diffusion de la progression (R8), limite du paquet (R4), libellés des erreurs du plugin.
- `network::Network::download` : téléchargement binaire par le client du poste, progression,
  annulation (`http::Cancel`), limite de taille ; échecs classés comme les envois.
- `commands/updates.rs` : session (version trouvée, état, paquet vérifié, annulation), installable
  (R1 : clé du `tauri.conf.json`, `Env::appimage`), diffusion `arnod://mise-a-jour`, installation à
  la sortie (R9, `RunEvent::Exit` dans `lib.rs`).
- Dépendance : `tauri-plugin-updater` (`default-features = false`), initialisé dans `lib.rs`.
  Aucune permission WebView : ses commandes ne sont pas appelables par l'interface.

### 6.3 Commandes IPC

| Commande           | Paramètres     | Réponse       | Erreurs (`kind`)                                    |
| ------------------ | -------------- | ------------- | --------------------------------------------------- |
| `updates_download` | -              | `UpdatesView` | `invalidRequest`, `network`, `timeout`, `cancelled` |
| `updates_cancel`   | -              | `UpdatesView` | -                                                   |
| `updates_install`  | `now: boolean` | `UpdatesView` | `invalidRequest`, `io`                              |

`UpdatesView` gagne `installable` (R1) et `install` (`InstallState` : `idle`, `downloading`
avec `received` / `total`, `ready` avec `version` / `onExit`, `failed` avec `message`).
`updates_install(now: true)` ne revient pas : Arnod se ferme (Windows) ou redémarre (Linux).

### 6.4 Frontend

- Store `updates` : `download`, `cancel`, `install(now)` (enregistre d'abord : R6), abonnement à
  `arnod://mise-a-jour` ; notification « Installer » au démarrage, puis « Arnod X est prête. »
  avec « Redémarrer maintenant » à la fin du téléchargement.
- `UpdatesSection.vue` : carte de la version trouvée selon l'état (Installer / progression et
  Annuler / Redémarrer maintenant et Au prochain lancement / échec et Réessayer) ; « Notes de
  version » ouvre toujours la page de la release.

### 6.5 Design

Aucun jeton : la barre de progression reprend `--color-accent` et `--color-border`. Tous les
boutons sont des `BaseButton` (un seul `primary` par état) ; la progression est un `<progress>`
nommé, annoncé par `role="status"`.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                                                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CLAUDE.md`                           | Updater sorti du hors périmètre ; plugin ajouté aux ajouts validés ; section « Mises à jour ».                                                             |
| `README.md`                           | « Publier une version » : clé de signature, secrets, `latest.json`, préversions.                                                                           |
| `src-tauri/capabilities/default.json` | Inchangé : aucune permission `updater:*`.                                                                                                                  |
| `.github/workflows/build.yml`         | Paquets signés quand la clé est fournie (obligatoire sur un tag), `.sig` publiés, `latest.json`.                                                           |
| `scripts/updater-manifest.mjs`        | Nouveau : `latest.json` d'après les paquets et leurs signatures.                                                                                           |
| `docs/maquette-etat.yml`              | `1.6-g` livré, écart réduit à la stratégie de groupe.                                                                                                      |
| `e2e/`                                | Inchangé : le parcours 1.6-g existant couvre la section ; l'installation demande une vraie release signée (critère 1, à vérifier sur la première release). |
| Installeur                            | Mesuré par la CI (< 15 Mo) ; le plugin sans options par défaut n'ajoute ni TLS ni zip.                                                                     |

## 8. Plan de tâches pour Claude Code

Un lot = une pull request, titrée en Conventional Commits. `npm run check` passe avant chaque
commit ; chaque tâche est relue avant commit.

**Lot L - Installation des mises à jour**

- [x] **T1** Cœur : manifeste, état, téléchargement par le réseau du poste, vérification,
      installation maintenant ou à la fermeture ; tests Rust R1 à R9.
- [x] **T2** Interface : store, notification, section 1.6-g ; faux cœur, tests Vitest au clavier,
      aperçu.
- [x] **T3** CI et publication : paquets signés, `latest.json`, préversions ; README.
- [x] **T4** `CLAUDE.md`, `maquette-etat.yml`, plan d'ensemble (Q1, lot L), e2e.

## 9. Critères d'acceptation

- [ ] Une release signée se télécharge, se vérifie et s'installe depuis Arnod sous Windows
      (compte standard) et Linux (AppImage), puis Arnod rouvre ses projets.
- [x] Un paquet modifié ou signé par une autre clé est refusé, rien n'est lancé (test Rust).
- [x] Le téléchargement passe par le proxy et les autorités de « Réseau et certificats ».
- [x] Sans `latest.json` ou sans clé publique, la section garde « Voir la version ».
- [ ] Un tag sans clé privée échoue dans la CI au lieu de publier une release non installable.

## 10. Risques

| Risque                                     | Impact                                                      | Parade                                                                                    |
| ------------------------------------------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Clé privée perdue                          | Plus aucune mise à jour automatique : réinstaller à la main | Clé gardée aussi dans un gestionnaire de mots de passe ; README.                          |
| Clé publique absente de `tauri.conf.json`  | Les versions publiées restent en « Voir la version »        | R1 : comportement du lot L allégé ; la clé se pose une fois, avant la première release.   |
| Brouillon d'une fenêtre détachée en cours  | Modification de moins de 600 ms perdue au redémarrage       | Enregistrement automatique ; la fenêtre qui installe enregistre tout (R6).                |
| Second `reqwest` (0.13) tiré par le plugin | Quelques centaines de Ko                                    | Options par défaut coupées (ni TLS, ni zip) ; taille contrôlée par la CI.                 |
| Mise à jour du plugin                      | `restore_update` ou `verify` changent                       | Plugin épinglé en `2.14` ; tests de la vérification avec une vraie paire de clés de test. |

## 11. Décisions

| ID  | Question                                                        | Décision                                                                                                                                                                                                          |
| --- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | `tauri-plugin-updater`, clés de signature et secrets de la CI ? | 2026-11-16 : oui (document « Mise à jour automatique »). Plugin appelé depuis Rust seulement ; clé publique dans `tauri.conf.json` ; secrets `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. |
| Q2  | Un `latest-beta.json` pour les préversions ?                    | 2026-11-16 : non. Chaque release porte son propre `latest.json` ; le canal choisit la release (lot L allégé), Arnod lit le manifeste de celle-ci.                                                                 |
| Q3  | Qui télécharge : le plugin ou Arnod ?                           | 2026-11-16 : Arnod (`network::Network`), pour le proxy et les autorités internes ; le plugin vérifie et installe.                                                                                                 |
