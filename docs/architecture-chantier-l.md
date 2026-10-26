# Chantier L — Polissage de l'interface : bilan

Bilan du 25 octobre 2026 pour la décision D40. Les analyses et maquettes d'origine sont
dans [interface-design-audit.md](interface-design-audit.md) et
[mockups/](mockups/). Le plan détaillé est dans la section « Chantier L » de
[ROADMAP-ARCHITECTURE.md](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md).

## Ce qui est livré

| Étape | Résultat | Commit |
| --- | --- | --- |
| L.0 | Choix validés : police embarquée, marque retirée, annuler/rétablir dans la barre, durée de rendu en info-bulle, bouton-menu unique P.-S./P.J. | `166c91c` |
| L.1 | Module `theme` : jetons clair/sombre, échelles, rapport de contraste, constantes unifiées | `a8e5742` |
| L.2 | `ui_kit` : boutons à variantes, champs, contrôle segmenté, `Dialog`, notifications, touche de raccourci | `057041e` |
| L.2 bis | `SideRail` : barre latérale droite et tiroir, **sans contenu** ni épinglage | `eb19d0c` |
| L.3a | Palette appliquée au style global d'egui ; l'accueil reçoit un fond | `c902eec` |
| L.3b | Huit dialogues sur l'enveloppe commune | `ed5f348` |
| L.3 (police) | Inter 4.0 normal et semi-gras (SIL OFL 1.1) | `786e9d9` |
| L.3 (barre, infos) | Barre d'outils à icônes, menu « fin de lettre », annuler/rétablir ; rubriques segmentées | `c192b1f` |
| L.3 (recherche…) | Recherche, palette de commandes (calque non modal), accueil | `00cb58d` |
| L.3 (état…) | Barre d'état à pastilles, aperçu sur fond « table », relecture MCP | `b084f27` |
| L.4 | Bandeaux dans le cadre des widgets (variante B), pointe vers le mot corrigé | `6a82961` |
| L.5 | Recette : captures, contrastes, ce bilan | (ce lot) |

## Vérifications

- **Tests** : 583 tests réussis lors de la recette (584 après les compléments) et aucun échec sur l'ensemble de l'espace de travail
  (`cargo test --locked --workspace`) ; 208 pour le crate `typstographe` ; 14 tests
  Python des scripts ; `clippy -D warnings`, `cargo fmt --check`, limite de taille des
  fichiers de production et frontière de dépendances du cœur réussis.
- **Captures natives** : 20 scénarios (8 historiques et 12 de widgets, clair et sombre)
  enregistrés avec `scripts/capture-baseline.sh --record --output output/phase-l5`,
  puis une **seconde passe strictement identique octet pour octet** (`--compare`, D19).
  Profil `native-fr-v2` : les anciennes références (profil `native-light-fr-v1`,
  8 scénarios clairs) restent dans `output/baseline/` et ne sont plus comparables, la
  présentation ayant changé volontairement. Les références ne sont pas versionnées
  (`output/` est ignoré), comme pour les phases précédentes.
- **Contrastes** (WCAG 2.x, tests `theme::tests` et `editor::tests`) : texte ≥ 4,5:1 sur
  toutes les surfaces et tous les états interactifs ; bordure de contrôle ≥ 3:1.

  | Paire | Clair | Sombre |
  | --- | --- | --- |
  | texte / fond | 14,3 | 13,8 |
  | texte secondaire / fond | 5,4 | 6,5 |
  | accent / panneau | 7,3 | 8,1 |
  | erreur / panneau | 6,5 | 6,2 |
  | bordure de contrôle / champ | 3,7 | 3,6 |
  | texte sur bouton principal | 7,3 | 8,5 |

- **Clavier et IME** : les scénarios existants (Entrée, Échap, Tab, composition IME,
  raccourcis personnalisés, focus des bandeaux, redimensionnement) passent tous. La
  composition est simulée par les événements egui, comme dans les phases précédentes ;
  **une vraie méthode de saisie du système n'a pas été essayée** (reste au chantier I).

## Ce que la recette a trouvé

1. Le bouton destructif, pressé, passait sous 4,5:1 en clair puis en sombre : son fond
   reste désormais le fond doux, sans changement à l'appui.
2. La couleur d'italique de l'éditeur en mode clair était à 4,46:1 sur le fond : elle
   est légèrement assombrie (18, 118, 83).
3. Des tests verrouillent ces deux points et la palette des états interactifs.

## Compléments du 25 octobre 2026 (après la recette)

