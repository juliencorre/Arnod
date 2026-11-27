# Plan - Version 4 : enchaîner et réutiliser

**Version** 0.1 · **Date** 2026-11-27 · **Statut** En cours · **Base** version 0.0.4 (V2 livrée)

Plan d'implémentation de la **V4** de la maquette : « Les tests deviennent des scénarios ; les
scripts gardés à l'import Bruno s'exécutent. » Il part de la version 0.0.4 (MVP, V1 et V2 livrés ;
V3 non commencée) et respecte les règles de [`CLAUDE.md`](../CLAUDE.md) et le design system
(`docs/design-system/`). L'import Postman (2.3-d), cinquième fonction de la V4, est hors de ce plan
(demande du 2026-11-27).

Source : maquette, page 0.1-a (section `feuille_de_route` de [`maquette-etat.yml`](maquette-etat.yml))
et pages 4.3 « Tests en schéma », 4.7 « Jeux de données », 4.8 « Templates », 3.2 « Endpoint »
(3.2-d) et 1.2 « Barre latérale du projet » (1.2-d bis). Les écrans sont cités par leur repère (voir
[`plan-suivi-maquette.md`](plan-suivi-maquette.md)).

## 1. Objectif

À la fin de ce plan, Arnod :

- montre un test en liste (étapes numérotées, chacune dit où l'on va ensuite) ou en schéma (palette,
  insertion sur un lien, glisser-déposer, zoom) ; un test enchaîne des étapes de natures variées :
  requête, condition (« si … aller à »), boucle (répéter, ou pour chaque élément d'une liste),
  attente (jusqu'à ce qu'une condition soit vraie), variable, script, test existant, template ;
- après exécution, montre le chemin suivi, les valeurs passées d'une étape à l'autre et l'étape qui
  a arrêté le test ; « Si une étape échoue » : arrêter le test ou continuer ;
- exécute quelques lignes de JavaScript isolé (`vars`, `response`, `assert()`), sans disque ni
  réseau, 2 s au plus ; exécute les scripts Bruno gardés à l'import (avant la requête, après la
  réponse, tests) ;
- joue une requête une fois par cas d'un jeu de données (tableau saisi dans Arnod ou fichier CSV du
  projet) ; le test compte une fois, avec son détail par cas, dans l'éditeur et dans un run ;
- prépare et nettoie les données d'un dossier : « Avant le dossier », « Avant chaque test »,
  « Après chaque test », « Après le dossier » (même en échec) ;
