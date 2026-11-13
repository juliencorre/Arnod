# Plan - MVP, lot A : corps de la réponse

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** version 0.0.2 (Develop @db2fbbd)

Plan du lot A de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : le corps de la réponse
reçue, écran `5.1-b bis` (page 5.1 « Envoi et réponse », fichier maquette « Arnod - Exécution »).
Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan
d'ensemble (cités ici « MVP-R… » pour les distinguer des règles du lot).

## 1. Objectif

À la fin de ce lot, dans le panneau de résultat d'une requête, Arnod :

- montre le corps dans un format détecté d'après `content-type` (JSON, XML, HTML, texte brut),
  modifiable, mis en forme, numéroté et coloré ; un HTML s'affiche aussi en aperçu, sans script
  ni ressource ;
- copie le corps, l'enregistre dans un fichier (sélecteur natif), efface la réponse affichée, et
  signale un corps tronqué à 5 Mo ;
- filtre le corps JSON par une expression JSONPath évaluée par le cœur : les valeurs trouvées sont
  surlignées, une expression invalide est expliquée en français, et la correspondance courante
  devient une vérification de la requête en un clic.

## 2. Maquettes

| Repère      | Capture (aperçu)                                           | Dans ce plan |
| ----------- | ---------------------------------------------------------- | ------------ |
| `5.1-b bis` | `node scripts/apercu.mjs 5.1-b-bis --maquette "5.1-b bis"` | F1 à F8      |

## 3. Modèle et règles

Le **chemin canonique** d'une valeur JSON s'écrit `$`, puis `.nom` (nom en lettres ASCII, chiffres
et `_`, sans chiffre en tête), `['nom spécial']` ou `[index]` : `$.items[0].sku`. La mise en forme
le donne à chaque valeur, le filtre à chaque correspondance ; l'interface surligne en comparant
des chemins, sans relire le JSON.

Les règles vivent dans le module pur `src-tauri/src/response/`, une ou plusieurs par test
(`response/tests.rs`).

| ID  | Règle                                                                                                                                                                                                                                                                                                                              |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Format détecté d'après `content-type` (`…/json`, `+json` ; `text/html` ; `…/xml`, `+xml` ; autre `text/…`) ; sans en-tête parlant (`text/plain` compris), d'après le corps : JSON lisible, `<!doctype html` ou `<html`, `<`, sinon texte. Le format choisi l'emporte.                                                              |
| R2  | JSON indenté de deux espaces ; un objet ou un tableau de valeurs simples tient sur une ligne s'il fait 72 caractères au plus. Un JSON illisible est montré tel quel, avec la ligne et la colonne de l'erreur. Au-delà de 1 Mo, pas de mise en forme.                                                                               |
| R3  | XML et HTML : une balise par ligne, indentée ; un texte court (80 caractères, une ligne) reste sur la ligne de sa balise ; éléments vides HTML et `<script>`/`<style>` en texte brut ; une balise jamais fermée reste du texte.                                                                                                    |
| R4  | Aperçu HTML : arbre de balises de mise en page sans attribut ; `script`, `style`, `head`, cadres, objets, `svg` et listes de formulaire retirés avec leur contenu ; une image devient `[image : alt]` ; une balise inconnue laisse son contenu ; 5 000 nœuds au plus. Ni `v-html` ni cadre.                                        |
| R5  | Filtre JSONPath selon la RFC 9535 (crate `serde_json_path`) : correspondances avec chemin canonique et valeur JSON compacte (200 caractères), 500 au plus, `total` les compte toutes. Expression vide : rien.                                                                                                                      |
| R6  | Vérification tirée d'une correspondance, cible `body` + chemin en notation pointée : valeur simple → « égal à », `null` → « est null », tableau → « a pour longueur », objet → « existe » ; origine « proposée ». Un nom qui ne s'écrit pas en notation pointée ne donne pas de vérification. Elle réussit sur la réponse filtrée. |
| R7  | Expression refusée ou corps non JSON : aucune correspondance et un message en français (`$` manquant, crochet, parenthèse ou guillemet non fermé, point final, caractère fautif).                                                                                                                                                  |
| R8  | Enregistrer : le cœur ouvre le sélecteur natif (`tauri-plugin-dialog`, depuis Rust), nom proposé = nom de la requête sans caractère interdit + extension du format (`reponse` si vide) ; le corps est écrit tel qu'il est affiché ; annuler n'écrit rien.                                                                          |
| R9  | Effacer : le dernier résultat de la requête est oublié par le cœur (`ResultBook::clear`) et diffusé (`arnod://resultat`) à toutes les fenêtres ; un envoi en cours n'est pas touché.                                                                                                                                               |
| R10 | Un corps tronqué à 5 Mo est signalé dans le corps, à la copie et à l'enregistrement (titre du sélecteur, notification).                                                                                                                                                                                                            |