- **Pointe du lien** : le bandeau de lien pointe désormais vers le texte sélectionné,
  comme celui de l'orthographe ; les paramètres de bloc restent sans pointe.
- **Options de liste** : les trois types (à puces, numérotée, descriptions) sont en
  contrôle segmenté, comme les autres choix de l'application. `segmented_focused` donne
  le focus clavier au segment du type actuel à l'ouverture, ce qui conserve le
  comportement testé ; les marges des segments se réduisent en colonne étroite.
  Aucun test n'a dû être supprimé : tous les scénarios clavier passent tels quels.
- **Icônes Lucide** : les sept icônes recopiées à la main sont remplacées par les fichiers
  officiels de Lucide 0.468.0, téléchargés depuis le dépôt du projet (README des icônes).
- Références ré-enregistrées (`output/phase-l5`, 20 scénarios, seconde passe identique) ;
  584 tests réussis sur l'espace de travail.

## Finitions du 25 octobre 2026 (relecture)

À la relecture, six défauts de présentation ont été corrigés. Les trois premiers
relevaient du rendu de l'éditeur, antérieur au chantier L mais mis en évidence par le
nouveau style (commit `d280053`) ; les autres des boutons et des marges (commit suivant).

- **Interligne** : de 1,55 à 1,35 (`LINE_HEIGHT`). Le curseur d'egui prend la hauteur de
  la ligne entière : il était disproportionné par rapport au texte.
- **Puces et numéros** : alignés sur la ligne de base du texte (ils étaient centrés sur la
  ligne, donc trop bas) ; les descriptions n'ont plus de puce.
- **Termes des descriptions** : egui cale chaque section sur le bas de sa propre ligne ;
  la ligne du terme, plus petit, est raccourcie de la différence des ascendantes pour que
  sa ligne de base soit celle du texte suivant.
- **Boutons** : règle unique — l'action principale tout à droite, les autres actions à
  droite ; l'action destructive **secondaire** (Ne pas enregistrer, Révoquer, Supprimer la
  copie, Retirer le lien) à part, tout à gauche (`footer_split`). Quand l'action
  destructive est la confirmation (Supprimer un widget) ou l'une des deux décisions
  (Refuser/Accepter le lot), elle reste à droite avec son style destructif.
- **Aération des bandeaux** : marge basse de 12 points et 8 points réservés sous le
  bandeau, pour que la pointe et la bordure n'effleurent pas le texte, hors bloc comme
  dans le cadre d'un bloc.
- Les fonctions devenues trop longues ont été découpées (`apply_footer`,
  `assistance_footer`, `recovery_entry`) pour rester sous le seuil de 120 lignes.

Références ré-enregistrées (20 scénarios, seconde passe identique) ; 584 tests réussis.

## Navigation clavier — corrections après la revue du design

- **Palette de commandes** : la sélection reste visible avec les flèches, y compris
  au passage entre la fin et le début du catalogue. Le défilement est immédiat et
  intervient à la navigation clavier, à une nouvelle recherche ou à la prise de focus
  du champ ; il ne ramène pas la liste en arrière pendant l'usage de la molette.
- **Contrôles segmentés** : un contour d'accent distingue le focus clavier de la
  valeur sélectionnée, dans les thèmes clair et sombre, sans changer leur géométrie.
- Quatre tests de régression couvrent le parcours du catalogue dans les deux sens,
  le défilement manuel puis la recherche, la réouverture de la palette et la
  navigation Tab/Entrée sur les segments sélectionnés ou non.
- Vérification : 588 tests Rust (doctests compris), 14 tests Python, Clippy sans
  avertissement, formatage et contrôles d'architecture réussis. Les 20 captures
  natives restent strictement identiques aux références de `output/phase-l5`.

## Limites connues et suite

- Sous environ 900 points de large, le contenu de l'éditeur déborde déjà de sa colonne
  (constaté pendant L.4, indépendant du chantier) ; le minimum de fenêtre est 960.
- La barre latérale (`SideRail`) est posée sans panneau ni commande d'ouverture : les
  commandes arriveront avec le premier panneau réel (chantier G de `ROADMAP.md`).
- Un redimensionnement qui change le nombre de lignes d'un bandeau le rogne pendant une
  frame (mesure puis application) ; le test tolère cette frame.

## Rejouer la recette

```sh
cargo test --locked --workspace
scripts/capture-baseline.sh --compare --output output/phase-l5
cargo run -p typstographe --example ui-kit-gallery -- sortie.png dark gallery
```
