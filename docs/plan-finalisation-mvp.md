# Plan - Finalisation du MVP

**Version** 1.0 · **Date** 2026-11-13 · **Statut** En cours · **Base** version 0.0.2 (Develop @db2fbbd)

Plan d'ensemble des écarts restants entre la maquette, le design system et le code, jusqu'à la fin
du MVP. Il découpe le travail en lots, dit lesquels sont indépendants et peuvent avancer en
parallèle, et fixe les règles communes à tous les lots. Il respecte les règles de
[`CLAUDE.md`](../CLAUDE.md) ; chaque lot reste une pull request, avec ses cas d'usage ici.

Source : maquette au 13 novembre 2026 (page 0.1 « Périmètre et feuille de route » : MVP « Remplacer
Bruno au quotidien », 8 livrés · 5 en partie · 6 à faire), [`maquette-etat.yml`](maquette-etat.yml)
réconcilié avec elle, design system « Arnod » ([`design-system/`](design-system/README.md)). Les
écrans sont cités par leur repère (`5.1-b bis`).

## 1. Objectif

À la fin de ce plan, Arnod remplace Bruno au quotidien : reprendre ses collections Bruno, écrire,
envoyer et ranger ses tests d'API, partir d'un contrat OpenAPI, sur un réseau d'entreprise. Toutes
les fonctions de la colonne MVP de 0.1-a sont livrées, et chaque écran MVP de la maquette est livré
ou porte un écart tranché.

## 2. Reste à faire du MVP

| Fonction (0.1-a)               | État      | Reste                                                                            | Lot       |
| ------------------------------ | --------- | -------------------------------------------------------------------------------- | --------- |
| Requête d'essai sans projet    | En partie | ranger dans un projet, transférer au référentiel, chemins généralisés            | J         |
| Requêtes et vérifications      | En partie | extractions, OAuth 2, cookies, cURL, variables inconnues, réponse, texte, Ctrl+J | A B E F H |
| Tests et étapes                | En partie | verdict, arrêt sur échec, valeurs passées, onglet du test                        | E         |
| Import Bruno                   | En partie | fichiers .bru et rapport d'import                                                | C         |
| Étiquettes                     | En partie | filtre (1.2-e), propositions avec leur nombre (4.9-a)                            | D K       |
| Valeurs dynamiques             | À faire   | 5.1-h                                                                            | E         |
| Réseau d'entreprise            | À faire   | 1.6-e, 5.1-i                                                                     | H         |
| Fichiers modifiés hors d'Arnod | À faire   | 4.1-e                                                                            | I         |
| Filtrer l'arborescence         | À faire   | 1.2-e                                                                            | D         |
| Mises à jour de l'application  | À faire   | 1.6-g                                                                            | L         |
| Copier et déplacer une requête | À faire   | 4.1-f, g                                                                         | J         |

S'y ajoutent les écrans MVP en partie ou à faire hors de ces fonctions (0.3, 1.3, 1.4-j, 1.2-d,
1.2-a ter, 3.2-a3, 4.1-b) et les écarts « aligner le code » (1.1-a, 1.2-d, 4.2-b, vocabulaire).

## 3. Règles communes

Ces règles valent pour tous les lots : elles fixent ce que deux lots parallèles doivent faire de la
même façon.

| ID  | Règle                                                                                                                                                                                                                                                                                                         |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Une variable dont le nom commence par `$` est une **valeur dynamique**, aux noms de Bruno (`$guid`, `$randomUUID`, `$timestamp`, `$isoTimestamp`, `$randomInt`, `$randomEmail`…) : tirée à chaque envoi, jamais signalée inconnue, jamais écrite dans un environnement.                                       |
| R2  | Une **extraction** est une action OpenCollection `set-variable` en phase `after-response` (`runtime.actions`, schéma 1.0.0) : elle range une valeur de la réponse dans une variable d'exécution, lue par les étapes suivantes du même test ou run. Aucun champ inventé.                                       |
| R3  | Un **test** est un dossier qui contient des requêtes et aucun sous-dossier ; ses requêtes sont ses étapes, dans l'ordre du dossier. Il réussit si toutes ses étapes réussissent ; il s'arrête à la première étape en échec (réglage « continuer » en V4).                                                     |
| R4  | Vocabulaire : un lot qui ajoute du texte d'interface garde les mots actuels (« requête », « vérification ») ; le lot K renomme tout d'un coup selon 0.2 (« test », « étape », « assertion »).                                                                                                                 |
| R5  | Fichiers partagés (`lib.rs` `generate_handler!`, `commands/mod.rs`, `src/types/ipc.ts`, `src/lib/ipc.ts`, `tests/fakeCore.ts`, `e2e/arnod.e2e.ts`, `docs/maquette-etat.yml`, `CLAUDE.md`) : n'y ajouter qu'en fin de liste ou de fichier, dans un bloc propre au lot, pour que les fusions restent triviales. |
| R6  | Chaque lot ajoute ses scénarios à l'aperçu (`tests/apercu/scenarios.ts`, nommés d'après le repère) et joint à sa pull request la comparaison avec l'écran de la maquette (`node scripts/apercu.mjs <scénario> --maquette <repère>`), thèmes clair et sombre.                                                  |
| R7  | Chaque lot met à jour `docs/maquette-etat.yml` (état, pull request, captures, écarts restants) et ajoute ses parcours e2e dans un `describe` à son nom à la fin de `e2e/arnod.e2e.ts`.                                                                                                                        |
| R8  | Pas de nouveau plugin Tauri ni de permission WebView, sauf le lot L (décision Q1). Une crate Rust ajoutée est justifiée en commentaire dans `Cargo.toml` ; l'installeur reste sous 15 Mo.                                                                                                                     |
| R9  | Les écarts « à décider » (1.1-e, 2.2-a, 3.1-e, 3.1-g, 3.2-b, 3.3-a, 4.4-c) ne sont pas traités ici : le code reste tel quel tant qu'ils ne sont pas tranchés.                                                                                                                                                 |
| R10 | Le design system s'applique (échelles `--text-*`, `--control-*`, `--radius-*`, un seul bouton plein par vue) ; une maquette qui dessine une valeur hors échelle est suivie dans le code par l'échelle.                                                                                                        |

