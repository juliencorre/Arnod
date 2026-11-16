<!--
Titre en Conventional Commits, vérifié par la CI (pr-title.yml) :
feat: …, fix: …, docs: …, refactor: …, test: …, ci: …, chore: …
Exemple : « feat: gestion des workspaces (lot F) ».
-->

## Objet

<!-- Ce que change cette pull request et pourquoi, en quelques lignes. -->

## Plan et cas d'usage

<!--
Plan de référence (docs/…), lot, tâches livrées (T…), cas d'usage (P…, W…) et règles (R…)
concernés, repères des écrans de la maquette (3.2-a2…). Issue liée : « Closes #… ».
-->

## Vérifications

- [ ] `npm run check` passe (format, lint, typecheck, Vitest, `cargo fmt`, clippy, `cargo test`)
- [ ] Tests e2e étendus si le changement est visible (`e2e/`)
- [ ] Tâches cochées dans le plan ; `CLAUDE.md` et README à jour si besoin
- [ ] `docs/maquette-etat.yml` à jour : écrans livrés (repère, PR, captures) et écarts avec la maquette
- [ ] Aucun nouveau plugin ni nouvelle permission Tauri, ou décision notée dans le plan

## Captures

<!-- Si l'interface change : captures en thème clair et en thème sombre. -->