## 4. Cas d'usage

| ID  | Cas d'usage       | Écran       | Comportement attendu                                                                                                                                                                         | Priorité |
| --- | ----------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| F1  | Lire le corps     | `5.1-b bis` | Liste « Format du corps » sur le format détecté (titre « Détecté : … »), lignes numérotées et colorées, raison si non mis en forme.                                                          | Must     |
| F2  | Changer de format | `5.1-b bis` | Choisir JSON, XML, HTML ou Texte brut remet en forme ; l'interrupteur « Aperçu » n'est actif qu'en HTML.                                                                                     | Must     |
| F3  | Filtrer           | `5.1-b bis` | Champ JSONPath sous le corps (corps JSON ou filtre déjà saisi) : « N résultats », valeurs surlignées, la courante cerclée ; Entrée / Maj+Entrée, ↑ ↓ ; Échap vide ; Ctrl+F y place le focus. | Must     |
| F4  | Erreur de filtre  | `5.1-b bis` | Message en français sous le champ (`role="alert"`), champ marqué invalide.                                                                                                                   | Must     |
| F5  | Vérification      | `5.1-b bis` | « + Vérification » ajoute au brouillon la vérification de la correspondance courante (enregistrée seule) ; déjà présente (même cible et opérateur) : notification, rien d'ajouté.            | Must     |
| F6  | Copier            | `5.1-b bis` | Le corps reçu est copié ; notification « Corps copié. » (« tronqué à 5 Mo » le cas échéant).                                                                                                 | Must     |
| F7  | Enregistrer       | `5.1-b bis` | Sélecteur natif, fichier écrit par le cœur, notification avec le chemin ; annulé : rien.                                                                                                     | Must     |
| F8  | Effacer           | `5.1-b bis` | La réponse disparaît de toutes les fenêtres ; « Envoyez la requête pour voir la réponse. »                                                                                                   | Must     |

## 5. Hors périmètre

- « Extraire dans une variable » depuis le filtre : lot E (extractions, MVP-R2) ; le bouton est
  absent d'ici là.
- Ctrl+J et l'icône près de l'horloge (résultat dessous ou à côté) : lot F ; la bascule existante
  « À droite · auto / Dessous » reste.
- Onglets « Envoyé » et « Chronologie » de la réponse, vocabulaire « assertion » (lot K).
- Joker dans une vérification (`body.items[*].sku contient …`) : les vérifications ne lisent que
  des chemins concrets.

## 6. Architecture

### 6.1 Formats sur disque

Aucun format nouveau. Une vérification ajoutée s'écrit comme les autres (`res.body.items[1].sku`
dans `runtime.assertions` du fichier OpenCollection). Le fichier enregistré est le corps brut.

### 6.2 Cœur Rust

- `response/format.rs` (R1, R8), `response/render.rs` (R2, R3 : jetons `Token { text, kind, path }`),
  `response/markup.rs` (lecture tolérante des balises, entités), `response/preview.rs` (R4),
  `response/filter.rs` (R5 à R7), `response/path.rs` (chemins canoniques et cibles).
- `results.rs` : `ResultBook::clear` (R9).
- Dépendance ajoutée : `serde_json_path` 0.7 (RFC 9535, chemins normalisés ; `regex` est déjà tiré
  par Tauri, `nom` et `inventory` s'ajoutent). Écrire un sous-ensemble à la main perdrait les
  filtres `?()` et les fonctions de la norme.
- Pas de nouvelle nature de `CommandErrorKind` : une expression invalide n'est pas une erreur de
  commande, elle revient dans `FilterResult.error`.

### 6.3 Commandes IPC

| Commande           | Paramètres                              | Réponse                     | Erreurs (`kind`) |
| ------------------ | --------------------------------------- | --------------------------- | ---------------- |
| `response_render`  | `body`, `headers`, `format` (ou `null`) | `RenderedBody`              | —                |
| `response_preview` | `body`                                  | `HtmlNode[]`                | —                |
| `response_filter`  | `body`, `expression`                    | `FilterResult`              | —                |
| `response_save`    | `body`, `name`, `format`, `truncated`   | chemin écrit, `null` annulé | `io`             |
| `response_clear`   | `project`, `path`                       | `RequestResult`             | `io`             |

### 6.4 Frontend

- Store `response` (`src/stores/response.ts`) : par onglet, format choisi, corps mis en forme,
  aperçu, expression (évaluée 150 ms après la dernière frappe), correspondances, courante ;
  actions copier, enregistrer, effacer, ajouter une vérification. Une réponse plus ancienne du
  cœur est ignorée.
- `ResponsePanel.vue` délègue l'onglet « Corps » à `ResponseBody.vue` (barre, corps, aperçu
  `HtmlPreview.vue`) et `ResponseFilter.vue`. Le surlignage est un affichage : un jeton est
  surligné si son chemin ou un de ses ancêtres est une correspondance.