## 4. Lots

Une vague regroupe des lots indépendants : fichiers principaux distincts, aucune dépendance de
données entre eux. Ils se développent en parallèle, chacun dans son worktree et sa branche
`claude/mvp-<lot>`, et fusionnent dans l'ordre de leur fin. La vague suivante part de Develop une
fois la précédente fusionnée.

### Vague 1

**Lot A - Réponse** (`5.1-b bis`). Format et aperçu (JSON, XML, HTML, texte), copier, enregistrer
dans un fichier, effacer ; filtre JSONPath qui surligne les valeurs ; un résultat du filtre devient
une vérification. Le filtre est évalué par le cœur. Fichiers : `ResponsePanel.vue` et ses nouveaux
sous-composants, module Rust `response/`. « Devient une variable » attend le lot E (bouton absent
d'ici là).

**Lot B - Saisie, texte et cURL** (`5.1-b ter`, `5.1-b quater`, `2.1-a sexies`). Variables inconnues
de l'environnement soulignées et signalées avant l'envoi (R1 exclue) ; compteur par onglet rempli ;
en-têtes ajoutés par Arnod listés, grisés ; « Éditer en texte » pour paramètres, en-têtes et
variables ; `</>` et Ctrl+Maj+C copient la requête en cURL (jeton remplacé par `$TOKEN`) ; une
commande cURL collée dans « Essayer une étape » devient une requête REST (jeton au coffre, option
sans équivalent signalée). Fichiers : `RequestEditor.vue`, `PairsEditor.vue`, `TryStep.vue`,
module Rust `curl/`, `steps/` (reconnaissance).

**Lot C - Import Bruno** (`2.3-a`, `2.3-b`, `2.3-f`). Une collection Bruno en fichiers `.bru`
(`bruno.json`, `collection.bru`, `environments/*.bru`, `folder.bru`) devient un projet
OpenCollection : requêtes, environnements, authentification, assertions, extractions (R2) et
valeurs dynamiques (R1) repris sans code ; scripts sans équivalent gardés dans `runtime.scripts` et
signalés ; rapport d'import (2.3-f). « Reprendre l'existant » s'ajoute aux points de départ (2.3-b).
Fichiers : module Rust `bruno/`, commandes `import_*`, vue du rapport, entrées « Importer ».

**Lot D - Barre latérale** (`1.2-e`, `1.2-d`, `1.2-a ter`, `3.2-a3`, écart `1.1-a`). Champ de filtre
en tête du référentiel (nom, adresse, `#étiquette`, méthode ; dossiers des résultats dépliés,
compte affiché) ; projet vide : quatre points de départ ; menu « + » complet avec raccourcis ;
réponses attendues listées sous chaque endpoint, avec leur nombre, un clic ouvre la réponse ;
nombre de contrats sur les cartes de projet. Le filtre est un filtre d'affichage. Fichiers :
`TreePanel.vue`, `TreeRow.vue`, `SidebarProject.vue`, `ContractsSection.vue`,
`LinkedRequestRow.vue`, `ProjectCard.vue`, `tree::tests_view` et `ContractSummary` côté cœur.

**Lot E - Tests et étapes** (`4.2-a`, `4.2-b bis`, `5.1-h`, `4.1-b`, écart `4.2-b`). Le dossier
devient un test (R3) : verdict, arrêt sur échec, étapes numérotées, onglet du test et « + Étape » ;
extractions (R2) éditées dans un onglet « Variables » de la requête et appliquées à l'envoi et au
run ; valeurs dynamiques (R1) tirées à chaque envoi, montrées dans « Envoyé », « Garder » en fait
une extraction ; « modifié, pas encore envoyé » et « Revenir à la version envoyée ». Fichiers :
`request/resolve.rs`, `request/` (extractions), `workbench.rs` (run d'un test), `RunPanel.vue`,
nouvel onglet du test, `opencollection/` (actions).

### Vague 2

**Lot F - Onglets et clavier** (`1.3-a`, `1.3-b`, `1.3-c`, `1.4-j`). Onglet d'aperçu en italique,
◐ pendant une exécution ; « +N » liste les onglets en trop, fermer les autres ou tous ; Ctrl+Tab
dans l'ordre d'utilisation ; Ctrl+J (résultat dessous ou à côté) et Ctrl+\ (barre latérale), dans
le menu Fenêtre, gardés par fenêtre. Fichiers : `layout/`, `ZoneView.vue`, `ZoneArea.vue`,
`menu.rs`.

**Lot G - Démarrage et erreurs** (`0.3-a`, `0.3-b`, `0.3-b bis`, `0.3-b ter`). Squelette au-delà de
300 ms ; erreur bloquante : cause, ce qui est intact, deux issues ; fichier verrouillé réessayé
seul ; détails pour le support repliés, à copier ou à joindre à une issue préremplie (adresse
copiée, rien n'est envoyé seul). Fichiers : `App.vue`, nouveau composant d'erreur, `storage.rs`,
`windows.rs` (lecture au démarrage).

**Lot H - Réseau et authentification** (`1.6-e`, `5.1-i`, `3.4-a`, `3.4-g`). Préférences (Ctrl+,)
→ Réseau et Certificats : proxy et autorités du système, autorité interne ajoutée ; échec réseau
expliqué (certificat, proxy, nom, serveur) avec l'action qui règle le poste ; OAuth 2 par
identifiants client réglé au projet, jeton obtenu et renouvelé ; cookies gardés par environnement,
listés et vidables. Préférences du poste dans `app_local_data_dir()`, jamais dans le projet.
Fichiers : `http.rs`, nouveau module `network/`, `AuthEditor.vue`, réglages du projet, fenêtre des
préférences.

**Lot I - Fichiers modifiés hors d'Arnod** (`4.1-e`). Surveillance du dossier du projet ;
arborescence, contrats, environnements et onglets rechargés ; un brouillon en conflit se règle
élément par élément, sans rien écraser. Fichiers : nouveau module `watch.rs` (crate `notify`),
store `workbench` (brouillons), dialogue de conflit.

### Vague 3

**Lot J - Copier, déplacer, ranger** (`4.1-f`, `4.1-g`, `4.1-a quater`, `2.2-b`, `2.2-c`, `2.2-d`,
`3.2-c`). Glisser-déposer entre les six endroits (0.2), Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+Z, « Copier
vers… / Déplacer vers… » (autre projet compris), ce qui change affiché avant de lâcher ; sélection
multiple ; « Transférer au référentiel de tests » ; chemins généralisés ; « Ranger dans un
projet… » depuis Non classés. Après D et E (barre latérale, notion de test).

**Lot K - Vocabulaire et finitions** (écart transverse de vocabulaire, `4.2-c`, `4.9-a`, `5.1-c`).
« test », « étape », « assertion » partout (interface, README, CLAUDE.md, maquette-etat) ; secret
saisi dans Auth proposé en variable locale ; étiquettes proposées avec leur nombre ; chronologie
détaillée autant que le client HTTP la mesure. En dernier : il touche tous les écrans.

**Lot L - Mises à jour** (`1.6-g`). Nouvelle version téléchargée, signature vérifiée, signalée en
bas de la barre latérale, installée sans droits administrateur. Suppose `tauri-plugin-updater`, une
paire de clés de signature et les secrets de la CI : décision Q1.

## 5. Hors périmètre

- Tout ce que 0.1-a range en V1 à V9 (historique, raccourcis modifiables, apparence, synchronisation,
  ligne de commande, recherche, barre d'état, GraphQL, templates, mocks…).
- Les natures d'étape autres que REST (2.1-a ter à a quinquies) : V6.
- Les écarts « à décider » (R9) et la barre de titre intégrée (aligner la maquette).

## 6. Vérification

| Niveau         | Outil                                                                                                                                                   | Quand                             |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| Règles         | `cargo test` (vrais dossiers temporaires, wiremock)                                                                                                     | chaque lot                        |
| Interface      | Vitest et faux cœur (`tests/fakeCore.ts`), clavier compris                                                                                              | chaque lot                        |
| Écran          | `scripts/apercu.mjs` : aperçu Playwright (Chromium) sur le faux cœur, côte à côte avec l'écran de la maquette, 1280 px et zone étroite, clair et sombre | chaque lot, joint à la PR         |
| Parcours réels | e2e WebdriverIO sur l'application réelle (WebKitGTK)                                                                                                    | à l'intégration, un lot à la fois |
| Tout           | `npm run check`, CI (Windows, Linux, SonarCloud)                                                                                                        | chaque commit, chaque PR          |

L'aperçu montre les composants et styles réels sur un cœur simulé : il juge la mise en page, pas
le comportement du cœur, que vérifient `cargo test` et l'e2e.

## 7. Impacts sur l'existant

| Fichier                                                        | Changement                                                                                                                                                                   |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/maquette-etat.yml`                                       | Versions V1 à V9 de la maquette ; écrans MVP 1.2-e, 1.6-e, 1.6-g, 2.3-f, 4.1-e à g, 5.1-h, i ; écarts transverses (barre d'état sortie, vocabulaire à aligner dans le code). |
| `scripts/check-maquette.mjs`                                   | Versions V2 à V9 acceptées.                                                                                                                                                  |
| `tests/apercu/`, `scripts/apercu.mjs`, `vite.apercu.config.ts` | Aperçu sur le faux cœur et captures Playwright.                                                                                                                              |
| `CLAUDE.md`                                                    | Travail en cours, commande de l'aperçu ; chaque lot y ajoute sa section.                                                                                                     |
| `src-tauri/capabilities/default.json`                          | Inchangé (lot L : à décider).                                                                                                                                                |

## 8. Tâches

- [x] **T0** — Plan, état de la maquette réconcilié, aperçu Playwright (`scripts/apercu.mjs`).
- [ ] **T1** — Vague 1 : lots A, B, C, D, E en parallèle, une PR chacun.
- [ ] **T2** — Vague 2 : lots F, G, H, I.
- [ ] **T3** — Vague 3 : lots J, K ; lot L après Q1.
- [ ] **T4** — Fin du MVP : 0.1-a sans « à faire » en MVP, maquette resynchronisée
      (`scripts/maquette-sync.py`), captures réelles à jour.

## 9. Critères d'acceptation

- [ ] Chaque écran MVP de `maquette-etat.yml` est « livré », ou porte un écart tranché.
- [ ] Une collection Bruno en `.bru` s'ouvre et son test s'exécute sans retouche (e2e).
- [ ] Un test de trois étapes passe une valeur extraite de l'étape 1 à l'étape 3 et s'arrête à la
      première étape en échec (e2e).
- [ ] Derrière un proxy d'entreprise avec une autorité interne, un envoi aboutit après réglage, et
      l'échec d'avant dit quoi régler.
- [ ] `npm run check` et la CI passent ; l'installeur reste sous 15 Mo.

## 10. Risques

| Risque                                              | Impact                                | Parade                                                                                              |
| --------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Conflits entre lots parallèles                      | Fusions longues                       | R5 ; vagues par fichiers ; fusion dans l'ordre de fin, rebase des suivants.                         |
| Machine de développement à 2 cœurs                  | Compilations Rust en file             | Un seul répertoire `target` partagé, sans compilation incrémentale ; e2e à l'intégration seulement. |
| Proxy et autorités Windows non testables sous Linux | Régression sur le poste cible         | Logique pure testée ; lecture du système derrière un trait ; vérification sous Windows listée.      |
| Surveillance des fichiers et écritures d'Arnod      | Rechargement de ses propres écritures | Ignorer les écritures récentes d'Arnod (empreinte du contenu).                                      |
| Vocabulaire changé en dernier                       | Captures et tests à reprendre         | Un seul lot (K), tests par sélecteurs plutôt que par textes quand c'est possible.                   |

## 11. Décisions

| ID  | Question                                                                       | Décision                                                                                                |
| --- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------- |
| Q1  | Mises à jour : `tauri-plugin-updater`, clés de signature et secrets de la CI ? | À trancher par Julien : il faut générer la paire de clés et poser les secrets ; le lot L attend.        |
| Q2  | Arrêt sur échec réglable dès le MVP ?                                          | 2026-11-13 : non, un test s'arrête toujours à la première étape en échec (R3) ; le réglage vient en V4. |
| Q3  | Où vit une extraction ?                                                        | 2026-11-13 : action `set-variable` du schéma OpenCollection (R2), sans extension Arnod.                 |
| Q4  | Filtre de l'arborescence : dans le cœur ou dans l'interface ?                  | 2026-11-13 : dans l'interface, c'est un filtre d'affichage (`CLAUDE.md`).                               |
| Q5  | Écarts « à décider » de la maquette                                            | 2026-11-13 : hors de ce plan (R9) ; à trancher écran par écran.                                         |
