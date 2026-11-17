# Plan - Qualité du MVP

**Version** 0.1 · **Date** 2026-11-17 · **Statut** Brouillon (Q5, Q10, Q11 tranchées) · **Base** version 0.0.3
(Develop @512f194)

Plan de correction des constats de l'[audit du MVP](audit-2026-11-mvp.md) : performance, qualité,
maintenabilité, design et ergonomie. Il n'ajoute aucune fonction : il rend le MVP sûr, fluide et
cohérent avant de le montrer. Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md) ; chaque lot
est une branche `claude/qualite-<lot>` et une pull request.

Source : `docs/audit-2026-11-mvp.md` (99 constats, cités par leur numéro : ES, PU, IN, PF, PI, MA,
DE, ER, AC) et ses preuves dans `docs/audit-2026-11-mvp/preuves/`. Les questions du § 11 sont à
trancher avant le lot qu'elles bloquent ; le lot A n'en dépend d'aucune.

## 1. Objectif

À la fin de ce plan, Arnod :

- ne se ferme plus et n'écrase plus un fichier de l'utilisateur sur les entrées que l'audit a
  trouvées, et envoie le corps choisi dans l'éditeur ;
- ne perd plus un brouillon à la fermeture d'une fenêtre, au rattachement ou à la sortie ;
- ne montre plus un secret encodé ni le mot de passe d'un proxy ;
- reste fluide sur un projet de 2 000 requêtes : frappe, enregistrement, run, réponse de 5 Mo ;
- est lisible dans les deux thèmes, se parcourt au clavier et offre « Annuler » partout où il
  supprime ;
- a des garde-fous qui attrapent ces régressions : miroir IPC vérifié, bancs de performance,
  contrôles de la CI.

## 2. Maquettes

La maquette est la référence (décision du 2026-11-17) : un écran que ce plan change est redessiné
d'abord (lot 0, consignes dans [`maquette-consignes-qualite.md`](maquette-consignes-qualite.md)),
et chaque divergence avec le code est notée dans `docs/maquette-etat.yml` en `aligner: code`,
avec son constat. Un lot qui livre la correction retire l'écart. Aucun écran n'est ajouté.

| Repère            | Redessiné                                                              | Constat       | Lot |
| ----------------- | ---------------------------------------------------------------------- | ------------- | --- |
| 0.5 (transverses) | bord de contrôle à 3:1, couleurs de texte, formes uniques              | DE4, AC6, DE7 | I   |
| 1.2-a             | infobulle d'un nom coupé, ⋯ au survol et au focus                      | DE5           | H   |
| 1.2-d             | quatre points de départ sur quatre colonnes                            | DE6           | I   |
| 1.6-e, 1.6-g      | bouton de fermeture ; « Vérifier maintenant » lisible en sombre        | ER4, DE2      | I   |
| 2.1-a             | un seul bouton plein, hauteurs de contrôle                             | DE9           | I   |
| 3.3-a, 3.4-a      | enregistrement sur place, « ✓ Enregistré »                             | ER2           | J   |
| 4.1-g, 4.2-c      | notes lisibles en sombre                                               | DE2           | I   |
| 4.2-a, 5.1-h      | barre des étapes sur une rangée qui défile                             | DE8           | I   |
| 4.2-b             | échec d'assertion affiché une fois, « Ignorer pour ce résultat »       | DE10, ER3     | I   |
| 5.1-b             | réponse vide en neutre, vue partagée sur une ligne, erreur avec action | DE1, DE3, ER6 | I   |

Écarts transverses ajoutés, sans écran redessiné : « Suppressions » (ER1, ER7, lot J) et « Clavier
et cibles » (AC1 à AC4, lots H et J). L'écart transverse du vocabulaire, résolu par le lot K du
MVP, est retiré ; l'écart de 1.2-d, résolu aussi, est remplacé.

## 3. Règles

Ces règles valent pour tous les lots.

