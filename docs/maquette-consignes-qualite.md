# Consignes - Maquette de référence du plan de qualité

**Date** 2026-11-17 · **Pour** une session Claude Code sur claude.ai qui a accès à la maquette
(Claude Design) · **Source** [`plan-qualite-mvp.md`](plan-qualite-mvp.md), lot 0

La maquette est la référence : les propositions du plan de qualité qui changent un écran y sont
dessinées d'abord ; le code les rattrape ensuite, lot par lot. Chaque divergence entre l'écran
redessiné et le code est déjà notée dans [`maquette-etat.yml`](maquette-etat.yml), avec
`aligner: code` et le numéro du constat de l'[audit](audit-2026-11-mvp.md) (DE, ER, AC).

## 1. Déroulé

1. Lire les pages actuelles de la maquette dans `.maquette/pages/` (fichiers `.dc.html` et
   `compat.json`), depuis la branche de ce plan.
2. Redessiner les écrans du § 3 dans ces fichiers, en ne touchant que ce qui est décrit : le reste
   de chaque écran reste tel quel. Thème clair, comme les autres écrans.
3. Régénérer pastilles, écarts, fiches et index :
   `python3 -I scripts/maquette-sync.py "claude/audit-mvp @<commit>"` (toutes les pages). Un écran
   porte maintenant une liste d'écarts quand il en a plusieurs.
4. Ne pas lancer `scripts/maquette-couleurs.py` sur ces écrans : la teinte du nouveau bord de
   contrôle n'est pas encore un jeton du code ; la page 0.5-e la listera comme teinte hors design
   system jusqu'au lot I.
5. Republier `.maquette/sortie/` et ajouter au journal de chaque page touchée une ligne datée
   « Redessiné selon plan-qualite-mvp (maquette de référence) : … ».

## 2. Valeurs à employer

| Élément                           | Clair                                        | Sombre             |
| --------------------------------- | -------------------------------------------- | ------------------ |
| Bord de case, interrupteur, radio | `#8f8a83` (3,43:1 sur surface)               | `#7d7871` (3,67:1) |
| Anneau de focus                   | `#9a5a0c` (`--color-focus`)                  | `#f0b46a`          |
| Texte secondaire neutre           | `#6b665e` (`--color-muted`)                  | `#b0a99e`          |
| Action textuelle, lien            | `#9a5a0c` (`--color-link`), graisse 600      | `#f0b46a`          |
| Compteur                          | pastille (comme « En-têtes 2 » de l'éditeur) |                    |
| Sélection                         | fond `#fdf1e1` (`--color-accent-soft`)       | `#3a2c1a`          |
| Ajouter une ligne                 | bouton secondaire « + Nom » (« + En-tête »)  |                    |
| Retirer une ligne                 | × sans bord, cible de 24 × 24 px             |                    |
| Bouton plein                      | un seul par vue : `#e8962e`, texte `#1d1b18` |                    |

## 3. Écrans

| Repère | Ce qui change dans la maquette                                                                                                                                                                                                                                      | Constat       |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- |
| 0.5    | Page générée : ne pas la redessiner. Les écarts transverses (bords, couleurs de texte, formes uniques) s'y affichent.                                                                                                                                               | DE4, DE7      |
| 1.2-a  | Une ligne au nom long, coupée, montre son infobulle au survol ; ⋯ n'apparaît qu'au survol et au focus.                                                                                                                                                              | DE5           |
| 1.2-d  | Les quatre points de départ sur quatre colonnes, « + Environnement » sur une ligne.                                                                                                                                                                                 | DE6           |
| 1.6-e  | Un bouton ✕ « Fermer » dans l'en-tête des préférences (vaut aussi pour 1.6-g).                                                                                                                                                                                      | ER4           |
| 1.6-g  | « Vérifier maintenant » en couleur de lien. Ajouter la même section en thème sombre si la page n'en a pas, pour montrer qu'il reste lisible.                                                                                                                        | DE2           |
| 2.1-a  | « Essayer » seul bouton plein ; « Créer le projet » en bouton secondaire ; champ et boutons à la hauteur de contrôle (32 px).                                                                                                                                       | DE9           |
| 3.3-a  | Plus de bouton « Enregistrer » : « ✓ Enregistré » à sa place ; le reste de l'écran (tableau unique, portée, utilisation, « Copier depuis un autre projet… ») ne change pas.                                                                                         | ER2, Q5       |
| 3.4-a  | Même chose pour les réglages du projet : « ✓ Enregistré » au lieu d'« Enregistrer ».                                                                                                                                                                                | ER2           |
| 4.1-g  | La note « après l'étape n » en couleur de texte.                                                                                                                                                                                                                    | DE2           |
| 4.2-a  | Barre des étapes d'un test de six étapes sur une rangée qui défile horizontalement, l'étape active visible, un fondu au bord coupé.                                                                                                                                 | DE8           |
| 4.2-b  | Un échec d'assertion affiché une fois, dans une liste dépliable ; « Ignorer pour ce résultat » ; liens d'action en couleur de lien, graisse 600, sans soulignement.                                                                                                 | DE10, ER3     |
| 4.2-c  | La note du coffre en couleur de texte.                                                                                                                                                                                                                              | DE2           |
| 5.1-b  | Panneau de réponse vide : « Envoyez la requête pour voir la réponse. » en texte secondaire neutre, sans fond ; vue partagée à 1280 px avec l'adresse entière et les onglets sur une ligne ; sous une erreur d'envoi, son action (« Ajouter », « Environnements… »). | DE1, DE3, ER6 |
| 5.1-h  | Barre des étapes sur une rangée qui défile, comme 4.2-a.                                                                                                                                                                                                            | DE8           |

Sur tous les écrans qui en montrent : cases, interrupteurs et radios au bord de contrôle du § 2 ;
compteurs en pastille ; sélection en fond accent doux ; une notification de suppression porte
« Annuler ».

## 4. Ce qui ne change pas

Les corrections de clavier, de focus, de performance, de secrets, de brouillons et du cœur ne se
dessinent pas : elles sont suivies par l'écart transverse « Clavier et cibles » et par le plan.
Aucune page n'est ajoutée ; aucun écran n'est retiré.
