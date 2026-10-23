# Relecture de sortie de phase 2

Relecture du **24 octobre 2026**, sur le code `39245c4edbafb9067231c84f11257056b05aabb5`,
complétée par deux tests de presse-papiers. **Le catalogue n’est pas encore gelé et
la phase n’est pas déclarée terminée.** Le périmètre de gel proposé ci-dessous
reste à valider ensemble.

Références : [feuille technique](../ROADMAP-ARCHITECTURE.md),
[inventaire relu](commands-inventory.md), [catalogue](commands.md),
[schémas d’entrée](commands.json), [historique des lots](architecture-phase-2.md).
Les tableaux de migration restent un relevé de départ ; ils ne constituent pas
une seconde définition des paramètres à côté du registre Rust.

## 1. État vérifié

- 49 commandes typées et 14 requêtes, soit **63 entrées** générées depuis les enums.
- 31 variantes de `UiCommand`, 41 définitions communes, deux actions de Courrier
  et un modèle fourni : 44 présentations pour Courrier, 41 pour Compte-rendu.
  Ce dernier type reste sans modèle fourni et sa création graphique indisponible.
- 38 commandes peuvent être étapes de `document.batch`. Fichiers, récupération,
  export, historique, requêtes et lots imbriqués sont exclus de ses étapes.
- Les 67 références de tests de l’inventaire existent encore aux emplacements indiqués.
- Aucun `Deref/DerefMut` vers `Session` ni écriture `engine.session` dans l’application.
  `App::apply` compose des actions UI ou des appels documentaires au moteur.
- La CI du code relu est verte : [Rust](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40038633290)
  (matrice Linux/macOS/Windows) et [couverture](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40038633323).

## 2. Inventaire des actions : voies relues

Une coche signifie que la famille est classée et que sa voie actuelle est
identifiée et vérifiée. Elle ne transforme pas les pistes de lecture du §6 de
l’inventaire en fonctions livrées.

| Relu | Famille / lignes de l’inventaire | Voie actuelle et vérification |
| --- | --- | --- |
| [x] | 44 présentations du §4 | Registres par type et route exhaustive `UiCommand::route` ; `app/tests/routing.rs`, `commands.rs` et `engine/tests.rs` |
| [x] | Mise en forme, liens, titres et listes | Commandes typées avec cible logique gardée ; `app/tests/engine.rs`, `engine/tests.rs`, `app/tests/toolbar.rs` |
| [x] | §5.1 : saisie, suppression, sélection, styles de frappe | Commandes et adaptateur natif D17 ; `engine/tests/native.rs`, `session/edits_tests.rs`, `app/tests/edit_validation.rs` |
| [x] | §5.1 : copier, couper, coller | Lecture/copie système dans l’UI ; édition par le brouillon natif validé. Les événements egui Copy/Cut/Paste positifs sont comparés aux commandes dans `app/tests/clipboard.rs` ; les refus de frontières dans `edit_validation.rs` |
| [x] | §5.1 : frappe interactive et IME | `text.insert` littéral par défaut ; transformations `typing` et validation IME distinctes ; `engine/tests/blocks.rs`, `app/tests/blocks_engine.rs`, `app/tests/ime.rs` |
| [x] | §5.2 : descriptions, annotations, scissions/sorties | `text.select { part }`, `block.split/exit` ; `engine/tests/annotations.rs`, `engine/tests/blocks.rs`, tests UI correspondants |
| [x] | §5.2 : images, légendes, texte alternatif, suppression et dépôt | Commandes d’images partagées, ressources validées avant publication ; `engine/tests/images.rs`, `app/tests/images_engine.rs`, `app/tests/images.rs`. Le dépôt reste un adaptateur d’événement système |
| [x] | §5.3 : champs, dates, options, modèles, signature | Champs anglais typés, date fixe explicite, politique du modèle et ressources gardées ; tests `metadata.rs`, `models.rs`, `images.rs` du moteur et tests UI correspondants |
| [x] | §5.4 : recherche, remplacement, orthographe documentaire | Commandes et lectures avec références d’occurrence/faute ; `engine/tests/search.rs`, `spelling.rs`, `availability.rs` et leurs tests UI |
| [x] | §5.4–5.5 : apprentissage/oubli, profil et préférences | V5 respectée : services applicatifs, pas des commandes documentaires ni des étapes de lot. `app/tests/profile.rs`, `spelling.rs`, `spelling_engine.rs` |
| [x] | §5.6 : fichiers et récupération | Gardes et choix explicite pour le travail non enregistré ; fichiers atomiques et copies autonomes ; `engine/tests/files.rs`, `app/tests/files_engine.rs`, `recovery.rs` |
| [x] | §5.6 : aperçu et compteurs | Requêtes gardées en points Typst et Unicode ; peinture/focus/défilement UI ; `engine/tests/render.rs`, `statistics.rs`, `app/tests/preview.rs`, `toolbar.rs` |
| [x] | Lots, histoire commune et effets externes | Une publication et une annulation ; rollback sur échec, sauvegarde finale facultative, pas d’export atomique multi-fichiers promis ; `engine/tests/batch.rs`, `files.rs`, tests UI correspondants |