| ID  | Règle                                                                                                                                                                                                                                       |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un constat se corrige avec son test de preuve : repris de `preuves/`, il échoue avant la correction et passe après. Un constat « lu dans le code » reçoit d'abord son test.                                                                 |
| R2  | Une entrée que l'utilisateur ne maîtrise pas (réponse d'un serveur, fichier d'un collègue, expression en cours de frappe) ne fait jamais paniquer une commande : erreur en français, jamais d'arrêt.                                        |
| R3  | Un fichier illisible n'est jamais réécrit, quel que soit son rôle (requête, environnement, contrat, état) ; une écriture ne remplace jamais le fichier d'un autre élément.                                                                  |
| R4  | Tout fichier d'état ou de poste s'écrit par une seule fonction : fichier temporaire puis renommage.                                                                                                                                         |
| R5  | Un secret est masqué sous toutes les formes où il part (brut, base64, encodé dans une adresse ou un formulaire) et n'apparaît dans aucun message, événement ni fichier.                                                                     |
| R6  | Une fenêtre ou un onglet ne se ferme pas sur un brouillon qui n'a pas pu être enregistré : `save` dit « enregistré », « suspendu » ou « échec », et l'appelant le montre.                                                                   |
| R7  | Budgets, mesurés par les bancs sur le projet de 2 000 requêtes, processeur ralenti 4× pour l'interface : 50 ms après une frappe ou un clic, 100 ms pour une ouverture, 16 ms par événement de run, 1 Mo par réponse IPC hors corps demandé. |
| R8  | Le coût d'une action ne dépend pas de la taille du projet quand son résultat n'en dépend pas : pas de relecture de l'arborescence pour un fichier, pas d'état complet pour un changement d'une ligne.                                       |
| R9  | Une liste qui peut dépasser 500 lignes (arbre, run, corps de réponse) ne monte que ses lignes visibles.                                                                                                                                     |
| R10 | Texte à 4,5:1, bord de contrôle et focus à 3:1, dans les deux thèmes ; chaque couple de jetons employé est vérifié par `tests/theme.spec.ts`.                                                                                               |
| R11 | Toute suppression se rattrape par « Annuler » dans la notification ; une saisie n'est jamais jetée sans le dire.                                                                                                                            |
| R12 | Un composant qui suit un motif WAI-ARIA (arbre, onglets, menu) en a le clavier complet ; le focus revient au déclencheur après une fermeture.                                                                                               |
| R13 | Un type échangé avec le cœur est vérifié par la CI des deux côtés ; un style partagé par plus de trois composants vit dans `styles.css` ou `components/ui/`.                                                                                |
| R14 | La maquette est la référence : un écran change d'abord dans la maquette ; le lot qui l'aligne dans le code retire son écart de `maquette-etat.yml` et compare son aperçu à l'écran redessiné (`scripts/apercu.mjs --maquette`).             |

## 4. Constats par lot

Priorités : **Must** (le MVP ne se montre pas sans), **Should** (attendu), **Could** (si le temps
le permet).

