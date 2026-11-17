# Audit du MVP - novembre 2026

**Date** 2026-11-17 · **Base** `Develop` au commit `512f194` (version 0.0.3) · **Statut** constats
rendus, rien n'est corrigé

Audit de la performance, de la qualité, de la maintenabilité, du design et de l'ergonomie du MVP :
cœur Rust (46 700 lignes, 136 fichiers) et interface Vue (34 000 lignes, 95 composants). Il suit
l'[audit du 11 novembre](audit-2026-11.md), fait à la version 0.0.2 sur un code deux fois plus
petit. Les corrections sont planifiées dans [`plan-qualite-mvp.md`](plan-qualite-mvp.md).

Sept revues ont été menées en parallèle, chacune tenue de prouver ses constats : test qui échoue
sur le code actuel, mesure, ou capture. Les preuves sont gardées dans
[`audit-2026-11-mvp/preuves/`](audit-2026-11-mvp/preuves/) (fichiers en `.txt` pour rester hors de
la compilation et du lint ; chaque lot du plan reprend les siens comme tests).

## 1. Verdict

Le socle est sain : tous les contrôles passent, aucune panique écrite à la main, aucune couleur en
dur, duplication sous 1 %, cœur hors Tauri couvert à 91 %, chaîne de mise à jour et TLS sans
reproche. L'audit a pourtant trouvé **99 constats**, dont quatre qui empêchent de montrer le MVP :

1. **Le type du corps d'une requête n'arrive jamais au cœur** : choisir « JSON » dans l'éditeur
   est sans effet dans l'application réelle (IN1).
2. **Arnod peut se fermer sur une entrée banale** : une expression JSONPath en cours de frappe, une
   réponse HTML très imbriquée (PU1, PU2).
3. **Des fichiers de l'utilisateur sont écrasés ou cachés** : environnement illisible réécrit,
   environnement de même nom de fichier, dossier renommé en nom réservé (ES1, ES2, PU4).
4. **L'interface se fige sur un grand projet** : un run de 2 000 requêtes fige l'affichage plus de
   huit minutes, une réponse de 1 Mo met 1,7 s à s'afficher (PI1, PI2).

Aucun de ces défauts n'est vu par les 1 089 tests existants : le faux cœur de Vitest recopie les
types de l'interface, les e2e ne tournent que sous Linux sur de petits projets, et rien ne mesure
la performance.

| Axe                    | Constats | Bloquants | Importants | Mineurs |
| ---------------------- | -------- | --------- | ---------- | ------- |
| Qualité du cœur        | 29       | 3         | 17         | 9       |
| Qualité de l'interface | 9        | 1         | 4          | 4       |
| Performance            | 21       | 0         | 15         | 6       |
| Maintenabilité         | 13       | 0         | 5          | 8       |
| Design et ergonomie    | 27       | 0         | 9          | 18      |

Le constat MA2 de la maintenabilité (test instable `r6`) est le même défaut qu'ES7 : il est compté
une fois, avec le cœur.

## 2. Contrôles au départ

| Contrôle                                       | Résultat                                    |
| ---------------------------------------------- | ------------------------------------------- |
| Versions, maquette, design system              | OK                                          |
| Prettier, ESLint, `vue-tsc` (app et e2e)       | 0 erreur                                    |
| Vitest                                         | 436 réussis en 8 s ; 0 échec sur 10 passes  |
| `cargo fmt --check`, `cargo clippy -D`         | 0 remarque                                  |
| `cargo test`                                   | 653 réussis en 3 s ; 0 échec sur 30 passes  |
| `cargo test` sur deux cœurs                    | 1 échec sur 15 (test `r6`, voir ES7)        |
| Couverture Vitest (instructions)               | 87,0 %                                      |
| Couverture Rust (lignes hors tests)            | 78,6 % ; cœur 91,2 % ; `commands/` 5,2 %    |
| Build de l'interface                           | JS 476 ko (150 ko gzip), CSS 186 ko (26 ko) |
| `npm run check` à froid                        | 89 s                                        |
| `npm audit --omit=dev`, `cargo audit`          | 0 vulnérabilité                             |
| Pipeline CI (17 réussites sur 20 exécutions)   | médiane 10 min 49 s                         |
| Tests de preuve de l'audit, sur le code actuel | 40 échouent sur 41 (§ 9)                    |

## 3. Qualité du cœur

Preuves : `preuves/coeur-es/audit_es.rs.txt` (rejoué deux fois : 13 échecs sur 14),
`preuves/coeur-purs/audit_constats.rs.txt` (18 échecs sur 18) et `audit_purs.rs.txt` (robustesse :
111 000 entrées engendrées). « Lu » : constat lu dans le code, sans test.

### 3.1 Fermetures de l'application

