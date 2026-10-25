# Chantier L — Polissage de l'interface : bilan

Bilan du 25 octobre 2026 pour la décision D40. Les analyses et maquettes d'origine sont
dans [interface-design-audit.md](interface-design-audit.md) et
[mockups/](mockups/). Le plan détaillé est dans la section « Chantier L » de
[ROADMAP-ARCHITECTURE.md](../ROADMAP-ARCHITECTURE.md).

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
