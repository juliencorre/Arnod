# Plan - Alignement du code sur la maquette

**Version** 0.1 · **Date** 2026-11-26 · **Statut** En cours · **Base** version 0.0.4 (Develop @4b858c7,
PR #81)

Plan de correction des écarts `aligner: code` de [`maquette-etat.yml`](maquette-etat.yml) relevés
par la vérification du 26 novembre 2026 (88 écrans livrés ou en partie comparés aux captures e2e et
à l'aperçu). Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md). Les 20 écarts déjà suivis par
[`plan-qualite-mvp.md`](plan-qualite-mvp.md) (constats DE, ER et AC) y restent et sont traités par
ses lots I, J et la tâche T25 du lot H, dans l'ordre du § 8 ; ce plan porte les 46 autres.

Source : la maquette « Application Tests Automatiques » (version 209, régénérée le 26 novembre) ;
les écrans sont cités par leur repère (`3.2-a2`, voir [`plan-suivi-maquette.md`](plan-suivi-maquette.md)).
Chaque écart est le texte de son entrée dans `maquette-etat.yml`.

## 1. Objectif

À la fin de ce plan, plus aucun écran livré n'a d'écart `aligner: code` :

- les textes, libellés et formes de l'application sont ceux de la maquette ;
- la réponse, l'envoyé et l'écran d'un run montrent ce que la maquette montre (statut en pastille,
  environnement, fil d'Ariane, date de fin, valeurs tirées surlignées) ;
- un dossier s'ouvre dans une vue qui liste ses sous-dossiers et ses tests dans l'ordre
  d'exécution, se réordonne par ⋮⋮ et s'exécute au survol ;
- un environnement se crée dans une fenêtre (adresse de base, variables reprises, place dans la
  chaîne), et une requête qui hérite d'une authentification la montre en carte ;
- l'import fait correspondre les environnements, laisse choisir où vont les secrets, et un script
  Bruno non repris se remplace par une assertion ou s'accepte sans lui.

## 2. Maquettes

| Repère                                           | Dans ce plan                       | Lot |
| ------------------------------------------------ | ---------------------------------- | --- |
| 0.3-b, 1.2-d, 2.1-a, 2.1-a ter                   | textes et libellés                 | A   |
| 2.1-a sexies, 2.3-b, 3.1-f, 3.2-c                | textes et libellés                 | A   |
| 3.3-0, 4.2-c, 4.4-d, 5.1-c                       | textes et libellés                 | A   |
| 1.1-a, 1.1-b bis, 1.1-d, 1.1-e                   | formes et dispositions             | B   |
| 1.2-a bis, 1.2-e, 1.3-a, 1.3-c                   | formes et dispositions             | B   |
| 1.4-b, 1.4-h, 2.2-b, 2.2-d, 2.2-f                | formes et dispositions             | B   |
| 3.1-e, 3.2-a, 3.2-a2, 3.4-a, 4.1-a quater, 4.1-g | formes et dispositions             | B   |
| 5.1-b, 5.1-h, 5.2-e, 5.2-f, 3.3-c, 4.2-b bis     | réponse, envoyé, run, confirmation | C   |
| 3.3-a, 3.3-b, 3.4-b                              | environnements et authentification | D   |
| 4.1-a, 4.1-a bis, 4.1-a ter, 5.2-d               | dossiers                           | E   |
| 1.1-c, 2.3-b, 2.3-f                              | import                             | F   |

## 3. Règles

Ces règles valent pour tous les lots, en plus de R1 à R14 de `plan-qualite-mvp.md`.

| ID  | Règle                                                                                                                                                                                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Un écart se corrige avec son test de preuve, qui échoue avant et passe après : Vitest pour un texte ou un comportement, e2e (couleur ou position calculée, comme `countTone`) pour un style que Vitest ne calcule pas.        |
| R2  | Le lot retire l'écart corrigé de `maquette-etat.yml` (ou réécrit ce qui reste), rend son état à l'écran (« livré » quand plus rien ne manque) et ajoute une ligne datée au journal de la page.                                |
| R3  | Seule la fonction de l'écran s'aligne : ce que la maquette montre d'autres versions (versions de test « v3 », campagnes, anomalies, mocks, templates, Windows) reste hors périmètre, comme en R2 de `plan-suivi-maquette.md`. |
| R4  | Un texte d'interface suit le vocabulaire de la page 0.2 (test, étape, assertion, projet) ; aucun nombre n'est accordé à la main : une fonction de `lib/format.ts` accorde au singulier et au pluriel.                         |
| R5  | Un type échangé nouveau ou changé reçoit sa dérivation ts-rs et son échantillon (`ts_mirror`) ; aucun plugin ni permission ajoutés.                                                                                           |
| R6  | Chaque lot compare son aperçu à l'écran de la maquette (`scripts/apercu.mjs <scénario> --maquette <repère>`) et ajoute ou met à jour la capture e2e de l'écran qu'il change.                                                  |

## 4. Cas d'usage

Identifiants AM (alignement sur la maquette). Priorité Must sauf mention.

**Lot A - Textes et libellés**

| ID   | Écran        | Comportement attendu                                                                                                                                                                                  |
| ---- | ------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AM1  | 0.3-b        | Les textes du cœur (`startup::describe`) mettent une espace insécable après « et avant » : jamais une ligne qui finit par un guillemet ouvrant.                                                       |
| AM2  | 1.2-d        | Carte « Écrire un test » : « Directement dans le référentiel de tests, sans contrat. » ; carte du contrat : « Vous y écrirez vos premiers tests. ».                                                   |
| AM3  | 2.1-a        | Formulaire du projet : indication « par exemple boutique-tests » ; « Un workspace est facultatif ; vous pourrez ranger le projet plus tard. », sans la phrase répétée sous le bouton.                 |
| AM4  | 2.1-a ter    | La note à droite de l'entrée dit la nature reconnue (« Reconnu : une requête REST. ») ou « Nature choisie : le formulaire s'ouvre en dessous. » ; la rangée compacte dit « ⇩ Importer l'existant… ».  |
| AM5  | 2.1-a sexies | En-tête du formulaire d'une commande cURL : « REST · requête », « repris de la commande, à vérifier avant l'envoi » ; la pastille REST de « Ou choisissez » est cerclée quand la nature est reconnue. |
| AM6  | 2.3-b        | « 10 vérifications deviendront des assertions sans code ».                                                                                                                                            |
| AM7  | 3.1-f        | Titre « Nouveau contrat de service », bouton « Créer le contrat ».                                                                                                                                    |
| AM8  | 3.2-c        | Sous-titre « La requête « … » devient l'étape 1 d'un nouveau test du référentiel. » ; « la requête et ses 2 assertions », « et son assertion », accordés (R4).                                        |
| AM9  | 3.3-0        | En-tête du menu de l'environnement : « Environnements · <projet> ».                                                                                                                                   |
| AM10 | 4.2-c        | Le bouton qui garde le secret dans la requête dit « Laisser dans la requête » pour une requête libre, « Laisser dans le test » pour une étape.                                                        |
| AM11 | 4.4-d        | Les champs du formulaire d'une nature sont sous « Formulaire de l'étape » ; « Ce qu'elle demande » ne titre que les permissions.                                                                      |
| AM12 | 5.1-c        | La note de la chronologie cite l'assertion de durée telle qu'écrite (« L'assertion « responseTime inférieur à 500 ms » porte sur ce total. »).                                                        |

**Lot B - Formes et dispositions**

| ID   | Écran         | Comportement attendu                                                                                                                                                                                                                                             |
| ---- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AM13 | 1.1-a         | Carte d'un projet introuvable : bord plein ; « Ouvrir » grisé dans son menu ⋯ (P10).                                                                                                                                                                             |
| AM14 | 1.1-b bis     | Le sous-menu « Changer la couleur ▸ » s'ouvre à côté du menu sans en cacher les raccourcis.                                                                                                                                                                      |
| AM15 | 1.1-d         | Gérer les workspaces : « + Nouveau workspace » aligné à gauche, pastille de couleur dans le champ « Nom », explication à droite de « Supprimer ce workspace ».                                                                                                   |
| AM16 | 1.1-e, 2.2-f  | « Tous les projets » : recherche, « Importer… » et « Nouveau projet » à droite du titre.                                                                                                                                                                         |
| AM17 | 1.2-a bis     | « + Nouvel environnement » en couleur de lien, comme « + Nouveau workspace ».                                                                                                                                                                                    |
| AM18 | 1.2-e, 4.1-g  | Champs de recherche sans bouton natif d'effacement (WebKitGTK et WebView2) ; la loupe reste à la saisie.                                                                                                                                                         |
| AM19 | 1.3-a         | × sur l'onglet actif, et sur un autre onglet seulement au survol ou au focus (Q1).                                                                                                                                                                               |
| AM20 | 1.3-c         | Un voile assombrit la zone derrière la liste Ctrl+Tab.                                                                                                                                                                                                           |
| AM21 | 1.4-b         | La bulle de taille nomme les zones (« test 500 px · mock 456 px ») ; poignée ⇔ au milieu de la séparation ; rangée d'aide pendant le glisser (« Glisser : redimensionner · Double-clic : parts égales · Alt+Maj+flèches : au clavier · Largeur mini : 280 px »). |
| AM22 | 1.4-h         | Fenêtre de contenu : pastille d'environnement avant « Rattacher » ; « Épinglée » en pastille accent doux avec ●.                                                                                                                                                 |
| AM23 | 2.2-b, 2.2-d  | Libellés des champs des fenêtres en texte secondaire, taille small (composant Field).                                                                                                                                                                            |
| AM24 | 3.1-e         | Version d'un contrat en pastille « v2.1.0 », dans l'onglet et dans la barre latérale.                                                                                                                                                                            |
| AM25 | 3.2-a, 3.2-a2 | En-tête de l'éditeur : Nom, Description et Étiquettes sur une rangée ; « ✓ Enregistré » à droite du fil d'Ariane ; « Délier » sur la ligne de l'adresse à 1280 px.                                                                                               |
| AM26 | 3.4-a         | Libellé « Type » au-dessus de sa liste, comme les autres champs.                                                                                                                                                                                                 |
| AM27 | 4.1-a quater  | « Supprimer » de la barre de sélection en rouge lisible sur la barre sombre (nouveau jeton, Q2).                                                                                                                                                                 |

**Lot C - Réponse, envoyé et run**

| ID   | Écran        | Comportement attendu                                                                                                                                                                                                                                                  |
| ---- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AM28 | 5.1-b        | Ligne d'état : statut en pastille (vert sous 400, rouge sinon), durée, taille, environnement de l'envoi à droite ; le cœur porte l'environnement dans le résultat.                                                                                                    |
| AM29 | 5.1-h        | Onglet Envoyé : « URL résolue », « En-têtes », puis « Variables utilisées » avec leur source (nom de l'environnement, « partagée », « locale », « extraite ») ; valeurs tirées surlignées dans le corps envoyé ; note « Exactement ce qui est parti sur le réseau… ». |
| AM30 | 5.2-e, 5.2-f | Écran du run : fil d'Ariane « … / <dossier> / Exécution en cours » puis « Dernière exécution » ; date de fin ; « Relancer les échecs » en bouton plein ; « Ouvrir le test » sur chaque échec.                                                                         |
| AM31 | 3.3-c        | Confirmation d'un environnement protégé : titre « Envoyer vers <env> ? » avec un cadenas, texte « Cet environnement est protégé. <MÉTHODE> <chemin> … sur <hôte>. », bouton plein « Envoyer vers <env> » ; 🔒 sur la pastille de l'environnement protégé.             |
| AM32 | 4.2-b bis    | Un clic sur une valeur du corps de la réponse propose « Extraire dans une variable » (comme le filtre JSONPath) ; la valeur extraite est marquée « → nom » dans le corps. Should.                                                                                     |

**Lot D - Environnements et authentification** (après le lot J de `plan-qualite-mvp.md`, qui
enregistre les environnements sur place, ER2)

| ID   | Écran | Comportement attendu                                                                                                                                                                                                                          |
| ---- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AM33 | 3.3-a | « Couleur ▾ » à droite du nom ; « Demander une confirmation avant POST, PUT, PATCH et DELETE » ; « Supprimer l'environnement » ; la liste numérote les environnements et montre leur adresse de base.                                         |
| AM34 | 3.3-b | « + Nouvel environnement » ouvre une fenêtre : Nom, URL de base (devient `baseUrl`), « Reprendre les variables de » (secrets non copiés), position dans la chaîne, « Protégé », « Créer ».                                                    |
| AM35 | 3.4-b | Auth héritée : « Type » et « Hériter » ; carte de l'authentification héritée (nature, « réglée sur le projet … », « Ouvrir », jeton obtenu et expiration, « Envoyé dans », valeurs utilisées) ; section « Jeton OAuth 2 » de l'onglet Envoyé. |

**Lot E - Dossiers**

| ID   | Écran     | Comportement attendu                                                                                                                                                                                                                                                                  |
| ---- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AM36 | 4.1-a     | Menu ⋯ d'un dossier : en-tête « Dans le dossier « … » », Nouveau test (Ctrl+N), Nouveau dossier (Ctrl+Maj+N), Dupliquer un test…, Renommer (F2), Déplacer vers…, Exécuter les tests du dossier, Supprimer le dossier ; « + » au survol d'un dossier.                                  |
| AM37 | 4.1-a bis | Un clic sur un dossier ouvre sa vue (onglet `folder`) : fil d'Ariane, « dossier · N tests et M sous-dossiers · T tests au total », « + Test », « + Sous-dossier », « ▶ Exécuter le dossier » ; tableau #, Type, Nom, Contenu, Résultat ; nom saisi sur place (« Entrée pour créer »). |
| AM38 | 4.1-a ter | ⋮⋮ réordonne les lignes de la vue (rang `info.seq` des requêtes et des `folder.yml`), le dépôt sur un sous-dossier y range l'élément, un dossier se glisse aussi ; équivalent au clavier (R12).                                                                                       |
| AM39 | 5.2-d     | ▶ au survol d'un dossier dans l'arborescence lance le run du dossier.                                                                                                                                                                                                                 |

**Lot F - Import**

| ID   | Écran | Comportement attendu                                                                                                                                                                                                                                      |
| ---- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AM40 | 1.1-c | « Importer… » d'« Ajouter des projets » passe par l'import unique (`import::detect`) : collection Bruno, projet Arnod ou OpenCollection, contrat OpenAPI.                                                                                                 |
| AM41 | 2.3-b | La fenêtre « Importer » fait correspondre chaque environnement de la collection à un environnement du projet (ou un nouveau) et dit combien de variables il reçoit ; « N valeurs ressemblent à des secrets » (case cochée : variables locales au coffre). |
| AM42 | 2.3-f | Dans le rapport, « Remplacer par une assertion… » ouvre l'ajout d'une assertion sur la requête et retire le script ; « Accepter sans ce script » le retire du fichier et le marque repris ; « Annuler » dans la notification.                             |

## 5. Hors périmètre

- Les écarts `aligner: maquette` et `à décider` (la maquette se redessine ; les questions se
  tranchent à part).
- Ce que la maquette montre des versions suivantes (R3) : versions de test, campagnes, anomalies,
  mocks, templates, Postman, SoapUI, Windows (NTLM, coffre de Windows).
- PI6 et T24 du lot H de `plan-qualite-mvp.md` (performance de l'arbre), qui restent à ce plan.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust.

### 6.1 Formats sur disque

- Rang d'un dossier : `info.seq` de son `folder.yml` (schéma OpenCollection `FolderInfo`), écrit
  quand l'utilisateur réordonne ; un dossier sans rang suit les rangés, par nom (lot E).
- Script Bruno accepté ou remplacé : retiré de `runtime.scripts` de la requête ; `import.json`
  (poste) marque l'élément repris (lot F).
- Rien d'autre ne change.

### 6.2 Cœur Rust

- `startup::describe` : espaces insécables (lot A).
- `SendOutcome` ou `RequestResult` porte l'environnement de l'envoi (nom, couleur) ; `SentRequest`
  porte la source de chaque variable lue et la place des valeurs tirées dans le corps (lot C).
- `tree::folder_view` (pur sur l'arborescence lue) : enfants d'un dossier dans l'ordre d'exécution,
  nombres et statuts ; `tree_reorder` écrit les rangs (lot E).
- `environment::create_with` : adresse de base, variables reprises (sans les secrets), place dans la
  chaîne (lot D).
- `import` : correspondance des environnements et choix des secrets dans `ImportOptions` ;
  `import_accept_script` et `import_replace_script` (lot F).

### 6.3 Commandes IPC

| Commande                  | Paramètres                                    | Réponse            | Lot |
| ------------------------- | --------------------------------------------- | ------------------ | --- |
| `request_send` (résultat) | inchangés                                     | + environnement    | C   |
| `tree_folder_view`        | `projectId`, `path`                           | `FolderView`       | E   |
| `tree_reorder`            | `projectId`, `folder`, `order: string[]`      | `TreeView`         | E   |
| `environment_create`      | + `baseUrl`, `copyFrom`, `after`, `protected` | `EnvironmentsView` | D   |
| `import_run`              | + `environments: map`, `secretsLocal: bool`   | `ImportReport`     | F   |
| `import_accept_script`    | `projectId`, `item`                           | `ImportReport`     | F   |
| `import_replace_script`   | `projectId`, `item`, `assertion`              | `ImportReport`     | F   |

### 6.4 Frontend

- Contenu `folder` (vue du dossier, `FolderTab.vue`) ouvert par un clic sur un dossier ; le
  chevron seul plie et déplie (lot E).
- `EnvironmentCreateDialog.vue` (lot D) ; carte d'authentification héritée dans `AuthEditor.vue`.
- `lib/format.ts` : accord des nombres (R4).

### 6.5 Design

- Jeton de danger lisible sur la barre sombre de la sélection (`--color-danger-on-inverse`, deux
  thèmes, note dans `scripts/design-system.mjs`) si Q2 le retient.
- Voile des listes flottantes (`--color-scrim`) pour Ctrl+Tab (lot B).

## 7. Impacts sur l'existant

| Fichier                  | Changement                                                                         |
| ------------------------ | ---------------------------------------------------------------------------------- |
| `CLAUDE.md`              | « Travail en cours » pointe vers ce plan ; règles des dossiers (lot E)             |
| `docs/maquette-etat.yml` | écarts retirés lot par lot ; 4.1-a bis, 4.1-a ter, 3.3-b et 5.2-d rendus « livré » |
| `e2e/`                   | preuves de style (R1) ; captures de la vue du dossier, du run en cours             |
| `tests/apercu/`          | scénarios des écrans changés (R6)                                                  |
| Installeur               | inchangé (aucune dépendance)                                                       |

## 8. Plan de tâches pour Claude Code

Un lot = une branche et une pull request vers `Develop`, titrée en Conventional Commits ;
`npm run check` passe avant chaque commit. Les lots s'empilent : chacun part du précédent et sa
pull request dit après laquelle la fusionner. Ordre, avec les lots de `plan-qualite-mvp.md` :

1. Lot A de ce plan (`claude/alignement-a-textes`)
2. Lot I de `plan-qualite-mvp.md` (`claude/qualite-i-lisibilite`) : T26, T27, T28
3. Lot B de ce plan (`claude/alignement-b-formes`)
4. Lot J de `plan-qualite-mvp.md` (`claude/qualite-j-ergonomie`) : T29, T30, T31
5. T25 du lot H de `plan-qualite-mvp.md` (`claude/qualite-h-clavier`) : AC1, AC9, DE5
6. Lot C, puis D, E et F de ce plan

**Lot A - Textes et libellés** (`claude/alignement-a-textes`)

- [x] **T1** AM1 à AM12, un test par texte ; `lib/format.ts` accorde les nombres (R4).

**Lot B - Formes et dispositions** (`claude/alignement-b-formes`)

- [x] **T2** AM13 à AM18 : cartes, menus, gestion des workspaces, champs de recherche.
- [x] **T3** AM19 à AM22 : onglets, Ctrl+Tab, séparations, fenêtre de contenu.
- [x] **T4** AM23 à AM27 : champs, pastille de version, en-tête de l'éditeur, sélection.

**Lot C - Réponse, envoyé et run** (`claude/alignement-c-reponse-run`)

- [x] **T5** AM28 : environnement porté par le résultat, ligne d'état en pastille.
- [x] **T6** AM29 : onglet Envoyé (sources, valeurs tirées surlignées).
- [x] **T7** AM30 : écran du run (fil d'Ariane, date, actions).
- [x] **T8** AM31 : confirmation d'un environnement protégé.
- [x] **T9** AM32 : extraire depuis une valeur du corps (Should).

**Lot D - Environnements et authentification** (`claude/alignement-d-environnements`)

- [ ] **T10** AM33, AM34 : page des environnements, fenêtre de création.
- [ ] **T11** AM35 : carte de l'authentification héritée, jeton OAuth 2 dans l'envoyé.

**Lot E - Dossiers** (`claude/alignement-e-dossiers`)

- [ ] **T12** AM36, AM39 : menu d'un dossier, ▶ au survol.
- [ ] **T13** AM37 : vue du dossier (cœur `tree_folder_view`, onglet `folder`).
- [ ] **T14** AM38 : réordonner et ranger par ⋮⋮ et au clavier (`tree_reorder`).

**Lot F - Import** (`claude/alignement-f-import`)

- [ ] **T15** AM40 : « Importer… » d'« Ajouter des projets » par l'import unique.
- [ ] **T16** AM41 : environnements et secrets à l'import.
- [ ] **T17** AM42 : scripts Bruno remplacés ou acceptés.

## 9. Critères d'acceptation

- [ ] `maquette-etat.yml` n'a plus aucun écart `aligner: code` ; `check-maquette` le compte.
- [ ] Chaque écart retiré a son test de preuve, qui échouait avant le lot.
- [ ] La maquette est régénérée après chaque fusion (`scripts/maquette-sync.py`).
- [ ] `npm run check`, la CI et les e2e passent ; l'installeur reste sous 15 Mo.

## 10. Risques

| Risque                                                     | Impact                      | Parade                                                                               |
| ---------------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------ |
| Lots empilés : une demande de changement sur un lot ancien | Rebase des lots suivants    | Fusionner dans l'ordre ; une correction se fait par un commit, jamais par réécriture |
| La vue du dossier change le clic sur un dossier            | Habitude perdue, e2e cassés | Le chevron garde le pliage ; e2e mis à jour dans le même lot                         |
| Le rang des dossiers réécrit des `folder.yml` de collègues | Conflits Git                | Rang écrit seulement pour les éléments déplacés                                      |
| Les preuves de style passent par l'e2e, lent               | Lots longs à vérifier       | Une exécution e2e avant, une après, par lot                                          |

## 11. Décisions

| ID  | Question                                                                                          | Décision                                                                                          |
| --- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Q1  | × des onglets (1.3-a) : seulement sur l'onglet actif, ou aussi au survol et au focus des autres ? | 2026-11-26 : actif, survol et focus (un onglet se ferme toujours à la souris et au clavier)       |
| Q2  | « Supprimer » sur la barre sombre (4.1-a quater) : nouveau jeton de danger sur fond inverse ?     | 2026-11-26 : oui, jeton dans les deux thèmes, contraste vérifié par `tests/theme.spec.ts`         |
| Q3  | Vue du dossier : l'écran 4.1-a bis (tableau) ou 4.1-a (liste) ?                                   | 2026-11-26 : 4.1-a bis ; sans la colonne de version (V4) : Contenu = nombre de tests d'un dossier |
| Q4  | Clic sur un dossier : ouvre la vue ou plie ?                                                      | 2026-11-26 : ouvre la vue (aperçu, comme une requête) ; le chevron plie                           |
| Q5  | Secrets à l'import (2.3-b), case décochée : valeurs partagées en clair ?                          | 2026-11-26 : oui, c'est le choix de l'utilisateur ; cochée par défaut                             |
| Q6  | « Accepter sans ce script » (2.3-f) retire le script du fichier ou le garde marqué « accepté » ?  | 2026-11-26 : le retire, avec « Annuler » ; le rapport le marque repris                            |
