# Product

<!-- impeccable:product-schema 1 -->

Contexte produit lu par la skill Impeccable (`.claude/skills/impeccable/`). Écrit d'après le dépôt
(`README.md`, `PRD-squelette.md`, `CLAUDE.md`, plans de `docs/`) ; il ne décrit aucun choix visuel :
le design system est dans `DESIGN.md`, généré depuis `src/styles.css`.

## Platform

web

Interface Vue 3 rendue par la WebView de Tauri 2 (WebView2 sous Windows, WebKitGTK sous Linux) :
application de bureau, pas un site. Une seule fenêtre principale, des fenêtres détachées, aucun
écran mobile.

## Users

Testeurs et développeurs qui vérifient des API en entreprise : ils écrivent et rejouent des
requêtes, des tests de plusieurs étapes et des campagnes, souvent derrière un proxy d'entreprise,
sur un poste Windows sans droits administrateur. Ils viennent de SoapUI et de Bruno et passent des
heures par jour dans l'outil, clavier en main.

## Product Purpose

Remplacer SoapUI et Bruno par une seule application de test : des étapes de toutes natures (REST
aujourd'hui ; SOAP, base de données, messages, traitements et écrans ensuite) dans un même test.
Réussir, c'est envoyer, vérifier et rejouer sans friction, et comprendre un échec du premier coup.

## Positioning

Un projet est une collection OpenCollection 1.0.0 (format ouvert, lu aussi par Bruno), rangée dans
un dossier que l'équipe versionne. Les contrats OpenAPI créent les requêtes et vérifient chaque
réponse. Un essai sans projet est gardé dans « Non classés », sous le contrat déduit de son serveur.
Tout le travail se fait dans un cœur natif Rust ; l'interface ne fait qu'afficher.

## Operating Context

- Poste Windows d'entreprise, installation sans droits administrateur (NSIS en utilisateur
  courant) ; Linux en AppImage.
- Proxy et autorités de certification internes ; la vérification TLS n'est jamais coupée.
- Usage dense : barre latérale de projets, onglets dans des zones redimensionnables, éditeur et
  résultat côte à côte à partir de 1280 × 775, fenêtres détachées sur un second écran.
- Fichiers du projet modifiables hors d'Arnod (Git, éditeur de texte) : Arnod suit et signale.

## Capabilities and Constraints

- Mots de l'interface (page 0.2 de la maquette) : « test », « étape », « assertion », « requête
  libre », « contrat de service », « endpoint », « Non classés ». Textes en français.
- CSP stricte : aucune ressource distante, donc police système uniquement, aucune image ni police
  chargée d'ailleurs.
- Thème clair et sombre suivant le système, mêmes jetons dans les deux.
- Pas de nouveau plugin Tauri ni de permission de la WebView sans besoin explicite.
- Hors périmètre actuel : SOAP, mocks, scripts, CLI, signature de code.

## Brand Commitments

- Nom : Arnod. Accent orange (`#e8962e`), un seul bouton plein par vue.
- La maquette (Claude Design, pages codées 0 à 7) et le design system « Arnod »
  (`docs/design-system/`) sont la référence ; un écran change d'abord dans la maquette
  (`docs/maquette-etat.yml`, `docs/maquette-consignes-qualite.md`).

## Evidence on Hand

- Captures de l'application réelle : `docs/screenshot-linux.png`, `docs/screenshot-linux-dark.png`,
  captures e2e citées dans `docs/maquette-etat.yml`.
- Audit du MVP : `docs/audit-2026-11-mvp.md` (99 constats) et ses preuves.
- Aucun client, témoignage, chiffre d'usage ni tarif : ne rien inventer de tel.

## Product Principles

1. Le cœur décide, l'interface affiche : aucune logique métier dans un composant.
2. Rien n'est perdu sans le dire : enregistrement automatique, « Annuler » après une suppression.
3. Un état, une forme : un même état s'écrit partout de la même façon.
4. Le clavier d'abord : toute action glissée a son équivalent au clavier.
5. Un échec s'explique : cause, ce qui est intact, et l'action pour s'en sortir.

## Accessibility & Inclusion

WCAG 2.1 AA : texte à 4,5:1, bords de contrôle et focus à 3:1 dans les deux thèmes (vérifié par
`tests/theme.spec.ts`) ; motifs WAI-ARIA complets au clavier (arbre, onglets, menus, séparations) ;
jamais la couleur seule pour un statut (✓ / ✕).
