# Phase 2 — Mise en place du modèle structuré et du moteur

**État au 22 octobre 2026 : phase commencée, pas terminée.** Les orientations
V1–V6 de [l’inventaire](commands-inventory.md) sont validées. Les premiers lots
portent sur les blocs, les zones, leurs identités, les cibles de sélection, puis
la protection des frontières et la publication commune des éditions.
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

## Protection des frontières et publication commune

Le découpage en blocs et zones sert maintenant aussi à la validation du tampon,
sans rapprochement approximatif de chaînes avant/après. Une suppression ou un
remplacement est accepté dans une zone textuelle, entre paragraphes du même
corps ordinaire, ou lorsque les structures traversées sont entièrement couvertes.
Les séparateurs terme/définition, corps/attribution, objet/légende et les jonctions
avec un bloc composé partiellement sélectionné sont protégés. Les refus sont
signalés dans la barre d’état, en français ou en anglais.

Une suppression totale laisse un paragraphe vide. Effacer seulement une légende
ou une attribution conserve son parent ; supprimer tout le bloc retire aussi ses
annotations. L’insertion d’une image ne scinde plus implicitement une description,
une citation ou une légende : elle doit cibler un paragraphe ordinaire, une frontière
de bloc ou une structure entièrement sélectionnée. L’insertion avant un bloc et
après une attribution vide conserve les identités de ses zones existantes.

Le tampon `TextInput` est partagé par la saisie egui et les remplacements explicites.
Il conserve la première erreur et refuse les gestes suivants du même brouillon :
une suppression refusée ne peut donc pas être suivie d’une insertion publiée.
Les séparateurs collés dans les zones qui n’acceptent qu’une ligne (terme,
définition, attribution et légende) sont refusés avec un message, sans perte de
la sélection. Le caractère objet interne ne devient pas une insertion textuelle.

La saisie native prépare désormais une copie, au lieu de modifier le document
publié pendant `TextEdit`. Les éditions de mise en forme, les remplacements
simples/multiples et les corrections orthographiques rejoignent la même voie de
`Session` : vérifier la session/révision et le brouillon, normaliser une copie,
puis publier contenu, sélection, révision et historique ensemble. Les commandes
qui préparent encore directement leurs champs dans `App` passent par un adaptateur
transitoire : il rétablit l’instantané avant de valider leur candidat. Ce passage
ne constitue pas le moteur typé ni la fin de leur migration.

Un brouillon invalide restaure aussi les métadonnées, le modèle, les dépendances
et les ressources ; un brouillon périmé ne remplace pas un commit intervenu entre
temps. Un remplacement multiple est annulé en entier si une occurrence échoue.
Aucune erreur ne modifie la révision, les piles d’annulation/rétablissement ou
l’horodatage d’édition. Une mise en forme au curseur seul met à jour le contexte
de frappe, sans marquer le document modifié ni ajouter d’étape d’historique.
Le remplacement d’un texte reprend son style de départ, au clavier comme par
appel explicite. Remplacer tout le texte d’un titre conserve son niveau ; effacer
la sélection totale du document laisse un paragraphe vide. Les compteurs et le
rendu continuent de lire le document publié.

## Vérifications

- 181 tests de l’application réussis, dont 38 tests ajoutés depuis le début de
  phase 2 : identités et cibles, refus des frontières, suppressions complètes,
  refus atomiques, brouillons périmés, ressources, contexte de frappe et parité
  des remplacements Unicode entre événements egui et appels explicites.
- Clippy sur toutes les cibles, avec le diagnostic `editor-model-spike`, sans
  avertissement ; ses 104 tests partagés réussissent aussi.
- Formatage conforme et nouveaux fichiers de production sous 800 lignes.
- Huit captures strictement identiques après les premiers lots, dans
  `output/baseline/run-ioma5t0l/` et `output/baseline/run-uvqd1xwk/`, puis après la
  voie commune dans `output/baseline/run-rot7603t/`. La comparaison finale sur le
  code de ce lot est également identique, dans `output/baseline/run-do7d7apa/`.
- Le scénario existant qui essayait une insertion d’image à chaque caractère
  conserve ses contrôles de validité. Il vérifie désormais soit l’insertion aux
  positions autorisées, soit le refus sans mutation dans une zone protégée.

La CI du lot précédent a signalé la dépréciation de `AtomicU64::fetch_update`
par sa version de Rust stable, plus récente que l’installation locale. Les deux
allocateurs utilisent désormais `identity::next`, fondé sur
`compare_exchange_weak`, sans changer le minimum Rust déclaré. Deux tests
vérifient l’unicité concurrente et le refus de boucler au dépassement de capacité.

## Suite de la phase

Les indices de recherche et la synchronisation d’aperçu fonctionnent encore via
le tampon graphique et les cartes de source existants. Leur migration n’est pas
déclarée terminée par les tests de conversion des positions. Les raccourcis de
transformation textuelle restent préparés par l’interface, puis validés par la
voie commune ; leur déplacement dans le cœur reste à faire (V4).

La préédition IME reste à isoler dans un brouillon conservé entre frames, avec
publication unique à validation. Le tampon de saisie est maintenant distinct du
document pendant une frame, mais cet ajout n’est pas présenté comme la réalisation
de cette isolation entre frames. D17 et la tâche 2.1 bis restent donc ouverts sur
ce point ; l’essai d’une vraie méthode IME du système reste dans le chantier I.

Viennent ensuite le registre typé, les erreurs et schémas, `Engine`, puis la
migration des familles de commandes et le catalogue généré. La séparation des
réglages globaux (V5) et les formats d’export séparés (V6) restent à implémenter
et à tester.