Le profil release a `panic = "abort"` : une panique dans une commande ferme Arnod, brouillons
compris.

| N°   | Constat                                                                                                                                                                                                                           | Où                                 | Effort   |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------- | -------- |
| PU1  | **Bloquant.** Le filtre JSONPath panique sur `\u` en fin d'expression (`$['\u`, début de `$['é']`) : `unreachable!` dans `serde_json_path` 0.7.2.                                                                                 | `response/filter.rs:61`            | S        |
| PU2  | **Bloquant.** L'aperçu HTML déborde la pile sur une réponse très imbriquée (200 000 `<div>` ouvertes, 1 Mo) : la limite de nœuds ne compte que les balises fermées.                                                               | `response/preview.rs:158`, `:186`  | S        |
| PU3  | La mise en forme HTML et XML prend une mémoire quadratique : 4 000 `<li>` non fermées (20 ko) donnent 32 Mo de jetons.                                                                                                            | `response/render.rs:268`, `:135`   | M        |
| PU15 | Lot de mineurs : `minLength: 4000000` fige la création d'une requête ; `decode_entities` quadratique ; contrat YAML de 5 Mo dit « illisible » ; `{{$date:+…d}}` déborde ; `$'a\041b'` mal lu ; `/oauth2/token` généralisé à tort. | `contract/sample.rs:181` et autres | S chacun |

### 3.2 Données perdues ou écrasées

| N°   | Constat                                                                                                                                                                            | Où                                                | Effort    |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | --------- |
| ES1  | **Bloquant.** Un environnement illisible (marqueurs de fusion Git) est réécrit par un « Local » vide : `init` le croit absent.                                                     | `environment.rs:228`                              | S         |
| ES2  | Créer « Recette-1 » écrase le fichier de « Recette 1 » : même nom de fichier, unicité vérifiée sur le nom seul.                                                                    | `environment.rs:58`, `:267`                       | S         |
| PU4  | Un dossier renommé, déplacé ou importé sous un nom réservé (`contracts`, `environments`) disparaît ; une collection Bruno avec un dossier `contracts` n'a plus de requête visible. | `tree/mod.rs:392`, `:449`, `bruno/convert.rs:813` | S         |
| PU5  | Renommage de dossier en deux temps sans retour arrière : si le second échoue, le dossier reste caché sous `.arnod-rename-<uuid>`.                                                  | `tree/mod.rs:407`                                 | S         |
| PU6  | « Annuler » une suppression groupée ne remet que les fichiers texte UTF-8 : image, autre encodage et dossier vide ne reviennent pas.                                               | `tree/batch.rs:69`, `tree/undo.rs`                | M         |
| PU7  | Les actions groupées suivent les liens en boucle : supprimer puis annuler écrit 41 copies.                                                                                         | `tree/batch.rs:62`, `:76`                         | S         |
| PU12 | Enregistrer efface les commentaires YAML ; renommer une réponse attendue perd sa partie `request`.                                                                                 | `opencollection/yaml.rs:74`, `request.rs:986`     | S, puis L |
| ES8  | Deux Arnod lancés ensemble s'écrasent `arnod.json`, `windows.json`, `statuses.json` : aucun garde d'instance unique.                                                               | `state.rs:454`, `lib.rs`                          | M         |
| ES10 | Renommage d'environnement à moitié appliqué si le coffre refuse : fichier renommé, valeurs locales vides.                                                                          | `workbench.rs:629`                                | S         |
| ES12 | `last-run.json` écrit sans fichier temporaire (lu) ; tronqué, la numérotation des runs repart à 1.                                                                                 | `workbench.rs:932`                                | S         |

### 3.3 Secrets

| N°   | Constat                                                                                                                                       | Où                                             | Effort |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- | ------ |
| ES3  | Le jeton d'un essai part vers un autre service : deux cURL collés donnent tous deux `{{token}}`, le second remplace le premier.               | `curl/parse.rs:570`, `commands/trial.rs:140`   | M      |
| ES4  | Un secret encodé n'est pas masqué dans « Envoyé » ni dans `arnod://resultat` : Basic en base64, paramètre d'adresse, formulaire.              | `request/resolve.rs:75`, `:434`                | M      |
| ES5  | Le mot de passe d'un proxy système (`HTTPS_PROXY=http://alice:…@hôte`) s'affiche dans le message d'échec, les préférences et `last-run.json`. | `network/mod.rs:188`, `network/failure.rs:254` | S      |
| ES6  | Un cookie posé pendant une redirection est perdu ; un cookie posé par l'hôte B après redirection est rangé sous A et renvoyé à A.             | `network/mod.rs:561`                           | M      |
| ES11 | Un run demande un jeton OAuth 2 par requête concurrente (8 demandes pour 8 requêtes).                                                         | `network/mod.rs:582`                           | S      |