### 6.5 Design

Jetons ajoutés aux deux thèmes, notes dans `scripts/design-system.mjs`, contrastes 4,5:1 vérifiés
(`tests/theme.spec.ts`) sur `color-surface`, `color-bg` et `color-match` : `--color-code-string`,
`--color-code-number`, `--color-code-keyword`, `--color-code-tag`, `--color-code-attr`,
`--color-match`. La correspondance courante ajoute un contour `color-accent`. Clavier : actions en
boutons, corps et aperçu focalisables, Ctrl+F, Entrée / Maj+Entrée, Échap.

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                               |
| ------------------------------------- | ---------------------------------------------------------------------------------------- |
| `CLAUDE.md`                           | Section « Corps de la réponse ».                                                         |
| `src-tauri/capabilities/default.json` | Inchangé : le sélecteur est appelé depuis Rust.                                          |
| `e2e/arnod.e2e.ts`                    | `describe` « Lot A - Réponse » (captures `a1-reponse-filtre`, `a2-reponse-apercu-html`). |
| `tests/apercu/scenarios.ts`           | Scénarios `5.1-b-bis`, `-filtre-invalide`, `-xml`, `-html`.                              |
| Installeur                            | `serde_json_path`, `nom`, `inventory` : quelques centaines de Ko au plus.                |

## 8. Plan de tâches

- [x] **T1** Module `response/` (R1 à R8) et `ResultBook::clear` (R9), tests règle par règle.
- [x] **T2** Commandes `response_*`, types miroirs, `ipc`, faux cœur (`tests/fakeResponse.ts`).
- [x] **T3** Store `response`, `ResponseBody`, `ResponseFilter`, `HtmlPreview` ; tests Vitest.
- [x] **T4** Jetons de coloration, design system, contrastes.
- [x] **T5** Aperçu Playwright comparé à `5.1-b bis` (clair, sombre, 900 px), e2e écrits,
      `maquette-etat.yml`, `CLAUDE.md`.

## 9. Critères d'acceptation

- [x] Une réponse JSON filtrée par `$.items[*].sku` surligne ses deux valeurs et ajoute
      `body.items[1].sku` égal à `"MUG-NOIR"` au fichier de la requête (e2e).
- [x] Un HTML avec `<script>` et une image distante s'affiche en aperçu sans rien exécuter ni
      charger (e2e).
- [x] `npm run check` passe.

## 10. Risques

| Risque                                | Impact           | Parade                                                                   |
| ------------------------------------- | ---------------- | ------------------------------------------------------------------------ |
| Corps de 5 Mo mis en jetons           | Interface lente  | Pas de mise en forme au-delà de 1 Mo (R2) ; 500 correspondances au plus. |
| Corps renvoyé au cœur à chaque filtre | IPC chargé       | Filtre différé de 150 ms ; réponses périmées ignorées.                   |
| Presse-papiers refusé par la WebView  | Copie impossible | Notification d'erreur ; enregistrer reste possible.                      |

## 11. Décisions

| ID  | Question                                      | Décision                                                                                                             |
| --- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Q1  | Crate JSONPath ou sous-ensemble à la main ?   | 2026-11-13 : `serde_json_path` (RFC 9535) ; messages d'erreur réécrits en français par le cœur.                      |
| Q2  | « + Assertion » sur `[*]` avec « contient » ? | 2026-11-13 : non, vérification de la correspondance courante, chemin concret (les vérifications n'ont pas de joker). |
| Q3  | Aperçu HTML : cadre `sandbox` ou arbre ?      | 2026-11-13 : arbre de balises sûres construit par le cœur, rendu par Vue (CSP stricte, ni script ni ressource).      |
| Q4  | Que fait « Aperçu » hors HTML ?               | 2026-11-13 : désactivé ; « Texte brut » montre le corps tel qu'il est reçu.                                          |