| Lot | Constats                                                                 | Résultat attendu                                                                                                                      | Priorité |
| --- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| A   | IN1, PU1, PU2, PU3, ES1, ES2, PU4, PU5, ES7, MA4                         | Le corps choisi part ; plus de fermeture sur JSONPath ou HTML ; plus de fichier écrasé ou caché ; `r6` stable ; clé réservée aux tags | Must     |
| B   | MA1, MA5, bancs de PF et de PI                                           | Miroir IPC vérifié en CI ; bancs de performance dans le dépôt ; CI alignée sur `npm run check`                                        | Must     |
| C   | IN2, IN3, IN4, IN5, IN6, IN9, ES8                                        | Aucun brouillon perdu ; une seule instance d'Arnod                                                                                    | Must     |
| D   | ES3, ES4, ES5, ES6, ES9, ES10, ES11, ES12, ES13                          | Secrets masqués sous toutes leurs formes ; cookies rangés sous leur hôte ; un jeton OAuth 2 par run                                   | Must     |
| E   | PU6 à PU15, ES14, MA12                                                   | Annulation complète ; contrat déduit conforme à sa réponse ; assertions Bruno justes ou signalées                                     | Should   |
| F   | PF1, PF2, PF4, PF5, PF8, PF9, PF10, PF11, PI5, PI8, PI9                  | Ouverture, frappe, enregistrement et lancement d'un run sous les budgets R7 à 2 000 requêtes                                          | Must     |
| G   | PF3, PF6, PF7, PI1, PI2, PI3, PI10                                       | Run de 2 000 requêtes sans gel ; réponse de 5 Mo affichée en moins de 100 ms                                                          | Must     |
| H   | PI4, PI6, PI7, DE5, AC1, AC9                                             | Barre latérale virtualisée, au clavier complet, noms lisibles                                                                         | Should   |
| I   | DE1, DE2, DE3, DE4, DE6, DE7, DE8, DE9, DE10, AC5, AC6, ER3, ER4         | Lisible dans les deux thèmes ; vue partagée qui tient à 1280 px ; un état, une forme                                                  | Must     |
| J   | ER1, ER2, ER5, ER6, ER7, ER8, IN7, IN8, AC2, AC3, AC4, AC7, AC8          | « Annuler » partout ; aucune saisie jetée ; focus rendu ; échecs toujours dits                                                        | Should   |
| K   | MA3, MA6, MA7, MA8, MA9, MA10, MA11, MA13, MA14, CSS partagée, code mort | Fichiers partagés découpés ; `CLAUDE.md` redevenu un guide ; façades testées                                                          | Should   |

## 5. Hors périmètre

- Toute fonction nouvelle, et les 14 écarts « à décider » de `maquette-etat.yml` autres que 3.3-a.
- La vérification sous Windows de chaque lot : elle reste due (MA13) et se fait sur un poste
  Windows, hors de ce plan, avant la version qui clôt le plan.
- Le chargement différé des écrans : le démarrage est mesuré fluide (23 ms).
- Les commentaires YAML conservés à l'enregistrement, si Q3 retient de l'écrire comme limite.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

Aucun format ne change. Trois fichiers de poste gagnent une écriture atomique (`last-run.json`) ou
un voisin : un résumé du dernier run à côté de `last-run.json` (PF11), et une zone d'attente du
poste pour les suppressions annulables (PU6). Un fichier verrou d'instance dans le dossier de
données, si Q1 le retient.

### 6.2 Cœur Rust

- **`storage::write_atomic`** : une seule fonction pour les huit écritures recopiées (R4, MA11).
- **Arborescence** : un contrôle de nom unique pour créer, renommer, déplacer et importer (PU4) ;
  lecture d'un dossier sans lire le projet (PF2, PF5) ; `TreeNode` porte le lien au contrat et le
  nombre d'exemples pour que `contract_list` ne relise rien (PF1).