### 3.4 Résultats faux

| N°   | Constat                                                                                                                                                                            | Où                                   | Effort |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ | ------ |
| ES7  | Une adresse ou un port contenant « 407 » classe un délai dépassé en « le proxy demande une identification, rien n'a été écrit sur le serveur ». Cause du test instable `r6` (MA2). | `network/failure.rs:229`             | S      |
| PU8  | Le contrat déduit d'un essai déclare non conforme la réponse dont il vient : schéma du premier élément seulement, tout est `required`.                                             | `contract/deduce.rs:123`, `:132`     | M      |
| PU9  | Un chemin littéral (`/orders/export`) est rattaché au chemin généralisé voisin (`/orders/{id}`).                                                                                   | `contract/deduce.rs:223`             | S      |
| PU10 | Une assertion Bruno sur une expression (`res.body.items.length: gt 0`) est importée fausse, sans mention au rapport.                                                               | `bruno/convert.rs:463`               | S      |
| PU11 | Une variable qui cite une autre variable part telle quelle (`https://api.test/{{version}}/orders`), sans erreur.                                                                   | `request/resolve.rs:158`             | M      |
| PU13 | Conformité : un champ `writeOnly` obligatoire est exigé dans la réponse.                                                                                                           | `contract/conformance.rs:249`        | S      |
| PU14 | Deux grands entiers différents sont dits égaux (comparaison en `f64`).                                                                                                             | `request/assertions.rs:139`          | S      |
| ES9  | « Annuler » pendant la lecture du coffre laisse partir la requête.                                                                                                                 | `workbench.rs:818`                   | S      |
| ES13 | Statuts et fenêtres diffusés hors verrou, sans révision (lu ; non reproduit sur 5 runs de 400 requêtes).                                                                           | `workbench.rs:502`, `windows.rs:291` | S      |
| ES14 | L'empreinte d'un fichier le lit en entier, sur le fil de la commande (lu ; non mesuré).                                                                                            | `watch/mod.rs:103`                   | M      |

### 3.5 Points ouverts de l'audit précédent

| N°  | État                                                                                          |
| --- | --------------------------------------------------------------------------------------------- |
| O1  | Ouvert et élargi : voir IN2 et IN3.                                                           |
| O2  | Résolu par le lot H : un client gardé par préférences, 8 connexions pour 2 900 requêtes.      |
| O3  | Ouvert, mesuré : voir PF4.                                                                    |
| O4  | Ouvert : les environnements ne lisent que `.yml` (`environment.rs:127`).                      |
| O5  | Ouvert : `precheck` puis `send` relisent environnements et coffre ; élargit la fenêtre d'ES9. |

## 4. Qualité de l'interface

Preuves : `preuves/interface/in-comportement.spec.ts.txt` et `in-secret-oublie.spec.ts.txt`
(9 tests, tous en échec). IN1 est prouvé par un programme d'essai qui sérialise `Body` : le cœur
écrit `{"type":"json"}` et lit `{"kind":"json"}` comme « sans corps ».

| N°  | Constat                                                                                                                                                                                                  | Où                                             | Effort |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- | ------ |
| IN1 | **Bloquant.** Le type du corps s'appelle `kind` dans l'interface et `type` dans le JSON du cœur, depuis le 9 novembre. Le choix de l'éditeur n'est jamais enregistré ; « Corps ● » est toujours affiché. | `request/model.rs:245`, `src/types/ipc.ts:425` | S      |
| IN2 | « Rattacher » n'enregistre rien : une frappe de moins de 600 ms, ou une source de contrat modifiée, part avec la fenêtre.                                                                                | `ContentWindow.vue:87`, `SidebarWindow.vue:34` | S      |
| IN3 | Un enregistrement suspendu (conflit, secret en attente) passe pour réussi : la fenêtre se ferme, le brouillon est perdu. La fenêtre principale n'enregistre jamais à la sortie (lu).                     | `workbench.ts:413`, `useTabActions.ts:36`      | M      |
| IN4 | « Laisser dans le test » est oublié à chaque changement d'onglet : le secret redevient « en attente », plus rien ne s'enregistre, sans bandeau visible.                                                  | `RequestEditor.vue:173`, `input.ts:54`         | S      |
| IN5 | Un éditeur ne suit pas un renommage ou une suppression faits dans une autre fenêtre : l'enregistrement automatique réécrit l'ancien chemin.                                                              | `workbench.ts:893`                             | M      |
| IN6 | Un éditeur n'est jamais libéré quand le cœur ferme ou remplace son onglet (réponse jusqu'à 5 Mo gardée).                                                                                                 | `useTabActions.ts:54`                          | M      |
| IN7 | « Annuler » d'un copier ou déplacer échoue sans message.                                                                                                                                                 | `transfer.ts:98`, `:129`                       | S      |
| IN8 | Huit boutons branchés sur une action asynchrone sans garde : un refus du cœur reste muet.                                                                                                                | `RunPanel.vue:252`, `ResponsePanel.vue:148`…   | S      |
| IN9 | Relire une requête écrase une frappe faite pendant l'appel.                                                                                                                                              | `workbench.ts:286`                             | S      |

