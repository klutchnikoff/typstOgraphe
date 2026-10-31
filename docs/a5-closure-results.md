# A.5 — Documentation, nettoyage et recette finale

**31 octobre 2026 — qualification en cours.** Ce bilan distingue les contrôles
locaux de la qualification CI multisystème. Il ne vaut pas validation matérielle
ni autorisation de distribution signée.

## Périmètre

Les choix validés sont conservés : documentation courante séparée des bilans,
README bref, nettoyage ciblé sans réécriture du moteur, diagnostics Rust sous
`recipe-driver`, frontend Vite de recette distinct, puis même corpus Courrier/
Article sur Tauri, CLI et MCP. Aucun nouveau jeu de captures n’est produit.
Les rapports, résultats, licences et références visuelles existants sont conservés.

## Nettoyage et documentation

- [Index documentaire](README.md), guide de développement et contrats à jour ;
  phases closes retirées des roadmaps, indications historiques explicites.
- Diagnostics natifs et IPC de contrôle absents de la construction normale ;
  options de recette refusées avant tout accès aux projets ou création de fenêtre.
  La recette exige un répertoire explicitement isolé.
- Frontend normal sans pont `__spike`, commande d’essai Rust ni modules de mesure.
  `check:distribution` contrôle les **assets réellement produits**, aussi en CI.
- `build:recipe` associe `dist-recipe/` et `recipe-driver` ; il conserve `dist/`.
  La CI native utilise ce même parcours. Les deux modes utilisent le même moteur
  documentaire ; aucune validation simplifiée n’est introduite pour la recette.
- Commande `recover` sans appelant et commande d’essai `prepare_rust_edit` retirées ;
  la recette utilise maintenant `block.insert` du contrat public. `jsonschema`
  direct de l’hôte et `lib0` direct du frontend retirés ; `zip` de l’hôte déplacé
  en dépendance de test. Les dépendances transitives utiles restent verrouillées.
- Licence MIT du projet déclarée ; licences, provenances et inventaire Typst tiers
  conservés. Aucun changement de l’identifiant natif ni des préférences existantes.

## Recette à qualifier

Depuis un worktree propre, installer avec les lockfiles, construire les assets
normaux et de recette, exécuter les tests et lancer réellement la WebView native.
La recette `test:closure-native` couvre le même Courrier et Article avec édition
et métadonnées, MCP rattaché/autonome, refus/stale/acceptation humaine, version
nommée, reprise par la modale et une annulation/rétablissement, sauvegarde,
réouverture et exports hors ligne. Les sources, ressources et dépendances sont
comparées octet par octet entre adaptateurs ; les PDF sont identifiés et les
dimensions de pages comparées, sans exiger l’identité de leurs métadonnées variables.

Résultats, révisions qualifiées et limites seront consignés après exécution.