Les deux renommages D2 sont réalisés. `document.import_model` et `document.recover`
restent des noms d’actions UI, suivies respectivement de `document.set_model` et
des opérations `recovery.*`. L’ouverture du menu `spelling.suggestions` est UI ;
la requête du même nom retourne des données sans déplacer la sélection.
Les noms `application.set_*` / `application.forget_word` proposés au départ ne
sont pas des identifiants JSON implémentés : leurs comportements restent V5/UI.

## 3. Lectures : couverture réelle du §6

Les 14 requêtes sont bien implémentées et testées. Le relevé initial décrivait
aussi des vues plus riches et huit noms supplémentaires. Voici leur statut exact.

| Lecture ou promesse initiale | État réel | Proposition avant gel |
| --- | --- | --- |
| `commands.list` | Catalogue global, schémas d’entrée, règle statique de disponibilité, admissibilité aux lots. Pas de filtre par type ni de disponibilité évaluée avec motif dans sa réponse | Garder cette requête ; conserver `Engine::available` / `available_intent` pour l’évaluation. Examiner une lecture de disponibilité lors du transport CLI/MCP |
| `document.inspect` | Garde, sélection logique, ouverture, chemin, récupération, état modifié, undo/redo, rendu courant | Garder ; type et champs sont lisibles dans `metadata.get`, pas directement dans `inspect`. Pas de catalogue de capacités agrégé |
| `document.selection` | Aucun identifiant dédié ; la sélection gardée est dans `document.inspect.selection`. Le contexte de frappe n’y est pas exposé | Ne pas ajouter d’alias maintenant ; revoir une éventuelle lecture du contexte avant un besoin CLI |
| `metadata.fields` | Aucun identifiant dédié ; `metadata.get.fields` donne valeurs, clés, groupes, types, multiligne et obligations d’export | Considérer la vue couverte, sans alias supplémentaire |
| `document.options` | Aucun identifiant dédié ; valeurs dans `metadata.get.metadata.options`, contraintes dans `models.presentation` | Considérer la vue couverte par ces deux lectures |
| `document.structure` / `document.text` | Blocs et zones ordonnés avec identités et texte ; concaténation lisible ciblée séparément, sans objet image | Garder. Les indices de la concaténation ne sont jamais des cibles publiques ; pas de lecture générale de tous les styles dans cet arbre |
| `search.find` / remplacements | Recherche exacte du document éditable, casse optionnelle ; pas de restriction de recherche à une zone/sélection, pas de regex ni mots entiers | Garder ce périmètre déjà retenu ; les restrictions de recherche restent une extension du chantier B |
| `document_types.list` | Absent du registre ; catalogue de types actuellement accessible en Rust | Ajouter quand nécessaire pour la découverte sans UI avant livraison CLI/MCP |
| `document.source` | Absent du registre ; accesseur Rust immuable disponible | Ajouter si nécessaire pour une lecture de source sans transport graphique ; aucun accès d’exécution arbitraire à créer |
| `document.assets` | Absent du registre ; accès internes et filtrage des ressources à la sauvegarde/export disponibles | Définir une vue d’inventaire des ressources lorsque le transport en a besoin, sans exposer les ressources abandonnées de l’historique |
| `document.validate` | Absent du registre ; validations existantes appliquées aux opérations, sans rapport agrégé autonome | Préparer une opération dédiée avant de livrer la commande CLI de validation prévue en phase 4 |
| `render.compile` | Absent du registre. Compilation/installation du rendu par l’adaptateur natif ; `render.locate` et `render.selection_boxes` exigent une référence déjà installée | Définir le service de rendu au découpage en crates, puis exposer la compilation si nécessaire pour CLI/MCP. Ne pas annoncer une compilation publique déjà disponible |
| Autres requêtes livrées | `document.statistics`, `models.list`, `models.presentation`, `spelling.issues/suggestions`, `recovery.list` | Garder ; pas de fonctionnalité proposée mais non implémentée annoncée dans le catalogue |

**Décision à valider :** ces absences/limites ne sont pas reclassées silencieusement
en éléments terminés. La proposition est de clore la migration des comportements
existants avec les 63 entrées actuelles, puis d’ajouter les lectures nécessaires
aux usages CLI/MCP. Une liste de commandes gelée peut être enrichie ; elle n’a pas
besoin de contenir tous les noms envisagés avant sa première validation.

## 4. Contrats relus avant stabilisation

- [x] IDs ASCII anglais, espaces de noms et exception `courrier.model.personal`.
- [x] Séparation C/Q/UI, schémas d’entrée générés, champs inconnus refusés,
  tests d’aller-retour pour chaque variante et comparaison des fichiers générés.
- [x] `current` / `all` résolus à l’appel ; sélection explicite avec garde et
  positions locales en valeurs scalaires Unicode. `part` : `body`, `annotation`,
  `term`, `definition`. Pas de ciblage JSON par octets, indices graphiques ou pixels.