Écarts à l'architecture : six règles métier calculées dans l'interface (groupe d'une ligne de run
dans `RunPanel.vue:171`, nom d'une variable extraite dans `workbench.ts:614`, dédoublonnage des
assertions, destinations d'un déplacement groupé…). Trois composants appellent `ipc` sans passer
par un store. `TreeNode.method` peut valoir `null` côté cœur et n'est pas optionnel côté interface
(`TreeRow.vue:328`).

## 5. Performance

### 5.1 Cœur

Banc : `preuves/perf-coeur/perf_coeur.rs.txt`. Profil release, Ryzen 9 9950X, Linux, cache chaud.
Projet de 2 000 requêtes (100 dossiers, 30 tests de 8 étapes, 10 contrats dont un de 1 Mo, 600
requêtes liées) et le même au quart.

| Opération                                   | 500 requêtes | 2 000 requêtes | Réponse IPC (2 000)  | Croissance  |
| ------------------------------------------- | ------------ | -------------- | -------------------- | ----------- |
| Ouverture d'un projet                       | 140 ms       | 545 ms         | 2,19 Mo              | linéaire    |
| Suite d'un enregistrement automatique       | 132 ms       | 522 ms         | 1,0 Mo               | linéaire    |
| `request_input` (après une pause de frappe) | 45 ms        | 166 ms         | 0,9 ko               | linéaire    |
| `request_save` seul                         |              | 0,11 ms        |                      | constant    |
| Lancement d'un run                          | 119 ms       | 471 ms         | 976 ko               | linéaire    |
| Run du projet entier                        | 0,45 s       | 5,36 s         | 2,83 Go d'événements | ×12 pour ×4 |
| Dupliquer 50 puis 200 requêtes d'un dossier | 0,28 s       | 4,37 s         |                      | quadratique |
| `response_render`, JSON de 983 ko           |              | 18,5 ms        | 10,62 Mo             | linéaire    |
| `request_send`, réponse de 4,93 Mo          |              | 56,6 ms        | 17,07 Mo             |             |
| `response_filter` `$..sku` sur 4,93 Mo      |              | 63,2 ms        | 5,69 Mo en argument  |             |

| N°   | Constat                                                                                                                                          | Où                                                   | Effort    |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------- | --------- |
| PF1  | Chaque enregistrement automatique fait relire tout le projet : `tree_load`, puis `contract_list`, qui relit l'arbre et chaque requête liée.      | `src/stores/workbench.ts:439`, `tree/mod.rs:141`     | S, puis M |
| PF2  | `request_input` relit toute l'arborescence pour trouver les étapes précédentes du test.                                                          | `commands/input.rs:68`, `tree/test.rs:101`           | S         |
| PF3  | Run : l'état complet repart deux fois par ligne (4 001 événements jusqu'à 976 ko) et `statuses.json` est réécrit à chaque ligne (277 Mo écrits). | `workbench.rs:1129`, `statuses.rs:127`               | M         |
| PF4  | Le contrat lié est relu et analysé à chaque ligne d'un run : 9,2 ms par ligne contre 0,21 ms (O3).                                               | `workbench.rs:883`, `contract/file.rs:772`           | S         |
| PF5  | Lancer un run lit trois fois chaque fichier.                                                                                                     | `workbench.rs:978`, `:986`, `:994`                   | S         |
| PF6  | `response_render` renvoie 11 à 14 fois le corps : chaque jeton est un objet, chaque valeur répète son chemin complet.                            | `response/render.rs:41`                              | M         |
| PF7  | Le corps d'une réponse traverse l'IPC à chaque usage : trois fois par envoi, une fois par filtre.                                                | `commands/requests.rs:94`, `commands/response.rs:18` | M         |
| PF8  | Dupliquer ou déplacer une sélection relit le dossier pour chaque élément, puis réécrit `windows.json` et `statuses.json` par élément.            | `tree/mod.rs:307`, `commands/batch.rs:63`            | S         |
| PF9  | `transfer_targets` relit arbre et contrats : 574 ms.                                                                                             | `tree/transfer.rs:141`                               | S         |
| PF10 | Les 50 derniers résultats sont bornés en nombre, pas en octets : 243 Mo pour 50 réponses de 5 Mo.                                                | `results.rs:18`                                      | S         |
| PF11 | `project_last_runs` analyse chaque dernier run en entier : 90 ms pour 200 projets.                                                               | `commands/runs.rs:71`                                | S         |

