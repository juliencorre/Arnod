# Plan - Suivi de la maquette face au code

**Version** 1.0 · **Date** 2026-11-12 · **Statut** Livré · **Base** version 0.0.2

Plan du suivi de la maquette : pour chaque écran dessiné, savoir ce qui est livré, ce qui reste et
où le code diverge. Il respecte les règles de [`CLAUDE.md`](../CLAUDE.md).

Source : maquette « Application Tests Automatiques » (Claude Design), réorganisée le 12 novembre
2026 : pages codées par groupe (0 Cadre à 7 Équipe), fiche en tête de chaque page, index en page
0.1. Les captures de l'application réelle viennent des tests e2e (`e2e/artifacts/`).

## 1. Objectif

À la fin de ce plan :

- chaque écran de la maquette a un repère stable (`3.2-a2`) et un état tenu dans le dépôt ;
- les écarts entre un écran et l'application sont écrits, avec ce qu'il faut aligner ;
- la CI refuse un état incohérent ou un repère cité qui n'existe pas ;
- la maquette affiche ces états, ces écarts et les captures de l'application réelle.

## 2. Repères

Un repère d'écran est le code de sa page suivi de sa lettre : `3.2-a2` est l'écran a2 de la page
3.2 (Endpoint). Une lettre n'est jamais réutilisée : un écran supprimé laisse un trou. Un écran
déplacé vers une page existante prend la lettre libre suivante ; une page découpée garde ses
lettres. La page 0.1-d de la maquette donne la correspondance avec les anciens repères (« 2a »,
« Premier usage 1 b », captures `docs/maquettes/1a` à `6c`).

## 3. Règles

| ID  | Règle                                                                                                                                                                                                                                                 |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | [`maquette-etat.yml`](maquette-etat.yml) est la source de vérité ; la maquette en est l'affichage (pastilles, écarts, fiches, index, captures).                                                                                                       |
| R2  | États d'un écran : livré, en partie, à faire, variante, remplacé, à décider ; référence pour les pages de cadre. Un écran est jugé sur sa propre fonction : ce qu'il montre d'autres pages (mocks, catalogue, autres natures) relève de ces pages.    |
| R3  | Un écran livré ou en partie cite sa pull request (`pr`) ou le plan de sa page (`plans`).                                                                                                                                                              |
| R4  | Un écart dit ce qui diffère (`texte`) et quoi aligner (`aligner`) : `maquette` (redessiner l'écran), `code` (tâche d'un plan) ou `à décider` (question à trancher). Un écart commun à plusieurs écrans va dans `transverses`.                         |
| R5  | `captures` nomme les captures e2e (`screenshot(browser, "…")`) qui montrent l'écran dans l'application réelle.                                                                                                                                        |
| R6  | Une pull request qui livre, modifie ou écarte un écran met le fichier à jour (case du modèle de pull request) ; un plan cite ses écrans par repère.                                                                                                   |
| R7  | `scripts/check-maquette.mjs` (`npm run check`, CI) vérifie la structure, les états, les plans, les captures, la traçabilité (R3), les écarts (R4) et que tout repère cité dans la documentation existe.                                               |
| R8  | Après une fusion, une session Claude régénère la maquette depuis le fichier avec `scripts/maquette-sync.py` (pages lues sur claude.ai dans `.maquette/pages/`, captures e2e) ; sa fiche et son index disent de quel commit de `Develop` ils viennent. |
| R9  | La page 0.5 (Design system) est générée en entier depuis `docs/design-system/` ; un écart transverse qui touche le design system porte `page: "0.5"` et s'affiche en 0.5-e.                                                                           |

## 4. Audit du 12 novembre 2026

Dix-sept écrans livrés ou en partie ont été comparés côte à côte avec les captures e2e de `Develop`
(après la PR #42). Résultat : 12 écarts d'écran et 4 écarts transverses (barre de titre, barre
d'état, vocabulaire test / assertion, environnement « Local » par défaut). L'écran 5.1-c passe en
partie (chronologie réduite à attente et réception).

| Aligner   | Écrans                                                                  |
| --------- | ----------------------------------------------------------------------- |
| maquette  | 3.2-a, 5.2-f ; transverses : barre de titre, vocabulaire, environnement |
| code      | 1.1-a, 1.2-d, 4.2-b ; transverse : barre d'état                         |
| à décider | 1.1-e, 2.2-a, 3.1-e, 3.1-g, 3.2-b, 3.3-a, 4.4-c                         |

## 5. Tâches

- [x] **T1** — `docs/maquette-etat.yml` : 38 pages, 189 écrans, états, pull requests, plans,
      captures et écarts.
- [x] **T2** — `scripts/check-maquette.mjs`, dans `npm run check` et la CI (étape « Maquette et
      code ») ; dépendance de développement `yaml`.
- [x] **T3** — Modèle de pull request (case « maquette-etat.yml à jour ») et modèle de plan (repères
      dans le tableau « Maquettes ») ; `CLAUDE.md`.
- [x] **T4** — Audit des écrans livrés face aux captures e2e (§ 4).
- [x] **T5** — `scripts/maquette-sync.py` : maquette régénérée depuis le fichier (pastilles,
      écarts, fiches, index, captures de l'application réelle sous chaque écran).

## 6. Hors périmètre

- Régénérer la maquette depuis la CI : elle vit sur claude.ai, sans accès depuis GitHub. Une session
  Claude la régénère sur demande (« synchronise la maquette »).
- Comparer automatiquement les images : la comparaison reste un regard, appuyé sur les captures.
