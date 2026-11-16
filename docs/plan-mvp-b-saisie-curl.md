# Plan - Saisie, texte et cURL (lot B du MVP)

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** version 0.0.2 (Develop @e10ba9a)

Plan d'implémentation du lot B de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : ce qui
aide à écrire une requête (variables inconnues, compteurs d'onglets, en-têtes ajoutés à l'envoi),
la requête en texte (listes en texte, copie en cURL) et la commande cURL collée dans « Essayer une
étape ». Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du
plan de finalisation (notées « MVP-R1 »… ci-dessous pour les distinguer).

Source : maquette au 13 novembre 2026, fichiers « Arnod - Exécution » (page 5.1) et « Arnod -
Premier usage » (page 2.1).

## 1. Objectif

À la fin de ce plan, Arnod :

- souligne en rouge, pendant la saisie, une variable `{{nom}}` inconnue de l'environnement actif
  et la signale sous l'adresse avant l'envoi, avec l'environnement qui la définit ;
- montre un nombre ou un point sur chaque onglet rempli de la requête, et liste, grisés, les
  en-têtes qu'Arnod ajoute à l'envoi ;
- édite paramètres et en-têtes en texte (`nom: valeur`, `nom=valeur`, `#` pour écarter une ligne) ;
- copie la requête en commande cURL (`</>`, Ctrl+Maj+C), secrets remplacés par `$TOKEN` ;
- reconnaît une commande cURL collée dans « Essayer une étape » : méthode, adresse, en-têtes et
  corps repris, jeton gardé au coffre du système, option sans équivalent signalée.

## 2. Maquettes

| Repère         | Capture (aperçu)                              | Dans ce plan |
| -------------- | --------------------------------------------- | ------------ |
| `5.1-b ter`    | `.apercu/5.1-b-ter-clair-1280.png`, e2e `22`  | SA1 à SA6    |
| `5.1-b quater` | `.apercu/5.1-b-quater-clair-1280.png`, e2e 23 | TX1 à TX4    |
| `2.1-a sexies` | `.apercu/2.1-a-sexies-clair-1280.png`, e2e 21 | CU1 à CU4    |

## 3. Modèle et règles

Une **variable connue** a une valeur dans l'environnement actif (partagée, ou locale renseignée sur
ce poste, secrète comprise) ou est déclarée dans `request.variables` d'un `folder.yml` du chemin ou
de `opencollection.yml`. Une **valeur dynamique** (`{{$guid}}`, MVP-R1) n'est jamais inconnue.

Les règles vivent dans le cœur Rust, modules purs `src-tauri/src/input/` et `src-tauri/src/curl/`,
testés règle par règle (`input/tests.rs`, `curl/tests.rs`, `steps/tests.rs`).