Rapide, à ne pas toucher : `request_save` (0,11 ms), l'observateur de fichiers (9,6 ms pour 2 000
fichiers), `AppState::mutate` (2,7 ms), la disposition (54 µs), l'import Bruno, `response_preview`.

### 5.2 Interface

Scripts : `preuves/perf-interface/`. Build de production de l'aperçu sur un faux cœur gonflé,
Chromium, médiane de 5 passes, puis processeur ralenti 4× (portable de bureau).

| Scénario                               | 500    | 2 000    | 2 000, ralenti 4× | Nœuds du DOM |
| -------------------------------------- | ------ | -------- | ----------------- | ------------ |
| Barre latérale, premier rendu          | 81 ms  | 249 ms   | 946 ms            | 20 996       |
| Filtre du référentiel, première frappe | 23 ms  | 89 ms    | 369 ms            |              |
| Arbre relu après un enregistrement     | 30 ms  | 144 ms   | 580 ms            |              |
| Frappe dans l'adresse                  | 4,4 ms | 10,6 ms  | 49 ms             |              |
| Un événement `arnod://run`             | 35 ms  | 225 ms   | 882 ms            | 42 197       |
| Interface figée pendant un run entier  | 22 s   | 525 s    | 2 087 s           |              |
| Réponse JSON de 100 ko                 |        | 210 ms   | 758 ms            | 44 719       |
| Réponse JSON de 1 Mo                   |        | 1 669 ms | 6 506 ms          | 443 791      |
| Réponse JSON de 5 Mo                   |        | 9 078 ms | 34 057 ms         | 2 210 959    |
| Correspondance JSONPath suivante       |        | 525 ms   | 2 231 ms          |              |
| Démarrage (JS compilé et évalué)       |        | 23 ms    | 91 ms             |              |

Aucune liste n'est virtualisée : arbre, run et corps de réponse sont rendus en entier.

| N°   | Constat                                                                                                                                        | Où                                         | Effort    |
| ---- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | --------- |
| PI1  | Run : chaque événement recalcule lignes × groupes² et re-rend tout ; à 2 000 lignes l'interface traite 3,7 lignes/s quand le cœur en finit 80. | `RunPanel.vue:180`, `:198`                 | S, puis M |
| PI2  | Corps de la réponse rendu en entier, un `span` par jeton, chaque jeton réactif : +1,17 Go à 5 Mo.                                              | `ResponseBody.vue:179`, `response.ts:53`   | M         |
| PI3  | Filtre JSONPath : « suivant » re-rend tout le corps.                                                                                           | `ResponseBody.vue:78`                      | S         |
| PI4  | Barre latérale : chaque ligne monte son menu et son sous-menu « Déplacer vers » (5 360 composants à 2 000 requêtes).                           | `TreePanel.vue:276`, `TreeRow.vue:84`      | S, puis M |
| PI5  | L'enregistrement automatique re-rend tout l'arbre (suite de PF1).                                                                              | `workbench.ts:439`, `TreeRow.vue:92`       | S         |
| PI6  | Soulignement des variables : calque reconstruit à chaque frappe, mise en page forcée sur les 200 champs de `PairsEditor`.                      | `lib/variables.ts:84`, `:101`              | S         |
| PI7  | Filtre du référentiel : la première frappe et Échap re-rendent toutes les lignes.                                                              | `TreePanel.vue:52`, `lib/treeFilter.ts:38` | S         |
| PI8  | `arnod://statuts` remplace toute la carte : chaque ligne et chaque dossier recalculent.                                                        | `workbench.ts:1086`, `lib/status.ts:30`    | S         |
| PI9  | La frappe coûte plus cher quand le projet grossit : `findRequest` parcourt l'arbre à chaque rendu d'onglet.                                    | `ZoneView.vue:67`, `workbench.ts:124`      | S         |
| PI10 | Aperçu HTML sans limite : 42 436 éléments en 1,5 s pour 2 Mo.                                                                                  | `HtmlPreview.vue:54`                       | S         |

Fluide, à ne pas toucher : le démarrage (charger des écrans à la demande gagnerait moins de 5 ms),
`isDirty` (0,3 ms), la frappe sans variable inconnue sur un projet de 500 requêtes.

## 6. Maintenabilité

Relevés et scripts de mesure : commandes citées dans le tableau, rejouables.