- [x] Identités opaques restaurées par historique, nouvelle révision même après undo,
  nouvelle session après réouverture ; pas de réancrage silencieux.
- [x] Defaults relus : insertion `literal`, styles `toggle`, recherche insensible à
  la casse, requête `document.text` sur `all`, statistiques sur document seul sans
  cible, export `both` et `page_images: true`, écrasement/abandon à `false`.
- [x] `metadata.set` garde/field/value/input ; `document.set_option` prend un objet
  `option { key, value }`, la puce automatique vaut `null`. Les clés de champs sont
  anglaises ; archives et paramètres internes Typst ne sont pas renommés.
- [x] Création, ouverture, fermeture et sauvegarde demandent la garde de la session
  courante, même pour créer depuis l’écran d’accueil. Le premier relevé envisageait
  des appels sans garde : ce n’est pas la signature implémentée.
- [x] Erreurs actuelles : `unavailable`, `invalid_params`, `not_found`, `io`,
  `document`, `template`, avec `params.reason` et données. `stale_revision`,
  `foreign_document`, `unsaved_changes`, etc. sont des raisons, pas de nouveaux codes.
- [x] Limites et atomicité D16 : 256 étapes, 4 Mio de paramètres, 64 Mio de ressources
  supplémentaires ; fichiers/exports/services globaux et requêtes interdits en étapes.
- [x] État de frappe et sélection sans édition = pas de nouvelle révision, pas de
  fichier modifié. Les diagnostics textuels détaillés et identités de session ne
  constituent pas des valeurs à figer entre exécutions.

## 5. Critères de sortie de 2.6

| Critère | Résultat de la relecture |
| --- | --- |
| `App::apply` compose UI ou moteur | Conforme ; routes exhaustives et tests des quatre déclencheurs |
| Aucune publication directe de session par l’UI | Conforme ; brouillons opaques et validation D17. Le chargeur interne `install_stored` est encore utilisé pour les démonstrations/captures, hors commandes utilisateur ; frontière à maintenir lors du découpage en crates |
| Blocs, zones, cibles et sélection migrés | Conforme pour les structures existantes. Zones réutilisables pour de futures cellules ; aucune structure Table/Cell ni commande de tableau n’est livrée |
| Inventaire classé et catalogue relu | Relecture réalisée ; **validation du périmètre du §3 en attente** |
| Gel des identifiants | **Non effectué**, à décider après validation de cette relecture |

Les fuites transitives d’egui/Fluent et la séparation des crates appartiennent
à la phase 3 déjà prévue. Elles ne sont pas présentées comme résolues par ce lot.
Le contrat IME est testé avec les événements egui ; le japonais a été confirmé
par l’utilisateur, mais la matrice native complète (candidats, annulation,
presse-papiers et dépôt système) reste au chantier I. Les tests Copy/Cut/Paste
ajoutés ici ne testent pas le presse-papiers réel des trois OS.

## 6. Validation de la relecture

- `cargo test --locked --all-targets --features editor-model-spike` : **339 tests
  de l’application et 203 du diagnostic passent**, dont les deux nouveaux scénarios
  du presse-papiers. Les tests contrôlent aussi les fichiers de catalogue générés.
- `cargo clippy --locked --all-targets --features editor-model-spike -- -D warnings` :
  succès ; formatage et différences vérifiés.
- Le code de production et les fichiers `docs/commands.md` / `commands.json` sont
  inchangés. Les huit captures natives du lot précédent restent la référence
  (`output/baseline/run-wfbf4qxl/`) ; aucune nouvelle capture n’est requise pour
  des tests et de la documentation seuls.
- Les liens et noms des 67 tests historiques de l’inventaire ont été vérifiés ;
  ils restent un index de départ, complété par les suites de parité du moteur.
  Quatre collisions de références T60–T63 ont été désambiguïsées : les scénarios
  IME portent désormais T66–T69, sans renommer leurs tests.

## 7. Proposition de gel à valider ensemble


1. Stabiliser les **49 identifiants de commandes et 14 de requêtes**, leurs
   paramètres existants, valeurs par défaut et invariants documentés. Pas de
   renommage ou suppression ni changement incompatible sans décision explicite.
2. Autoriser les ajouts compatibles pour les lectures manquantes, types et capacités
   futurs. Ne pas créer maintenant les alias déjà couverts par d’autres réponses.
3. Ne pas annoncer un schéma exhaustif de réponses : plusieurs lectures renvoient
   encore `serde_json::Value`. Les champs de réponse documentés sont à préserver,
   les détails de diagnostic ne deviennent pas du texte contractuel ; préciser les
   schémas de sortie nécessaires aux transports avant leur livraison.
4. Après accord seulement : clôturer 2.6, actualiser l’en-tête généré du catalogue
   et son test de référence, puis passer à la phase 3. Ce rapport et les cases
   d’implémentation vérifiées ne valent pas cet accord.