- **Run** : événement différentiel (ligne, compteurs, révision) à la place de l'état complet ;
  statuts écrits une fois par run, diffusés par lot ; contrat analysé une fois par run (PF3, PF4).
  `RunItem` porte son groupe (écart d'architecture de `RunPanel.vue`).
- **Réponse** : les commandes `response_*` prennent `(projet, chemin)` et lisent le résultat gardé
  par le cœur ; mise en forme servie par plage de lignes ; profondeur d'indentation envoyée en
  nombre et plafonnée ; `ResultBook` borné en octets (PF6, PF7, PF10, PU2, PU3).
- **Réseau** : redirections suivies une étape à la fois dans `Network::send` (ES6) ; classement
  des échecs sans le texte de l'adresse (ES7) ; une demande de jeton par clé (ES11).
- **Secrets** : `Prepared.hidden` reçoit les formes dérivées quand elles sont produites (ES4).
- Nouvelle nature d'erreur possible : `alreadyRunning` (ES8, selon Q1). Dépendances : `ts-rs` en
  dépendance de test (Q2) ; `reqwest` 0.13 (Q9) ; rien d'autre.

### 6.3 Commandes IPC

| Commande                                                                  | Changement                                                                                  | Lot |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | --- |
| `response_render`, `response_filter`, `response_preview`, `response_save` | prennent `(projectId, path)` au lieu du corps ; `response_render` prend une plage de lignes | G   |
| `request_save`                                                            | renvoie si l'arbre doit être relu (nom, méthode, adresse, étiquettes)                       | F   |
| `tree_load`, `contract_list`                                              | une seule lecture du projet pour les deux                                                   | F   |
| `arnod://run`                                                             | porte un changement de ligne ; l'état complet reste lu par `run_load`                       | G   |
| `arnod://statuts`                                                         | porte les statuts changés, pas la carte entière                                             | G   |
| `arnod://projet`                                                          | porte les renommages et suppressions de requêtes                                            | C   |
| `arnod://fermer`                                                          | émis aussi avant de quitter et avant d'installer une mise à jour                            | C   |

### 6.4 Frontend

- **`save`** renvoie son issue ; `useTabActions`, `App.vue`, les fenêtres détachées et le store
  `updates` refusent de lâcher un brouillon non enregistré (R6).
- **Liste virtualisée** : un composant `components/ui/VirtualList.vue` (hauteur de ligne fixe)
  pour le corps de la réponse, le run puis l'arbre aplati (R9, Q7).
- **Arbre** : un menu partagé par panneau, entrées calculées à l'ouverture ; index chemin → nœud ;
  flèches, Début, Fin, F2, Suppr, Maj+F10 (PI4, PI9, AC1).
- **Garde d'action** : un composable `useGuard` qui notifie tout refus du cœur (IN8).
- **Styles partagés** dans `styles.css` : `:focus-visible` global, `.field`, `.hint`, `.truncate`,
  `.method--*` ; `ipc.ts`, `types/ipc.ts`, les scénarios de l'aperçu et les e2e découpés par
  domaine (MA7).

### 6.5 Design

- Nouveau jeton de bord de contrôle à 3:1, dans les deux thèmes, avec sa note d'usage dans
  `scripts/design-system.mjs` (DE4, Q10) ; `--color-accent-text` réservé aux fonds accent (DE2) ;
  `--color-subtle` réservé aux indications (AC6).
- Une forme de compteur et une forme de sélection (DE7) ; une forme d'ajout et une de retrait de
  ligne (ER8).
- `prefers-reduced-motion` et zoom de la WebView (`zoomHotkeysEnabled`).

## 7. Impacts sur l'existant

| Fichier                               | Changement                                                                                             |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `CLAUDE.md`                           | « Travail en cours » pointe ici ; lot K : sections par fonctionnalité déplacées dans `docs/reference/` |
| `README.md`                           | inchangé                                                                                               |
| `src-tauri/capabilities/default.json` | inchangé ; un plugin d'instance unique seulement si Q1 le retient                                      |
| `.github/workflows/build.yml`         | clé de signature réservée aux tags ; design system, audits, délais, miroir IPC, bancs                  |
| `e2e/`                                | un parcours par constat Must visible : corps JSON envoyé, rattacher enregistre, run de 500 requêtes    |
| `docs/maquette-etat.yml`              | écarts résolus retirés, écarts nés des lots H, I, J ajoutés                                            |
| Installeur                            | mesuré après le passage à `reqwest` 0.13 (attendu : plus petit)                                        |

## 8. Plan de tâches pour Claude Code

Un lot = une pull request, titrée en Conventional Commits. `npm run check` passe avant chaque
commit ; chaque tâche est relue avant commit. Vague 0 : lot 0, la maquette, sur claude.ai ; il
ne bloque que H, I et J. Vague 1 : A puis B, seuls. Vague 2, en parallèle :
C, D, E, F, I. Vague 3, en parallèle : G, H, J, K.

**Lot 0 - Maquette de référence** (session claude.ai, `docs/maquette-consignes-qualite.md`)

- [x] **T0a** Divergences notées dans `maquette-etat.yml` (`aligner: code`) ; un écran peut porter
      une liste d'écarts (`check-maquette.mjs`, `maquette-sync.py`).
- [x] **T0b** Écrans du § 2 redessinés dans la maquette, puis régénérée et republiée.

**Lot A - Urgences** (`claude/qualite-a-urgences`)

- [x] **T1** IN1 : `Body` aligné des deux côtés, test Rust de sa forme JSON, e2e qui envoie un
      corps JSON choisi dans l'éditeur.
- [x] **T2** PU1, PU2, PU3 : filtre JSONPath sans panique, aperçu et mise en forme bornés en
      profondeur.
- [x] **T3** ES1, ES2, PU4, PU5 : environnement illisible jamais réécrit, nom de fichier libre,
      contrôle de nom unique, renommage défait en cas d'échec.
- [x] **T4** ES7 : classement sans le texte de l'adresse ; test sur un port contenant 407.
- [x] **T5** MA4 : clé de signature passée aux seuls tags.

**Lot B - Garde-fous** (`claude/qualite-b-garde-fous`)

- [ ] **T6** MA1 : concordance des structures IPC vérifiée en CI (selon Q2) ; le faux cœur lit des
      échantillons JSON produits par le cœur.
- [ ] **T7** Bancs dans le dépôt : `src-tauri/tests/perf_coeur.rs` (`#[ignore]`) et
      `scripts/perf-interface.mjs`, avec les seuils de R7 ; lancés à la demande et chaque semaine.
- [ ] **T8** MA5 : design system, `cargo audit`, `npm audit`, `timeout-minutes`, couverture de
      `commands/` publiée ; SonarCloud selon Q8.

**Lot C - Brouillons et fenêtres** (`claude/qualite-c-brouillons`)

- [ ] **T9** IN3 : issue de `save` ; fermeture, rattachement, déplacement et installation refusent
      un brouillon non enregistré ; la fenêtre principale enregistre avant de quitter.
- [ ] **T10** IN2, IN4, IN9 : « Rattacher » enregistre ; choix du secret gardé ; frappe pendant
      une relecture conservée.
- [ ] **T11** IN5, IN6 : renommages et suppressions suivis dans toutes les fenêtres ; éditeurs
      sans onglet libérés.
- [ ] **T12** ES8 : instance unique (selon Q1).

**Lot D - Secrets et réseau** (`claude/qualite-d-secrets-reseau`)

- [ ] **T13** ES3, ES4, ES5 : variable de jeton par service, formes dérivées masquées, adresse du
      proxy sans identifiant.
- [ ] **T14** ES6, ES11 : redirections suivies une étape à la fois, jeton OAuth 2 demandé une fois.
- [ ] **T15** ES9, ES10, ES12, ES13 : annulation inscrite à l'entrée, renommage tout ou rien,
      écriture atomique, révision des statuts et des fenêtres.

**Lot E - Arborescence et formats** (`claude/qualite-e-formats`)

- [ ] **T16** PU6, PU7, MA12 : annulation complète d'une suppression groupée, liens non suivis,
      `watch::settle` partout.
- [ ] **T17** PU8, PU9, PU13, PU14 : schéma déduit de tous les éléments, chemin exact d'abord,
      `writeOnly`, grands entiers.
- [ ] **T18** PU10, PU11, PU12, PU15, ES14 : assertions Bruno, variables imbriquées (Q4),
      `request` d'un exemple renommé, mineurs.

**Lot F - Performance du cœur** (`claude/qualite-f-perf-coeur`)

- [ ] **T19** PF1, PF2, PI5 : arbre relu seulement s'il a changé ; `request_input` lit le dossier.
- [ ] **T20** PF4, PF5, PF8, PF9 : contrat par run, une lecture au lancement, noms lus une fois par
      lot, cibles d'un transfert.
- [ ] **T21** PF10, PF11, PI8, PI9 : résultats bornés en octets, résumé du dernier run, statuts
      fusionnés, index de l'arbre.

**Lot G - Run et réponse** (`claude/qualite-g-run-reponse`)

- [ ] **T22** PF3, PI1 : événements différentiels, groupe porté par la ligne, une révision
      appliquée par image, run virtualisé.
- [ ] **T23** PF6, PF7, PI2, PI3, PI10 : commandes par `(projet, chemin)`, mise en forme par plage,
      corps virtualisé, aperçu borné.

**Lot H - Barre latérale** (`claude/qualite-h-barre-laterale`)

- [ ] **T24** PI4, PI7 : menu partagé, arbre aplati et virtualisé, filtre différé.
- [ ] **T25** AC1, AC9, DE5, PI6 : clavier de l'arbre, noms accessibles, infobulles, calque des
      variables.

**Lot I - Lisibilité** (`claude/qualite-i-lisibilite`)

- [ ] **T26** DE1, DE2, DE4, AC5, AC6 : couleurs d'état et de texte, bord de contrôle (Q10),
      couples ajoutés à `tests/theme.spec.ts`.
- [ ] **T27** DE3, DE6, DE8, DE9 : vue partagée, points de départ, barre des étapes, accueil.
- [ ] **T28** DE7, DE10, ER3, ER4 : compteurs, sélection, échec affiché une fois, libellés.

**Lot J - Ergonomie et clavier** (`claude/qualite-j-ergonomie`)

- [ ] **T29** ER1, ER2, IN7 : « Annuler » des suppressions, environnements et réglages (Q5).
- [ ] **T30** IN8, ER6, ER7, ER8 : garde d'action, erreur avec son action, notification d'erreur
      qui reste, une forme d'ajout et de retrait.
- [ ] **T31** AC2, AC3, AC4, AC7, AC8, ER5 : focus rendu, onglets au clavier, cibles, zoom,
      mouvement réduit.

**Lot K - Maintenabilité** (`claude/qualite-k-maintenabilite`)

- [ ] **T32** MA7 : e2e, types IPC, `ipc.ts` et scénarios découpés par domaine.
- [ ] **T33** MA8, MA13 : `CLAUDE.md` en guide, références dans `docs/reference/`, plans cochés.
- [ ] **T34** MA10, MA9, MA11 : `workbench.rs`, `workbench.ts` et les quatre grands composants
      découpés ; `write_atomic` ; styles partagés ; code mort retiré.
- [ ] **T35** MA3, MA6, MA14 : logique des façades descendue et testée, `reqwest` 0.13 (Q9), faux
      minuteurs.

## 9. Critères d'acceptation

- [ ] Les 41 tests de preuve de l'audit sont dans le dépôt et passent.
- [ ] e2e : une requête POST dont le corps JSON est choisi dans l'éditeur arrive au serveur avec
      ce corps.
- [ ] e2e : une frappe suivie aussitôt de « Rattacher » est enregistrée.
- [ ] Banc du cœur à 2 000 requêtes : ouverture sous 250 ms, `request_input` sous 16 ms, suite
      d'un enregistrement sous 5 ms, run entier avec moins de 20 Mo d'événements.
- [ ] Banc de l'interface, processeur ralenti 4× : aucun événement de run au-delà de 16 ms, réponse
      de 5 Mo affichée sous 100 ms, frappe sous 50 ms.
- [ ] axe-core sur les 49 scénarios, deux thèmes : aucune violation critique ni sérieuse.
- [ ] Parcours au clavier seul, de la création d'un projet au run, sans perdre le focus.
- [ ] `npm run check` et la CI passent ; l'installeur reste sous 15 Mo ; vérification sous Windows
      faite et cochée.

## 10. Risques

| Risque                                                        | Impact                             | Parade                                                                           |
| ------------------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------- |
| Les lots F et G changent le contrat des événements `arnod://` | Régression entre fenêtres          | Bancs et tests de preuve posés d'abord (lot B) ; révision sur chaque événement   |
| La virtualisation casse le clavier ou le glisser de l'arbre   | Régression d'accessibilité         | Lot H après le lot G, qui rode le composant sur des listes sans interaction      |
| Les lots parallèles touchent `workbench.ts` et `workbench.rs` | Conflits de fusion                 | Découpage (lot K) après les corrections ; ajouts en fin de fichier d'ici là      |
| Un jeton de bord plus foncé change l'allure des écrans        | Écart avec la maquette             | Q10 tranchée avant le lot I ; design system et page 0.5 régénérés                |
| Rien n'est vérifié sous Windows                               | Défaut propre à WebView2 ou à NTFS | Vérification sur un poste Windows avant la version de clôture                    |
| Gains de performance estimés, non mesurés                     | Budget R7 manqué                   | Chaque tâche de performance se clôt sur une mesure du banc, pas sur l'estimation |

## 11. Décisions

Questions ouvertes, à trancher avant le lot cité.

| ID  | Question                                                                                                                                                  | Lot | Décision                                                                                               |
| --- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | --- | ------------------------------------------------------------------------------------------------------ |
| Q1  | Instance unique : `tauri-plugin-single-instance` (un plugin de plus, ramène la fenêtre) ou fichier verrou écrit par Arnod (sans plugin, message seul) ?   | C   | À trancher                                                                                             |
| Q2  | Miroir IPC : `ts-rs` en dépendance de test (types générés, rien dans le binaire) ou échantillons JSON vérifiés par `vue-tsc` (sans dépendance, partiel) ? | B   | À trancher                                                                                             |
| Q3  | Commentaires YAML perdus à l'enregistrement : les conserver (réécriture de la couche YAML, effort L) ou l'écrire comme limite connue ?                    | E   | À trancher                                                                                             |
| Q4  | Variable qui cite une autre variable : la résoudre comme Bruno (profondeur bornée, cycles refusés) ou la signaler inconnue ?                              | E   | À trancher                                                                                             |
| Q5  | Environnements et réglages du projet : enregistrement automatique comme le reste (résout l'écart 3.3-a) ou garde à la fermeture ?                         | J   | 2026-11-17 : enregistrement automatique                                                                |
| Q6  | Taille de projet visée par les budgets R7 : 2 000 requêtes et un poste ralenti 4×, ou une autre cible ?                                                   | F   | À trancher                                                                                             |
| Q7  | Liste virtualisée : composant maison à hauteur fixe ou dépendance (`@tanstack/vue-virtual`) ?                                                             | G   | À trancher                                                                                             |
| Q8  | SonarCloud : versionner sa configuration dans le dépôt, ou retirer ses badges du README ?                                                                 | B   | À trancher                                                                                             |
| Q9  | `reqwest` 0.13 pour n'avoir qu'un client HTTP : maintenant (lot K) ou à la prochaine montée de Tauri ?                                                    | K   | À trancher                                                                                             |
| Q10 | Bords de contrôle à 3:1 : foncer les bords de tous les champs, ou seulement cases, interrupteurs et focus ?                                               | I   | 2026-11-17 : cases, interrupteurs, radios et focus ; jeton `--color-border-control` (#8f8a83, #7d7871) |
| Q11 | Barre des étapes trop haute (DE8) : une rangée qui défile, ou « Étape n / total » avec un menu ?                                                          | I   | 2026-11-17 : une rangée qui défile                                                                     |