| Indicateur                                         | Valeur                                                        | Verdict      |
| -------------------------------------------------- | ------------------------------------------------------------- | ------------ |
| `unwrap()`, `panic!`, `unsafe`, `todo!` hors tests | 0 ; un seul `expect(`                                         | sain         |
| TODO, FIXME, code commenté, `any`, tests ignorés   | 0                                                             | sain         |
| Duplication (`jscpd`, 50 jetons)                   | `src` 0,77 % ; Rust 0,88 % ; `tests` 2,76 % ; `e2e` 5,17 %    | sain         |
| Fonctions Rust hors tests                          | 1 228 ; médiane 8 lignes ; 15 de 100 lignes ou plus           | correct      |
| Fichiers de plus de 600 lignes                     | 23, dont 6 de tests                                           | à surveiller |
| Crates                                             | 523 au verrou ; 17 en double, dont `reqwest` 0.12 et 0.13     | à corriger   |
| Miroir IPC                                         | 197 types appariés, 2 divergences, 0 structure testée sur 155 | à corriger   |
| Couverture de `commands/`                          | 5,2 %                                                         | façade nue   |
| `CLAUDE.md`                                        | 825 lignes, touché par 58 commits sur 236                     | journal      |

| N°   | Constat                                                                                                                                                                                                                                          | Effort |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------ |
| MA1  | Aucune des 155 structures échangées n'a de test de concordance entre Rust et TypeScript ; c'est ce qui a laissé passer IN1.                                                                                                                      | M      |
| MA3  | La couche Tauri n'est presque pas testée : 14 façades dépassent 30 lignes (`updates_download` 92, `trial_send` 69), vérifiées seulement par les e2e sous Linux.                                                                                  | L      |
| MA4  | La clé de signature des mises à jour est passée à chaque build de la CI, tags ou non (`build.yml:125`) : exposée aux scripts de build de toutes les dépendances.                                                                                 | S      |
| MA5  | Écarts entre `npm run check` et la CI : contrôle du design system absent de la CI ; ni audit des dépendances, ni couverture, ni `timeout-minutes` ; SonarCloud sans fichier dans le dépôt.                                                       | S à M  |
| MA7  | Les fichiers partagés freinent le travail en parallèle : `e2e/arnod.e2e.ts` (3 185 lignes, un commit sur trois), `src/types/ipc.ts` (1 927), `tests/apercu/scenarios.ts` (1 607).                                                                | L      |
| MA6  | Deux clients HTTP dans le binaire : `reqwest` 0.12 (Arnod) et 0.13 (`tauri-plugin-updater`).                                                                                                                                                     | M      |
| MA8  | `CLAUDE.md` est devenu un journal : 5 sections de guide, 25 sections par fonctionnalité ; `updates.rs` y est dit « pur » alors qu'il écrit `updates.json`.                                                                                       | M      |
| MA9  | Quinze fonctions de 100 lignes ou plus (`assertions::evaluate` 212, `conformance::walk` 187, `transfer::preview` 183).                                                                                                                           | M      |
| MA10 | Cinq gros fichiers mêlent plusieurs rôles : `workbench.rs`, `opencollection/request.rs`, `bruno/convert.rs`, `startup.rs`, `request/assertions.rs` ; côté interface `workbench.ts`, `RequestEditor.vue`, `ZoneView.vue`, `ContractsSection.vue`. | L      |
| MA11 | Huit écritures « fichier temporaire puis renommage » recopiées ; huit serveurs locaux recopiés dans les e2e.                                                                                                                                     | S à M  |
| MA12 | Dix écritures dans un projet sans `watch::settle` (créations de dossier, suppressions de la migration).                                                                                                                                          | S      |
| MA13 | 35 tâches non cochées dans huit plans, dont les vérifications sous Windows de quatre lots.                                                                                                                                                       | S      |
| MA14 | Attentes réelles dans les tests : 13 `setTimeout` dans Vitest (jusqu'à 2,3 s par test), 8 `sleep` côté Rust.                                                                                                                                     | S      |

Le CSS de 186 ko est à 97 % du style `scoped` répété : l'anneau de focus est réécrit dans 62
composants, 164 blocs de déclarations sont identiques. L'enjeu est la maintenance, pas le poids.
Code mort relevé : `ImportStart.vue`, `ipc.runStatus`, `ipc.hello`, `formatTime`, `emptyOAuth2`,
six fonctions des stores `windows` et `network`.

## 7. Design et ergonomie

Captures : 221 images des 49 scénarios de l'aperçu (deux thèmes, 1280 × 775, 960 × 600 et
1920 × 1080), non versées au dépôt ; axe-core 4.14 sur 98 écrans.

### 7.1 Design

