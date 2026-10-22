# Phase 2 — Mise en place du modèle structuré et du moteur

**État au 22 octobre 2026 : phase commencée, pas terminée.** Les orientations
V1–V6 de [l’inventaire](commands-inventory.md) sont validées. Les premiers lots
portent sur les blocs, les zones, leurs identités et les cibles de sélection.
Le moteur de commandes et son catalogue ne sont pas encore implémentés.

## Blocs et zones en mémoire

`src/document/structure.rs` décrit les blocs existants : paragraphes et titres,
éléments de listes, descriptions, citations, images, P.-S. et mentions de P.J.
Les descriptions possèdent des zones terme/définition ; les citations des zones
corps/attribution ; les images une zone de légende et leurs propriétés d’image.
Chaque zone réutilise la projection textuelle et ses styles. Les annotations
vides ont une identité, sans texte indicatif enregistré. Les lignes vides du
tampon restent représentées pour préserver exactement ses séparateurs actuels.

La structure est reconstruite à l’ouverture depuis la projection du Typst.
Lors d’un commit, elle est actualisée après la normalisation existante, puis
alimente le sérialiseur Typst via un adaptateur vers la projection. Les sources,
cartes de source, styles et ressources restent identiques. Aucun nouveau format
disque, aucune dépendance et aucune fonctionnalité d’édition ne sont ajoutés.

Pendant cette migration, le tampon plat reste l’adaptateur de saisie d’egui.
Des ancres de lignes suivent ses insertions, suppressions et transformations.
La réconciliation des identités utilise ces ancres et les rôles, jamais la
ressemblance du texte ni le numéro de ligne. Les identifiants sont opaques,
distincts pour les blocs et les zones, et alloués sans réutilisation pendant le
processus. Ils ne sont pas sérialisés et ne participent pas à l’état modifié.

Annuler/rétablir restaure les identités avec les instantanés. Une nouvelle branche
d’historique reçoit de nouvelles identités. Scinder une citation conserve
l’identité de son attribution avec le texte correspondant, même si elle passe
dans le second bloc. Un remplacement sans changement conserve les ancres et la
révision précédentes, sans créer d’étape d’historique.

## Positions et garde de révision

Une position désigne une zone et un offset en valeurs scalaires Unicode, ou une
frontière avant/après un bloc. La conversion vers le tampon graphique préserve
les deux extrémités et le sens de la sélection, sans exposer ses sentinelles.
Les positions hors limites et les identités inconnues sont refusées.

La session fournit une garde contenant son identité et sa révision. Une cible
provenant d’une autre session ou d’une ancienne révision est refusée, y compris
après une annulation qui restaure sa zone. Les lectures de sélection ne changent
pas la révision ; les modifications de métadonnées l’invalident également.
Les instantanés conservent désormais leur sélection logique pour la restaurer
avec les objets et les affinités de curseur. Cette restauration interne actualise
la révision de l’instantané ; aucune actualisation n’est faite sur une cible externe.

L’adaptateur graphique conserve encore ses indices globaux. Si une sélection
graphique transitoire n’est pas adressable dans un instantané, sa restauration
conserve le comportement numérique antérieur ; les nouvelles cibles explicites
restent strictement validées. Les types Rust sont internes : les signatures et
schémas JSON du registre ne sont pas gelés.

## Vérifications

- 163 tests de l’application réussis, dont 20 nouveaux tests de structure et de
  cibles : Unicode, rôles, aller-retour Typst, cartes de source, ressources,
  identités, séparation/jonction, annulation/rétablissement, branche d’historique,
  gardes périmées et sessions distinctes.
- Clippy sur toutes les cibles, avec le diagnostic `editor-model-spike`, sans
  avertissement ; ses 90 tests, incluant les modules partagés, réussissent aussi.
- Formatage conforme et nouveaux fichiers de production sous 800 lignes.
- Huit captures strictement identiques après le premier lot de blocs, dans
  `output/baseline/run-ioma5t0l/`, puis après les cibles et leur raccordement à
  l’historique, dans `output/baseline/run-uvqd1xwk/`.

## Suite de la phase

La représentation et l’historique disposent du socle d’identités. Il reste à
protéger les remplacements partiels entre rôles (V2), préparer les éditions sur
une copie et unifier leur validation/normalisation/publication (D17). Les indices
de recherche et la synchronisation d’aperçu fonctionnent encore via le tampon
graphique et les cartes de source existants. Leur migration n’est pas déclarée
terminée par les tests de conversion des positions.

Viennent ensuite le registre typé, les erreurs et schémas, `Engine`, puis la
migration des familles de commandes et le catalogue généré. La séparation des
réglages globaux (V5), les formats d’export séparés (V6) et la préédition IME à
publication unique restent à implémenter et à tester.