- donne des propriétés à un dossier (héritées par ses sous-dossiers, avant l'environnement) ;
- réutilise un test, un dossier ou une arborescence comme template, avec ses paramètres : créé
  depuis un dossier ou transféré depuis un endpoint, utilisé dans un dossier (on ne règle que les
  valeurs), appelé comme étape ;
- range les sections des versions suivantes sous « Plus » dans la barre latérale ; chacune
  s'affiche seule à sa première utilisation.

## 2. Maquettes

| Repère         | Écran                                                                      | Lot | Cas       |
| -------------- | -------------------------------------------------------------------------- | --- | --------- |
| `4.3-a`        | Liste : étapes numérotées, la condition renvoie vers une étape             | A   | SC1 à SC3 |
| `4.3-e`        | « + Étape ▾ » : choisir sa nature                                          | A   | SC4       |
| `4.3-c`        | Étape sélectionnée : entrées, extractions, assertions, si elle échoue      | A   | SC5       |
| `4.3-d`        | Après exécution : chemin suivi, valeurs sur les liens, arrêt               | B   | SC6, SC7  |
| `4.3-b`        | Schéma : palette, nœuds et liens                                           | B   | SC8       |
| `4.3-b bis`    | Insérer entre deux étapes : « + » sur un lien, sélecteur au clavier        | B   | SC9       |
| `4.3-b ter`    | Glisser depuis la palette sur un lien                                      | B   | SC9       |
| `4.3-b quater` | Étape insérée « à configurer », liens reconnectés, « Annuler l'insertion » | B   | SC10      |
| `4.3-g`        | Étape Script : JavaScript isolé, `vars`, `response`, `assert()`            | C   | JS1 à JS3 |
| `4.7-a`        | Onglet « Données » : tableau, cas sélectionné, requête du cas              | D   | JD1 à JD3 |
| `4.7-a bis`    | Depuis un fichier CSV du projet, relu à chaque envoi                       | D   | JD4       |
| `4.7-b`        | Après l'envoi : résultat par cas, réponse du cas                           | D   | JD5, JD6  |
| `4.7-b bis`    | Dans l'exécution d'un dossier : le test compte une fois, détail par cas    | D   | JD7       |
| `4.7-c`        | Avant / Après sur un dossier                                               | E   | AV1 à AV3 |
| `4.7-c bis`    | Si « Avant » échoue : tests non joués, « Après » nettoie quand même        | E   | AV4       |
| `4.8-a`        | Propriétés d'un dossier : définies ici, héritées, de l'environnement       | E   | PR1, PR2  |
| `4.8-b`        | Créer un template depuis un dossier : paramètres détectés                  | F   | TP1, TP2  |
| `4.8-c`        | Template : paramètres, contenu, utilisations                               | F   | TP3       |
| `4.8-d`        | Utilisation dans un dossier : on ne règle que les valeurs                  | F   | TP4, TP5  |
| `3.2-d`        | Transférer vers les templates : valeurs saisies devenues paramètres        | F   | TP6       |
| `1.2-d bis`    | « Plus » : les autres sections ; une section s'affiche seule               | G   | PL1, PL2  |

Hors V4 dans ces pages : `4.3-f` (étape SOAP, V5), `4.3-h` et `4.3-i` (étapes IHM, V6) ; dans la
palette et le menu des natures, SOAP, IHM et « Manuelle » viennent avec leurs versions ; « Créer une
anomalie » (4.7-b) vient avec la V7 ; les sections Tableau de bord, Produits et versions,
Exigences, Mocks, Campagnes et Anomalies de « Plus » viennent avec leurs versions.

## 3. Modèle et règles

**Schéma d'un test** : la liste ordonnée de ses étapes ; chaque étape a un identifiant stable, une
nature et sa suite (« puis → étape », « Fin ✓ » ou « Fin en échec » ; sans suite écrite, l'étape
suivante de la liste, ou la fin). Natures : **requête** (un fichier de requête du dossier du test),
**condition** (une cible, un opérateur et une valeur, comme une assertion ; « si oui → », « sinon
→ »), **boucle** (répète les étapes qui la suivent jusqu'à une étape donnée : N fois, ou pour chaque
élément d'une liste, rangé dans une variable), **attente** (renvoie une requête, ou relit les
variables, jusqu'à ce qu'une condition soit vraie : intervalle et nombre d'essais), **variable**
(un nom et une valeur, `{{…}}` résolues), **script** (JavaScript isolé), **test existant** (un test
du référentiel, joué tel quel avec les variables du test), **template** (un template et les valeurs
de ses paramètres). Un test sans schéma écrit est la liste de ses requêtes, dans l'ordre du dossier.

**Jeu de données** : un tableau (une colonne par variable, une ligne par cas) lié à une requête ;
il est rangé dans un fichier CSV du projet. **Avant / Après** : étapes d'un dossier jouées une
fois au début (avant le dossier), avant et après chacun de ses tests, une fois à la fin (après le
dossier). **Propriétés** d'un dossier : ses variables (`request.variables` de son `folder.yml`).
**Template** : un test, un dossier ou une arborescence rangé sous `templates/`, avec des
**paramètres** (nom, valeur par défaut, description) ; une **utilisation** est un dossier du
référentiel qui joue le contenu du template avec ses valeurs.

Les règles vivent dans le cœur Rust, modules purs testés règle par règle.

| ID  | Règle                                                                                                                                                                                                                                                                                                                                                                         |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Le schéma d'un test vit dans `extensions.arnod.tests` de `opencollection.yml`, sous le chemin du test ; un test qui se joue comme la liste de ses requêtes (sans renvoi, sans autre nature, arrêt sur échec) n'y est pas écrit. Les requêtes gardent leur `info.seq` dans l'ordre du schéma.                                                                                  |
| R2  | Le schéma relu est réconcilié avec le dossier : une étape requête dont le fichier a disparu est retirée (ses renvois passent à l'étape suivante) ; une requête du dossier absente du schéma s'ajoute à la fin ; un renvoi vers une étape inconnue vaut « étape suivante ».                                                                                                    |
| R3  | Les renvois suivent l'étape, pas son numéro : réordonner, insérer ou retirer une étape garde les renvois des autres ; insérer sur un lien reconnecte ce lien (l'étape d'avant renvoie vers la nouvelle, la nouvelle vers celle d'après).                                                                                                                                      |
| R4  | Un renommage, un déplacement ou une copie faits dans Arnod font suivre le schéma, les Avant / Après et les renvois « test existant » ; après une suppression, ils restent (l'annulation remet le test entier) et ne valent pour aucun dossier.                                                                                                                                |
| R5  | Exécution : une étape à la fois, dans l'ordre des renvois ; une étape en échec arrête le test (« Arrêter le test »), sauf « Continuer » (réglage du test, ou de l'étape) : le test reste alors en échec. « Fin en échec » termine le test en échec ; « Fin ✓ » le termine sans échec ajouté.                                                                                  |
| R6  | Une condition se lit sur la réponse de la dernière requête jouée (`status`, `body.…`, `headers.…`, `responseTime`) ou sur une variable (`vars.nom`), avec les opérateurs des assertions ; elle ne fait jamais échouer le test.                                                                                                                                                |
| R7  | Une boucle a au plus 1 000 tours ; une attente au plus 100 essais et 60 s entre deux essais ; un test joué en « test existant » ou en template n'est pas rappelé par lui-même (profondeur 8 au plus) ; un parcours qui dépasse 10 000 étapes jouées s'arrête en erreur.                                                                                                       |
| R8  | Les étapes non empruntées sont « non exécutées » ; chaque étape jouée garde son résultat, son nombre de passages et les valeurs qu'elle a extraites, masquées comme dans l'onglet « Envoyé ».                                                                                                                                                                                 |
| R9  | Script : JavaScript (QuickJS) sans accès au disque, au réseau ni à l'horloge du système de fichiers ; 2 s et 32 Mo au plus ; `vars` (lecture, `vars.set`), `response` (statut, en-têtes, corps lu en JSON s'il en est), `assert(condition, message)` ; une exception ou un `assert` faux fait échouer l'étape.                                                                |
| R10 | Scripts Bruno (`runtime.scripts` d'une requête, `request.scripts` des dossiers et de la racine) : exécutés à l'envoi (avant la requête : racine, dossiers, requête ; après la réponse et tests : requête, dossiers, racine), avec `bru`, `req`, `res`, `test` et `expect` ; un test du script donne une assertion.                                                            |
| R11 | Jeu de données : un fichier CSV du projet (première ligne : noms des variables), lié à la requête par l'étiquette `data:<fichier>` (tableau modifiable dans Arnod) ou `data-file:<fichier>` (fichier relu à chaque envoi, jamais écrit par Arnod). Le tableau d'Arnod est rangé dans `données/`.                                                                              |
| R12 | Une requête jouée seule (envoi, run) avec un jeu de données est jouée une fois par cas ; les valeurs du cas priment sur toutes les autres variables ; elle réussit si tous ses cas réussissent et compte pour une ligne dans un run. Une étape d'un test ne lit pas de jeu de données : une boucle répète.                                                                    |
| R13 | Avant / Après d'un dossier (`extensions.arnod.folders`) : « Avant le dossier » une fois avant ses tests directs ; s'il échoue, ces tests ne sont pas joués et sont en échec (« Avant en échec ») ; ses sous-dossiers se jouent avec les leurs ; « Avant chaque test » et « Après chaque test » encadrent chacun de ses tests directs ; « Après » se joue même après un échec. |
| R14 | Les requêtes d'un Avant / Après et celle d'une attente appartiennent à leur étape : rangées dans le dossier, absentes de l'arborescence, jamais jouées seules.                                                                                                                                                                                                                |
| R15 | Variables à l'envoi, de la plus forte à la plus faible : valeurs du cas, variables d'exécution (extraites, scripts, étapes variable), valeurs du template, propriétés des dossiers (du plus proche au plus lointain), environnement, variables de la collection.                                                                                                              |
| R16 | Un template est un dossier de `templates/` marqué par l'étiquette `arnod:template` ; ses paramètres sont les `request.variables` de son `folder.yml` (valeur par défaut, description) ; un paramètre sans valeur par défaut est obligatoire. Sa nature (test, dossier, arborescence) se déduit de son contenu.                                                                |
| R17 | Une utilisation est un dossier du référentiel marqué `template:<chemin du template>` ; ses valeurs sont ses `request.variables`. Une valeur vide reprend la propriété des dossiers parents, puis la valeur par défaut ; un paramètre obligatoire sans valeur arrête l'exécution en le nommant.                                                                                |
| R18 | Créer un template depuis un dossier copie son contenu sous `templates/` ; les variables lues par ses requêtes et données par ses propriétés (ou héritées) sont proposées comme paramètres, celles de l'environnement non cochées. « Remplacer le dossier » en fait une utilisation, au même résultat.                                                                         |
| R19 | Transférer un endpoint vers les templates : chaque valeur écrite du corps JSON et des paramètres d'adresse est proposée ; une valeur cochée devient `{{nom}}` et un paramètre dont elle est la valeur par défaut ; la requête quitte le contrat.                                                                                                                              |
| R20 | `templates/` et `données/` sont réservés à la racine du projet comme `environments/` et `contracts/` : absents de l'arborescence, refusés comme nom de dossier racine. Les étiquettes `arnod:`, `template:`, `data:` et `data-file:` ne sont jamais montrées ni héritées.                                                                                                     |
| R21 | « Plus » liste les sections disponibles et masquées du projet ; « Afficher » la garde affichée (poste, `local.json`) ; une section s'affiche seule dès qu'elle a un élément.                                                                                                                                                                                                  |

## 4. Cas d'usage

Priorités : **Must** (indispensable au lot), **Should** (attendu), **Could** (si le temps le
permet).

### Lot A - Test en liste (4.3-a, 4.3-e, 4.3-c)

| ID  | Cas d'usage           | Écrans  | Comportement attendu                                                                                                                      | Priorité |
| --- | --------------------- | ------- | ----------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| SC1 | Voir le test en liste | `4.3-a` | « N étapes · M conditions », Liste / Schéma ; colonnes #, nature, étape, données (« extrait », « utilise »), enchaînement (« puis → 2 »). | Must     |
| SC2 | Réordonner            | `4.3-a` | ⋮⋮ se glisse ou se déplace par ↑ ↓ (R3) ; enregistrement automatique.                                                                     | Must     |
| SC3 | Renvoi                | `4.3-a` | « puis → » se choisit : une étape, Fin ✓, Fin en échec ; une condition a « si oui » et « sinon ».                                         | Must     |
| SC4 | Ajouter une étape     | `4.3-e` | « + Étape ▾ » : recherche, natures groupées (Appels, Logique, Données et code, Réutiliser) ; raccourcis « + Requête », « + Condition »…   | Must     |
| SC5 | Configurer une étape  | `4.3-c` | Panneau : nom, entrées (« ← étape · source »), extractions, assertions, « Si cette étape échoue » ; formulaire de chaque nature.          | Must     |

### Lot B - Exécution et schéma (4.3-d, 4.3-b à b quater)

| ID   | Cas d'usage          | Écrans          | Comportement attendu                                                                                                                           | Priorité |
| ---- | -------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| SC6  | Jouer le test        | `4.3-d`         | ▶ suit les renvois (R5 à R7) ; chaque étape : ✓ 52 ms, ✓ oui, ✗ 9/12, non exécuté.                                                             | Must     |
| SC7  | Lire le chemin suivi | `4.3-d`         | Chemin en vert, branche non empruntée en pointillé, valeurs sur les liens (`customerId = cus_31`), « ✗ Arrêtée à … ».                          | Must     |
| SC8  | Voir le schéma       | `4.3-b`         | Départ, nœuds par nature, liens « oui » / « non », Fin ✓, Fin en échec ; − 100 % + Ajuster.                                                    | Must     |
| SC9  | Insérer              | `4.3-b bis/ter` | « + » d'un lien (au survol et au focus) : sélecteur au clavier ; glisser une nature de la palette sur un lien, ou dans le vide (fin de liste). | Must     |
| SC10 | Étape insérée        | `4.3-b quater`  | Étape « à configurer », panneau ouvert, données disponibles à ce point ; « Annuler l'insertion » (Ctrl+Z), « Supprimer l'étape ».              | Should   |

### Lot C - Scripts (4.3-g)

| ID  | Cas d'usage        | Écrans  | Comportement attendu                                                                                           | Priorité |
| --- | ------------------ | ------- | -------------------------------------------------------------------------------------------------------------- | -------- |
| JS1 | Étape script       | `4.3-g` | Éditeur de code, « À disposition » : `vars`, `response`, `assert()` ; « Exécution isolée… 2 s maximum ».       | Must     |
| JS2 | Résultat du script | `4.3-d` | ✓ 3 ms, ou ✗ avec le message de l'`assert` ou de l'exception ; `vars.set` passe aux étapes suivantes.          | Must     |
| JS3 | Scripts Bruno      | -       | Un script gardé à l'import s'exécute (R10) ; ses tests deviennent des assertions ; le rapport d'import le dit. | Must     |

### Lot D - Jeux de données (4.7-a à b bis)

| ID  | Cas d'usage              | Écrans      | Comportement attendu                                                                                                             | Priorité |
| --- | ------------------------ | ----------- | -------------------------------------------------------------------------------------------------------------------------------- | -------- |
| JD1 | Saisir un tableau        | `4.7-a`     | Onglet « Données N » : colonnes (variables), lignes (cas), « + Cas », « + Colonne » ; enregistré seul (R11).                     | Must     |
| JD2 | Coller depuis un tableur | `4.7-a`     | Coller des cellules (tabulations) ajoute des lignes.                                                                             | Should   |
| JD3 | Le cas sélectionné       | `4.7-a`     | « Cas 4 sur 12 ‹ › », requête du cas (adresse résolue, masquée), valeurs du cas, assertions.                                     | Must     |
| JD4 | Depuis un fichier CSV    | `4.7-a bis` | « Fichier CSV… » : choisi sur le poste, rangé dans `données/`, en lecture seule, relu à chaque envoi ; « Ouvrir ».               | Must     |
| JD5 | Envoyer les cas          | `4.7-b`     | « Envoyer » joue tous les cas ; ✓ / ✗ par ligne ; « 11 ✓ 1 ✗ » ; le menu d'une ligne envoie ce cas seul.                         | Must     |
| JD6 | Réponse d'un cas         | `4.7-b`     | La réponse du cas choisi ; « ✕ Cas 4 : status attendu 200, reçu 400 » ; « Ignorer ce cas pour ce résultat », « Retirer le cas ». | Should   |
| JD7 | Dans un run              | `4.7-b bis` | Une ligne « 12 cas » ; son premier cas en échec, « 11 autres cas réussis · Voir les 12 cas ».                                    | Must     |

### Lot E - Avant / Après et propriétés (4.7-c, 4.7-c bis, 4.8-a)

| ID  | Cas d'usage      | Écrans      | Comportement attendu                                                                                                                          | Priorité |
| --- | ---------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| AV1 | Ajouter un Avant | `4.7-c`     | Menu ⋯ du dossier et vue du dossier : « Avant le dossier », « Avant chaque test », « Après chaque test », « Après le dossier » ; « + Étape ». | Must     |
| AV2 | Voir l'ordre     | `4.7-c`     | Vue du dossier : Avant en tête, Après à la fin, l'ordre de la liste est l'ordre d'exécution.                                                  | Must     |
| AV3 | Jouer            | `4.7-c`     | Run du dossier (R13) ; résultat de chaque ligne dans la vue.                                                                                  | Must     |
| AV4 | Avant en échec   | `4.7-c bis` | Tests directs « ✗ Échec · Avant en échec », « — Non joué » ; Après « ✓ Réussi ».                                                              | Must     |
| PR1 | Propriétés       | `4.8-a`     | Onglet « Propriétés N » : nom, valeur, origine (défini ici, hérité de …, environnement …) ; Retirer, Redéfinir, + Propriété.                  | Must     |
| PR2 | Résolution       | `4.8-a`     | `{{devise}}` d'une requête du dossier reprend la propriété (R15).                                                                             | Must     |

### Lot F - Templates (4.8-b à d, 3.2-d)

| ID  | Cas d'usage                    | Écrans  | Comportement attendu                                                                                                        | Priorité |
| --- | ------------------------------ | ------- | --------------------------------------------------------------------------------------------------------------------------- | -------- |
| TP1 | Créer un template d'un dossier | `4.8-b` | « ◇ Créer un template de ce dossier » : nom, « Ranger dans », paramètres détectés (R18).                                    | Must     |
| TP2 | Remplacer par une utilisation  | `4.8-b` | « Remplacer le dossier par une utilisation du template » ou « Garder le dossier tel quel ».                                 | Must     |
| TP3 | Voir un template               | `4.8-c` | Paramètres (défaut, obligatoire, description, + Paramètre), contenu, utilisations (valeurs, dernière exécution).            | Must     |
| TP4 | Utiliser dans un dossier       | `4.8-d` | « ◇ Utiliser dans un dossier… » ; la vue de l'utilisation règle les valeurs (origine de chacune), contenu en lecture seule. | Must     |
| TP5 | Étape template                 | `4.3-a` | Une étape template d'un test joue le template avec ses valeurs ; ✗ 9/12.                                                    | Must     |
| TP6 | Transférer un endpoint         | `3.2-d` | « Transférer vers les templates » : valeurs saisies à cocher (R19), nom, « Ranger dans ».                                   | Must     |

### Lot G - Plus (1.2-d bis)

| ID  | Cas d'usage           | Écrans      | Comportement attendu                                                                 | Priorité |
| --- | --------------------- | ----------- | ------------------------------------------------------------------------------------ | -------- |
| PL1 | Voir les autres       | `1.2-d bis` | « Plus N » sous les sections ; chaque section avec sa raison d'être et « Afficher ». | Must     |
| PL2 | Affichage automatique | `1.2-d bis` | « Templates » s'affiche dès que le projet a un template (R21).                       | Must     |

## 5. Hors périmètre

- Import Postman (2.3-d) : retiré de ce plan à la demande ; « Postman » n'est pas cité.
- Étapes SOAP (4.3-f, V5), IHM (4.3-h, 4.3-i, V6), manuelles (V7) ; « Parcourir le catalogue… »
  ouvre le catalogue existant (lecture seule).
- Fichiers Excel : un jeu de données se lit en CSV (Excel l'enregistre ainsi) ; lire `.xlsx`
  demanderait une bibliothèque de plus (Q3).
- « Créer une anomalie » (V7), campagnes (V7), versions métier d'un test (3.2-c, V7).
- « Relancer depuis cette étape » (4.3-d) et l'enregistreur : à décider.

## 6. Architecture

La chaîne reste : composant Vue → store Pinia → `src/lib/ipc.ts` → commande Rust. La ligne de
commande appelle le même cœur : schémas, scripts, jeux de données, Avant / Après et templates
s'y jouent comme dans l'application.

### 6.1 Formats sur disque

- **`opencollection.yml`** (partagé) : `extensions.arnod.tests` et `extensions.arnod.folders`,
  sous le chemin du test ou du dossier (Q1). Le reste d'`extensions.arnod` est gardé à chaque
  écriture.

  ```yaml
  extensions:
    arnod:
      version: 1
      id: 6c1f4e0a-…
      tests:
        commandes/v2/parcours-achat:
          onFailure: stop
          steps:
            - { id: s1, kind: request, file: Créer un client.yml }
            - id: s3
              kind: condition
              name: Commande acceptée ?
              check: { target: status, operator: equals, value: 201 }
              next: s4
              otherwise: s6
            - { id: s6, kind: request, file: Refus attendu.yml, next: fail }
      folders:
        commandes/v2:
          beforeAll: [{ id: h1, kind: request, file: Créer un client de test.yml }]
          afterAll: [{ id: h2, kind: request, file: Supprimer le client de test.yml }]
  ```

- **Requête** : étiquette `data:données/Lister les commandes.csv` ou `data-file:données/filtres.csv`
  (jeu de données, R11). Rien d'autre ne change ; les scripts restent où l'import les a gardés.
- **Dossier** : `request.variables` de `folder.yml` (propriétés, valeurs d'une utilisation,
  paramètres d'un template) ; étiquettes `arnod:template` (template) et `template:<chemin>`
  (utilisation).
- **`données/`** : CSV en UTF-8, séparateur `,` (ou `;` reconnu à la lecture), guillemets RFC 4180.
- **`templates/`** : dossiers et requêtes OpenCollection ordinaires.
- Compatibilité : un projet écrit par la V4 s'ouvre dans une version précédente ; ses tests s'y
  jouent comme la liste de leurs requêtes (ce qu'une version précédente ignore reste écrit).

### 6.2 Cœur Rust

- **`flow/`** : modèle (`Flow`, `Step`, `Action`, `Check`, `OnFailure`), `store.rs` (lecture,
  écriture, suivi des chemins, R1, R4), `reconcile.rs` (pur, R2, R3), `check.rs` (pur : condition
  sur une réponse ou des variables, R6), `engine.rs` (parcours, R5, R7, R8), `hooks.rs`.
- **`script/`** : QuickJS par `rquickjs` (moteur compilé avec Arnod, Q2), étape script (R9),
  scripts Bruno (`bruno.js` : `bru`, `req`, `res`, `test`, `expect`, R10).
- **`dataset/`** : CSV écrit à la main (pur), lien `data:`, cas (R11, R12).
- **`template/`** : `templates/`, nature déduite, paramètres, création depuis un dossier (R18),
  utilisations (R17), transfert d'un endpoint (R19).
- **`request/inherit.rs`** : propriétés des dossiers et de la collection, avec leur source (R15).
- **`workbench`** : un run se construit par unités (test et son schéma, requête seule et ses cas,
  utilisation d'un template) groupées par dossier à Avant / Après ; `RunItem` gagne la nature,
  l'étape, la branche prise, les passages, les cas, la phase d'Avant / Après.

### 6.3 Commandes IPC

| Commande                | Paramètres                                                   | Réponse               | Erreurs (`kind`)               |
| ----------------------- | ------------------------------------------------------------ | --------------------- | ------------------------------ |
| `flow_save`             | `project`, `path`, `flow`                                    | `TestView`            | `invalidRequest`, `notFound`   |
| `flow_add_step`         | `project`, `path`, `kind`, `after?`, `link?`                 | `FlowAdded`           | `invalidRequest`               |
| `hooks_read`            | `project`, `path`                                            | `FolderHooks`         | -                              |
| `hooks_save`            | `project`, `path`, `hooks`                                   | `FolderHooks`         | `invalidRequest`               |
| `hooks_add_request`     | `project`, `path`, `phase`                                   | `FolderHooks`         | `invalidRequest`               |
| `folder_properties`     | `project`, `path`, `environment`                             | `FolderProperty[]`    | `notFound`                     |
| `folder_set_properties` | `project`, `path`, `properties`                              | `FolderProperty[]`    | `invalidRequest`               |
| `data_read`             | `project`, `path`                                            | `DataSet?`            | `invalidRequest`               |
| `data_save`             | `project`, `path`, `columns`, `rows`                         | `DataSet`             | `invalidRequest`               |
| `data_import`           | `project`, `path`                                            | `DataSet?`            | `invalidRequest`, `cancelled`  |
| `data_remove`           | `project`, `path`                                            | -                     | -                              |
| `data_parse`            | `text`                                                       | `DataTable`           | -                              |
| `data_case`             | `project`, `path`, `request`, `env`, `case`                  | `CasePreview`         | `invalidRequest`               |
| `request_send_cases`    | `project`, `path`, `request`, `env`, `cases?`                | `CasesOutcome`        | `protectedEnvironment`, réseau |
| `data_open`             | `project`, `path`                                            | -                     | `notFound`                     |
| `template_list`         | `project`                                                    | `TemplateSummary[]`   | -                              |
| `template_read`         | `project`, `path`                                            | `TemplateView`        | `notFound`                     |
| `template_save_params`  | `project`, `path`, `params`                                  | `TemplateView`        | `invalidRequest`               |
| `template_candidates`   | `project`, `folder`, `environment`                           | `TemplateCandidate[]` | `notFound`                     |
| `template_create`       | `project`, `folder`, `name`, `category`, `params`, `replace` | `TemplateView`        | `invalidRequest`, `nameTaken`  |
| `template_use`          | `project`, `template`, `folder`, `name`                      | `string`              | `nameTaken`                    |
| `template_instance`     | `project`, `path`, `environment`                             | `InstanceView`        | `notFound`                     |
| `template_set_values`   | `project`, `path`, `values`                                  | `InstanceView`        | `invalidRequest`               |
| `template_literals`     | `project`, `path`                                            | `Literal[]`           | `notFound`                     |
| `template_from_request` | `project`, `path`, `name`, `category`, `params`              | `TemplateView`        | `invalidRequest`, `nameTaken`  |
| `sections_show`         | `project`, `section`, `shown`                                | `string[]`            | -                              |

`test_view` (existant) rend aussi le schéma réconcilié (`TestView.flow`, `TestView.info`) ;
`tree_load` rend les templates du projet (`TreeView.templates`) et les sections affichées.

### 6.4 Frontend

- `TestTab.vue` : en-tête (Liste / Schéma), `FlowList.vue` (4.3-a), `FlowDiagram.vue` (4.3-b, SVG,
  disposition par `lib/flowLayout.ts`, pur et testé), `FlowPalette.vue`, `StepPicker.vue`
  (4.3-e, 4.3-b bis : motif combobox), `StepPanel.vue` (4.3-c, formulaire de chaque nature,
  `ScriptEditor`), store `flows` (brouillon du schéma, enregistrement automatique, annulation).
- `DataEditor.vue` (onglet « Données » de `RequestEditor`), `CasesBar.vue` (cas du résultat),
  store `datasets`.
- `FolderTab.vue` : Avant / Après (`FolderHooks.vue`), onglet « Propriétés »
  (`FolderProperties.vue`), « Créer un template de ce dossier » (`CreateTemplateDialog.vue`).
- `TemplatesSection.vue` (barre latérale), `TemplateTab.vue` (contenu `template`),
  `InstanceTab.vue` (contenu `instance`), `UseTemplateDialog.vue`, `TransferTemplateDialog.vue`
  (3.2-d, depuis `TrialTransfer`), `MoreSections.vue` (« Plus »).

### 6.5 Design

- Couleurs : les natures d'étape reprennent les jetons des familles (`--color-family-*`) ; le
  chemin suivi prend `--color-success`, une branche non empruntée `--color-subtle` en pointillé ;
  aucun jeton nouveau prévu (sinon défini pour les deux thèmes avec sa note dans
  `scripts/design-system.mjs`).
- Clavier : toute action glissée (⋮⋮, palette) a son équivalent (↑ ↓, « + » d'un lien, menu
  « + Étape ») ; le schéma se parcourt à la tabulation (nœuds puis « + » des liens).

## 7. Impacts sur l'existant

| Fichier                        | Changement                                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------------------------- |
| `CLAUDE.md`                    | Sections « Tests en schéma », « Scripts », « Jeux de données », « Templates » ; dossiers réservés |
| `src-tauri/core/Cargo.toml`    | `rquickjs` (moteur JavaScript compilé avec Arnod, Q2)                                             |
| `opencollection/collection.rs` | `set_meta` garde les clés d'`extensions.arnod` qu'il ne gère pas                                  |
| `tree/`                        | `templates/` et `données/` réservés ; requêtes d'une étape absentes de l'arborescence (R14)       |
| `workbench.rs`                 | Run par unités et schémas ; `RunItem` étendu                                                      |
| `bruno/`                       | Le rapport d'import dit que les scripts s'exécutent                                               |
| `docs/maquette-etat.yml`       | 4.3-a à e, g ; 4.7 ; 4.8 ; 3.2-d ; 1.2-d bis ; 4.2-a (« Si une étape échoue »)                    |

## 8. Plan de tâches pour Claude Code

Les lots sont livrés ensemble, sur la branche de la session, en une pull request (demande du
2026-11-27 : « implémenter directement »). `npm run check` passe avant chaque commit.

**Lot A - Test en liste**

- [ ] **T1** `flow/` : modèle, réconciliation (R2, R3), stockage (R1), suivi (R4) ; tests règle par règle.
- [ ] **T2** `test_view` avec le schéma, `flow_save`, `flow_add_step` ; types IPC, faux cœur.
- [ ] **T3** `TestTab` en liste, `StepPicker`, `StepPanel` ; Vitest au clavier.

**Lot B - Exécution et schéma**

- [ ] **T4** `flow/engine.rs` et `workbench` : parcours (R5 à R8) ; tests contre un serveur simulé.
- [ ] **T5** `FlowDiagram`, palette, insertion, résultats sur le schéma ; `lib/flowLayout.ts`.

**Lot C - Scripts**

- [ ] **T6** `script/` : étape script (R9), scripts Bruno (R10) ; tests.
- [ ] **T7** Éditeur de script dans `StepPanel` ; rapport d'import.

**Lot D - Jeux de données**

- [ ] **T8** `dataset/` (R11, R12), envoi par cas, run ; tests.
- [ ] **T9** `DataEditor`, cas du résultat, run ; Vitest.

**Lot E - Avant / Après et propriétés**

- [ ] **T10** Avant / Après (R13, R14), propriétés et résolution (R15) ; tests.
- [ ] **T11** `FolderTab` : Avant / Après, Propriétés ; Vitest.

**Lot F - Templates**

- [ ] **T12** `template/` (R16 à R19), étape template, run d'une utilisation ; tests.
- [ ] **T13** Section Templates, `TemplateTab`, `InstanceTab`, dialogues, 3.2-d ; Vitest.

**Lot G - Plus**

- [ ] **T14** « Plus » (R21) ; Vitest ; maquette-etat, CLAUDE.md, e2e.

## 9. Critères d'acceptation

- [ ] `cargo test` : règles R1 à R21, dont un test joué contre un serveur simulé (condition,
      boucle, attente, script, test existant, template, jeu de données, Avant / Après).
- [ ] Vitest : liste, schéma et insertion au clavier, panneau d'étape, données, Avant / Après,
      propriétés, templates, « Plus », sur le faux cœur.
- [ ] `arnod run` joue un test en schéma et une requête à jeu de données.
- [ ] `npm run check` passe ; installeur < 15 Mo (QuickJS compte pour environ 1 Mo).

## 10. Risques

| Risque                                      | Impact                        | Parade                                                                 |
| ------------------------------------------- | ----------------------------- | ---------------------------------------------------------------------- |
| Moteur JavaScript lourd ou non portable     | Installeur, ligne de commande | QuickJS (C, ~1 Mo), compilé comme libgit2 ; lié statiquement sous musl |
| Script sans fin ou gourmand                 | Run bloqué                    | Interruption à 2 s, mémoire à 32 Mo (R9)                               |
| Schéma désaccordé du dossier (Git, éditeur) | Étapes perdues                | Réconciliation à la lecture (R2), jamais d'écriture sans action        |
| Boucle infinie de renvois                   | Run sans fin                  | Plafonds (R7)                                                          |
| `opencollection.yml` plus souvent modifié   | Conflits de synchronisation   | Schéma écrit seulement s'il n'est pas linéaire (R1)                    |

## 11. Décisions

| ID  | Question                                             | Décision                                                                                                                                                                                                                                                     |
| --- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Q1  | Où ranger le schéma d'un test et les Avant / Après ? | Retenu le 2026-11-27 : `extensions.arnod` de la racine (seul point d'extension du schéma, Q1 du plan OpenCollection), sous le chemin ; Arnod le fait suivre (R4). Un fichier propre à Arnod dans le dossier du test reste exclu (R6 du plan OpenCollection). |
| Q2  | Quel moteur JavaScript ?                             | Retenu : QuickJS par `rquickjs` (petit, interruptible, sans module système) ; écartés : V8 (taille), Boa (taille, lenteur).                                                                                                                                  |
| Q3  | Excel ?                                              | Retenu : CSV seulement (« Fichier CSV… ») ; un `.xlsx` demanderait `calamine` et ses dépendances. À revoir sur demande.                                                                                                                                      |
| Q4  | Jeu de données d'une étape d'un test ?               | Retenu : non (R12) ; la boucle « pour chaque élément » répète des étapes. Un jeu de données sur un test entier : plus tard.                                                                                                                                  |
| Q5  | Corps d'une boucle                                   | Retenu : les étapes qui suivent la boucle jusqu'à l'étape « jusqu'à » choisie ; un renvoi qui sort du corps termine la boucle.                                                                                                                               |
| Q6  | Paramètre obligatoire et valeur par défaut           | Retenu : un paramètre sans valeur par défaut est obligatoire (R16) ; rien d'autre n'est écrit (pas de champ inventé).                                                                                                                                        |
| Q7  | « Plus » et les sections des versions suivantes      | Retenu : « Plus » ne liste que les sections disponibles dans cette version (Templates) ; les autres s'y ajoutent avec leur version.                                                                                                                          |