| N°   | Constat                                                                                                                                   | Critère              | Où                                                                        | Effort |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------- | -------------------- | ------------------------------------------------------------------------- | ------ |
| DE1  | Messages neutres en rouge d'erreur : « Envoyez la requête pour voir la réponse. », « Envoi en cours… ».                                   | design system, états | `ResponsePanel.vue:191`                                                   | S      |
| DE2  | Trois textes illisibles en thème sombre (1,06:1 et 1,27:1) : « Vérifier maintenant », note du copier-déplacer, note du coffre.            | WCAG 1.4.3           | `UpdatesSection.vue:394`, `CopyMoveDialog.vue:398`, `SecretNotice.vue:87` | S      |
| DE3  | Vue partagée trop serrée à 1280 × 775 : adresse de 274 px, onglets de l'éditeur et du résultat sur deux rangées ; à 960 × 600 tout tient. | plan-qualite C3      | `RequestTab.vue:33`                                                       | M      |
| DE4  | Bords des contrôles sous 3:1 : `border` 1,29 et 1,26, `border-strong` 1,65 et 1,69, interrupteur éteint 1,19 et 1,14.                     | WCAG 1.4.11          | `src/styles.css:20`, `ToggleSwitch.vue:52`                                | M      |
| DE5  | Noms coupés dans la barre latérale, sans infobulle (« Créer l… », « Ch… » à 200 px).                                                      | Nielsen              | `TreeRow.vue:324`                                                         | M      |
| DE6  | « Par où commencer ? » : cinq colonnes pour quatre cartes, bouton sur deux lignes.                                                        | R2                   | `ProjectWelcome.vue:181`                                                  | S      |
| DE7  | Un même état écrit de plusieurs façons : compteur en pastille, « (1) » ou point ; sélection en fond, soulignement ou plein noir.          | R5                   | `StepBar.vue:131`, `RequestEditor.vue:480`                                | S      |
| DE8  | La barre des étapes passe sur trois rangées dès quatre étapes.                                                                            | densité              | `StepBar.vue:88`                                                          | M      |
| DE9  | Deux boutons pleins et hauteurs hors échelle à l'accueil (37 et 42 px).                                                                   | R5, R2               | `TryStep.vue`, `ProjectForm.vue`                                          | S      |
| DE10 | Un échec d'assertion est affiché deux fois ; pluriel « assertion(s) ».                                                                    | Nielsen              | `ResponsePanel.vue:87`, `:163`                                            | S      |

### 7.2 Ergonomie

| N°  | Constat                                                                                                                | Où                                                            | Effort |
| --- | ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | ------ |
| ER1 | Suppressions sans « Annuler » : dossier ou test, contrat, environnement. La sélection multiple, elle, l'offre.         | `TreePanel.vue:186`, `contracts.ts:301`                       | M      |
| ER2 | Saisies perdues sans avertissement : Échap dans Environnements ou dans les réglages du projet jette les modifications. | `EnvironmentsDialog.vue:167`, `ProjectSettingsDialog.vue:108` | M      |
| ER3 | « Ignorer pour ce run » sur un envoi seul.                                                                             | `ResponsePanel.vue:163`                                       | S      |
| ER4 | Préférences sans bouton de fermeture.                                                                                  | `PreferencesDialog.vue:58`                                    | S      |
| ER5 | À l'accueil, le focus arrive sur « Nom du projet » ; « Essayer une étape », en tête, est à quatre Maj+Tab.             | `ProjectForm.vue:122`                                         | S      |
| ER6 | Une erreur d'envoi s'affiche sans action possible.                                                                     | `ResponsePanel.vue:69`                                        | S      |
| ER7 | Une notification d'erreur disparaît en 6 s (lu).                                                                       | `stores/toasts.ts:19`                                         | S      |
| ER8 | Ajouter et retirer une ligne : trois formes d'ajout, cinq formes de retrait.                                           | plusieurs composants                                          | M      |

### 7.3 Clavier et accessibilité

| N°  | Constat                                                                                                                                             | Où                      | Effort |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | ------ |
| AC1 | L'arbre (`role="tree"`) n'a ni flèches, ni Début, ni Fin, deux arrêts par ligne : 34 Tab pour atteindre l'adresse. Maj+F10, F2 et Suppr sans effet. | `TreeRow.vue:303`       | M      |
| AC2 | Le focus retombe sur `body` après Échap dans Environnements, après la création d'un projet, après un essai et après ▶.                              | plusieurs composants    | S      |
| AC3 | Onglets de l'éditeur et de la réponse sans flèches ni `tabindex` itinérant (13 arrêts).                                                             | `RequestEditor.vue:480` | S      |
| AC4 | Cibles sous 24 px : « TEST » 39 × 13, retrait d'étiquette 11 × 20, × d'une ligne 23 × 15.                                                           | plusieurs composants    | S      |
| AC5 | Poignée de séparation : focus en `--color-accent` (2,28:1) et bouton « Détacher » imbriqué dans un élément interactif (36 écrans).                  | `SplitHandle.vue:207`   | S      |
| AC6 | `--color-subtle` sur du texte à lire : 3,27:1 et 3,5:1.                                                                                             | `TreePanel.vue:389`     | S      |
| AC7 | Pas de zoom : `zoomHotkeysEnabled` absent ; à 150 % et 200 % simulés, rien n'est hors d'atteinte.                                                   | `tauri.conf.json`       | S      |
| AC8 | Pas de `prefers-reduced-motion` (8 transitions de 200 ms au plus).                                                                                  | `src/styles.css`        | S      |
| AC9 | Lignes d'un contrat nommées « Ouvrir GET /orders » sans leur texte visible.                                                                         | `ContractsSection.vue`  | S      |