| ID  | Règle                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| R1  | Variables connues : partagées de l'environnement actif, locales renseignées (valeur locale ou secret au coffre), puis celles des dossiers et de la collection. Les noms en `$…` ne sont jamais inconnus.                                                                                                                                                                                                                                                           |
| R2  | Variables inconnues : celles de ce qui part à l'envoi (adresse, paramètres et en-têtes actifs, hérités compris, authentification effective, corps du type choisi), absentes de R1, avec les autres environnements qui les définissent (valeur d'une partagée).                                                                                                                                                                                                     |
| R3  | En-têtes ajoutés à l'envoi : Authorization (ou clé d'API) de l'authentification effective, masquée ; Content-Type du corps ; User-Agent `Arnod/<version>`, Accept `*/*`, Content-Length (avec un corps), Host (adresse résolue avec les variables non secrètes). Un en-tête saisi ou hérité du même nom remplace celui du corps ou du client ; celui de l'authentification s'y ajoute.                                                                             |
| R4  | Une commande cURL se lit comme un shell POSIX (guillemets simples, doubles, `$'…'`, `\` de fin de ligne) ; `-X -H -d --data* --data-urlencode --json -u -b -A -e -G -I --url` sont repris ; méthode déduite comme cURL (POST avec des données, HEAD avec `-I`) ; `http://` ajouté sans schéma ; Content-Type `application/x-www-form-urlencoded` ajouté pour des données sans type.                                                                                |
| R5  | Un secret collé ne reste pas dans le texte : jeton `Bearer` → `{{token}}`, `Basic` (et `-u`) → `{{basicAuth}}`, autre Authorization → `{{authorization}}`, Cookie et en-têtes `*api-key*`, `*token*`, `*secret*`, `*password*` → leur nom en camelCase. Une variable du shell (`$TOKEN`) devient `{{token}}` sans secret. À l'envoi de l'essai, les secrets vont au coffre en variables locales secrètes de « Local » de « Non classés » ; la réponse est masquée. |
| R6  | Une option sans équivalent est signalée une fois, avec sa raison (`--compressed`, `-k`, `--cert`, `-x`, `--max-time`, `-o`, `-F`, option inconnue, adresse en trop, fichier `@…`) ; `-s -S -v -i -L` sont sans effet et sans message.                                                                                                                                                                                                                              |
| R7  | Un texte qui commence par `curl` mais ne se lit pas (guillemet non fermé, pas d'adresse) n'est pas reconnu : rien n'est rempli.                                                                                                                                                                                                                                                                                                                                    |
| R8  | La commande écrite : `curl [-X M] 'adresse'`, une option par ligne (`-H`, `-u`, `--data`, `--data-raw` si le corps commence par `@`), guillemets simples, doubles autour d'une variable du shell.                                                                                                                                                                                                                                                                  |
| R9  | Une Authorization écrite en clair dans les en-têtes est remplacée : `Bearer $TOKEN`, sinon `$CREDENTIALS`.                                                                                                                                                                                                                                                                                                                                                         |
| R10 | Variables partagées résolues dans l'environnement actif ; variables secrètes → `$NOM` (`apiKey` → `$API_KEY`) ; jeton Bearer → `$TOKEN`, mot de passe Basic → `-u "user:$PASSWORD"`, clé d'API → `$API_KEY` ; inconnues et valeurs dynamiques laissées `{{…}}`.                                                                                                                                                                                                    |
| R11 | Texte d'une liste : une ligne par paire, `Nom: valeur` (en-têtes) ou `nom=valeur` (paramètres), `# ` devant une paire désactivée.                                                                                                                                                                                                                                                                                                                                  |
| R12 | Lecture du texte : lignes vides sautées ; `#` désactive ; en-tête sans `:` ou avec un nom contenant un espace, nom vide : ligne illisible, nommée (`Ligne 5 : « : » manquant entre le nom et la valeur.`) ; paramètre : premier `=` ou `:`, sans séparateur un nom seul ; pseudo-en-tête `:authority` sauté.                                                                                                                                                       |
| R13 | Une paire lue garde la description de la paire de même nom de la liste d'avant (casse ignorée pour un en-tête).                                                                                                                                                                                                                                                                                                                                                    |

## 4. Cas d'usage

| ID  | Cas d'usage                   | Écrans         | Comportement attendu                                                                                                                                                         | Priorité |
| --- | ----------------------------- | -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| SA1 | Variable inconnue soulignée   | `5.1-b ter`    | Adresse, paramètres, en-têtes, corps, auth : `{{nom}}` inconnu souligné en rouge ondulé pendant la saisie (relu 250 ms après la frappe).                                     | Must     |
| SA2 | Variable signalée avant envoi | `5.1-b ter`    | Sous l'adresse : « apiVersion est inconnue dans Recette ; définie dans Local (v2). Ajouter · Environnements… ». L'envoi reste possible : le cœur le refuse avec son message. | Must     |
| SA3 | Ajouter la variable           | `5.1-b ter`    | « Ajouter » la met dans les variables partagées de l'environnement actif, avec la valeur d'un autre environnement ; « Environnements… » ouvre leur gestion.                  | Should   |
| SA4 | Compteur ou point par onglet  | `5.1-b ter`    | Nombre pour Paramètres, En-têtes, Vérifications ; point pour Corps, Auth, Docs, Réglages remplis (« rempli » pour un lecteur d'écran).                                       | Must     |
| SA5 | En-têtes ajoutés listés       | `5.1-b ter`    | Onglet En-têtes : « De cette requête · n », « Hérités · n », « Ajoutés à l'envoi · n par Arnod », grisés, et la règle de remplacement.                                       | Must     |
| SA6 | User-Agent envoyé             | `5.1-b ter`    | Le client HTTP envoie `User-Agent: Arnod/<version>` (remplacé par un en-tête saisi).                                                                                         | Should   |
| TX1 | Éditer en texte               | `5.1-b quater` | « Éditer en texte » sur Paramètres de requête et En-têtes : zone de texte numérotée ; chaque saisie lisible met la liste à jour (enregistrement auto).                       | Must     |
| TX2 | Ligne illisible               | `5.1-b quater` | Message par ligne, numéro de ligne en rouge, « Revenir au tableau » refusé tant qu'une ligne est illisible ; rien n'est perdu.                                               | Must     |
| TX3 | Copier en cURL                | `5.1-b quater` | `</>` ouvre « Copier la requête cURL · variables de <env> », commande, note, « Copier ». Échap ferme et rend le focus à `</>`.                                               | Must     |
| TX4 | Ctrl+Maj+C                    | `5.1-b quater` | Dans l'éditeur, copie la commande dans le presse-papiers et montre la fenêtre.                                                                                               | Must     |
| CU1 | cURL collée reconnue          | `2.1-a sexies` | « Reconnu : une commande cURL. » ; l'entrée prend l'adresse ; méthode, en-têtes (texte `Nom: valeur`), type du corps et corps remplis.                                       | Must     |
| CU2 | Jeton au coffre               | `2.1-a sexies` | Le jeton devient `{{token}}` dans les en-têtes ; une note le dit ; à l'envoi, il part au coffre (variable locale secrète de « Local »), jamais dans le projet.               | Must     |
| CU3 | Option signalée               | `2.1-a sexies` | « --compressed est ignorée : … » sous le formulaire.                                                                                                                         | Must     |
| CU4 | Échap                         | `2.1-a sexies` | Échap vide l'entrée et oublie la commande, jeton compris.                                                                                                                    | Should   |

## 5. Hors périmètre

- Liste de variables éditable en texte : l'onglet Variables vient du lot E ; `PairsEditor` porte
  déjà le mode texte (`text-format`).
- Variables de dossier et de collection à l'envoi : elles sont **connues** (R1) mais
  `request::resolve::prepare` ne les remplace pas encore (lot E, résolution des variables).
- cURL collée dans la barre d'adresse d'une requête, `-F` (multipart), cookies d'un fichier :
  signalés, non repris.
- Chrome « Copier en cURL (cmd) » (Windows, `^`) : seule la syntaxe bash est lue.

## 6. Architecture

### 6.1 Formats sur disque

Aucun format nouveau. Un essai collé avec un jeton écrit `Authorization: Bearer {{token}}` dans sa
requête et déclare `token` en variable secrète (sans valeur) de `environments/local.yml` de
« Non classés » ; la valeur est au coffre (`arnod/<projet>/Local/token`).

### 6.2 Cœur Rust

- `curl/` (pur) : `parse` (mots, options, secrets), `render` (guillemets, commande d'une requête).
- `input/` (pur) : `variables` (R1, R2), `added` (R3), `text` (R11 à R13).
- `request/inherit.rs` : `variable_names` (variables des dossiers et de la collection).
- `steps/` : règle de reconnaissance `curl` (définition REST : `{ "kind": "curl" }`, champ
  `headers` multiligne), `Detection.curl` (secrets, options ignorées), `http::with_secrets`.
- `http.rs` : `USER_AGENT`. `commands/trial.rs` : `trial_send` reçoit `secrets`.
- Aucune crate, aucun plugin, aucune permission ajoutés.

### 6.3 Commandes IPC

| Commande          | Paramètres                                   | Réponse        | Erreurs (`kind`)                            |
| ----------------- | -------------------------------------------- | -------------- | ------------------------------------------- |
| `request_input`   | `project`, `environment`, `path?`, `request` | `RequestInput` | `notFound`                                  |
| `request_curl`    | `project`, `environment`, `path?`, `request` | `CurlCopy`     | `notFound`, `invalidRequest`, `secretStore` |
| `pairs_to_text`   | `pairs`, `format` (`header`, `query`)        | `string`       | —                                           |
| `pairs_from_text` | `text`, `format`, `previous`                 | `PairsText`    | —                                           |
| `trial_send`      | + `secrets?` (nom → valeur)                  | `TrialResult`  | + `secretStore`                             |
| `steps_detect`    | inchangée                                    | + `curl?`      | —                                           |

`request_input` lit les environnements et `local.json` sans ouvrir le coffre (appelée à chaque
frappe, après 250 ms) ; `request_curl` lit les valeurs, coffre compris, pour les masquer.

### 6.4 Frontend

- Store `input` : dernière `RequestInput` par requête ouverte, `curl`, `addVariable`, texte.
- `lib/variables.ts` : découpage d'affichage et directive `v-variables` (calque transparent posé sur
  le champ, seul le soulignement se voit) ; les variables connues sont fournies par
  `RequestEditor` (`provide`) à `PairsEditor`, `BodyEditor`, `AuthEditor`.
- `PairsEditor` : `title`, `text-format` ; `RequestEditor` : bandeau, points, sections d'en-têtes,
  `</>` et sa fenêtre ; `TryStep` : note de la commande, secrets passés à `trial_send`.

### 6.5 Design

Aucun jeton nouveau : `--color-danger`, `--color-danger-soft`, `--color-subtle` (en-têtes
ajoutés), `--color-link` (variables du shell). Classes globales `.variable-marks` dans
`styles.css`. Clavier : `</>` est un bouton (Entrée), sa fenêtre se ferme par Échap ; « Éditer en
texte », « Revenir au tableau », « Ajouter », « Environnements… » sont des boutons.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                     |
| ------------------------------------- | -------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Saisie, texte et cURL ».                             |
| `src-tauri/steps/rest.json`           | Règle `curl`, champ `headers`.                                 |
| `src-tauri/capabilities/default.json` | Inchangé (presse-papiers par l'API du navigateur).             |
| `e2e/arnod.e2e.ts`                    | `describe("Lot B : saisie, texte et cURL")`, captures 21 à 23. |
| Installeur                            | Aucune dépendance ajoutée.                                     |

## 8. Tâches

- [x] **T1** Cœur : `curl/` (lecture, secrets, écriture) et `input/` (variables, en-têtes ajoutés,
      texte), tests règle par règle.
- [x] **T2** Reconnaissance cURL dans `steps/`, en-têtes du formulaire REST, secrets de
      `trial_send` au coffre ; User-Agent.
- [x] **T3** Commandes `request_input`, `request_curl`, `pairs_to_text`, `pairs_from_text` ; types
      et `ipc`.
- [x] **T4** Interface : soulignement, bandeau, points, en-têtes ajoutés (SA1 à SA5).
- [x] **T5** Interface : édition en texte, `</>` et Ctrl+Maj+C (TX1 à TX4).
- [x] **T6** Interface : cURL collée (CU1 à CU4).
- [x] **T7** Vitest (faux cœur `tests/fakeInput.ts`), aperçu (`5.1-b-ter`, `5.1-b-quater`,
      `2.1-a-sexies`), e2e, `maquette-etat.yml`, `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] `npm run check` passe.
- [ ] e2e « Lot B » sur l'application réelle (à l'intégration).
- [ ] Copie dans le presse-papiers vérifiée sous Windows (WebView2) et Linux (WebKitGTK).

## 10. Risques

| Risque                                                 | Impact                       | Parade                                                                              |
| ------------------------------------------------------ | ---------------------------- | ----------------------------------------------------------------------------------- |
| Calque de soulignement décalé du texte (polices, zoom) | Soulignement mal placé       | Styles recopiés du champ, hauteur de ligne du champ, recalcul au redimensionnement. |
| Presse-papiers refusé par la WebView                   | Ctrl+Maj+C sans effet        | La commande reste affichée, sélectionnable ; une notification le dit.               |
| Coffre indisponible (Linux sans Secret Service)        | Essai cURL avec jeton refusé | Erreur `secretStore` explicite ; l'e2e n'envoie pas de jeton.                       |
| `request_input` à chaque frappe                        | Lectures disque répétées     | 250 ms après la dernière frappe, sans le coffre ; réponse périmée ignorée.          |

## 11. Décisions

| ID  | Question                                                | Décision                                                                                                                                 |
| --- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Q1  | Envoi bloqué par une variable inconnue ?                | 2026-11-13 : non, comme la maquette (« Envoyer » reste actif) ; le cœur refuse l'envoi avec son message (`unknownVariable`).             |
| Q2  | Texte ↔ lignes : cœur ou interface ?                    | 2026-11-13 : cœur (`input::text`), la même lecture sert aux en-têtes du formulaire REST ; l'interface relaie.                            |
| Q3  | Où garder le jeton entre la reconnaissance et l'envoi ? | 2026-11-13 : dans l'interface, en mémoire (`Detection.curl.secrets`), jamais écrit ; le coffre ne reçoit que le jeton d'un essai envoyé. |
| Q4  | Le formulaire REST a-t-il un champ d'en-têtes ?         | 2026-11-13 : oui, multiligne `Nom: valeur`, pour tout essai REST (pas seulement cURL).                                                   |
| Q5  | User-Agent                                              | 2026-11-13 : `Arnod/<version>` envoyé par le client HTTP, comme la maquette ; certaines API refusent une requête sans.                   |