axe-core : `nested-interactive` et `region` sur 36 écrans (poignée), `aria-required-children` sur 29
(`ul[role=tree]`, onglets de zone), `heading-order` sur 21, `color-contrast` sur 11.

Respect du design system dans le code : 0 couleur en dur, 0 jeton inutilisé, graisses 400, 600 et
700 seulement. Restent 16 hauteurs hors échelle (26, 30, 36, 38 px), 12 hauteurs qui recopient un
jeton, trois rayons `50%`, et 127 `<button>` natifs pour 115 `BaseButton`, dont une quinzaine de
liens habillés à la main.

Écarts connus avec la maquette : 5 « aligner le code » (dont deux entrées périmées de
`maquette-etat.yml` : 1.2-d et le vocabulaire) et 14 « à décider ».

## 8. Ce qui est sain

- **Sécurité de la chaîne de mise à jour et du TLS** : version du manifeste contrôlée, signature
  liée à la version, paquet refusé jamais gardé ; aucun chemin ne coupe la vérification TLS.
- **Fichiers d'état** : `arnod.json` et `windows.json` écrits de façon atomique, jamais réécrits
  s'ils sont illisibles ou plus récents ; `tree::resolve` refuse `..` et les chemins absolus.
- **Format strict tenu** : sur 7 233 requêtes mutées, aucune ne devient non conforme à
  l'enregistrement ; scripts, variables et authentifications non gérés sont conservés.
- **Parseurs écrits à la main** (cURL, `.bru`, variables, chemins) : aucune panique d'index ni de
  découpe de caractère sur 111 000 entrées ; `$ref` circulaires bornés.
- **Frontière IPC** : un seul point d'accès, erreurs typées, écouteurs tous retirés au démontage ;
  les corrections F1 à F3 de l'audit précédent tiennent.
- **Outillage** : aucune vulnérabilité de production, licences compatibles avec MIT, Dependabot sur
  trois écosystèmes, actions épinglées par SHA, `npm run check` en 89 s.
- **Design** : échelles tenues à l'exécution sur 49 écrans, statuts jamais par la couleur seule,
  écrans d'erreur qui disent la cause et l'issue, première réponse en un geste.

## 9. Limites de cet audit

- **Linux seulement.** Ni Windows (NSIS, WebView2, registre du proxy, pile de 1 Mo, lectures de
  fichiers plus lentes), ni l'application réelle, ni les e2e n'ont été lancés. Les effets d'IN1
  dans l'application sont déduits du code et du programme d'essai.
- **Budget.** Les revues ont été interrompues une fois par la limite d'usage et ont conclu avec un
  budget compté. Non couvert : `layout/`, `catalog/`, `opencollection/migrate.rs`,
  `request/inherit.rs` ; `WorkspaceSwitcher.vue`, `BaseMenu.vue` et les composants de `manage/`,
  `home/`, `import/`, `startup/` lus par sondage ; 29 interfaces IPC sans structure Rust homonyme.
- **Design.** 55 des 98 captures à 1280 px ont été regardées à l'œil ; les mesures automatiques
  couvrent les 98. Parcours clavier non joués jusqu'au bout : ajouter une étape, une assertion,
  lancer un run, changer d'environnement. Lecteur d'écran, fenêtres détachées et glisser-déposer
  non vus.
- **Performance.** Chromium et non WebKitGTK ni WebView2 ; transport IPC réel et démarrage de
  l'application non mesurés ; onglets et zones (40 onglets, séparations) non mesurés ; build
  release et taille du binaire non mesurés (disque plein). Les gains annoncés au plan sont des
  estimations.
- **Tests de preuve.** 40 échouent sur 41 : 13 sur 14 pour le disque et le réseau (la course ES13
  n'est pas reproduite), 18 sur 18 pour les modules purs, 9 sur 9 pour l'interface. Seuls ceux du
  disque et du réseau ont été rejoués une seconde fois, indépendamment.
