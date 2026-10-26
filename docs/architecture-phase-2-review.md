# Relecture de sortie de phase 2

**Phase 2 terminée le 24 octobre 2026**, après les cinq lectures et les deux
réponses complémentaires demandées par l’utilisateur avant le gel. Les 68
identifiants et contrats d’entrée sont stabilisés ; les ajouts compatibles restent
possibles et un changement incompatible exige une décision explicite.

La première relecture portait sur `39245c4edbafb9067231c84f11257056b05aabb5`, puis
sur les tests de presse-papiers. Les absences signalées ont ensuite été complétées
dans les lots `88d68d7` et `01f4328`, avec les vérifications ci-dessous.

Références : [feuille technique](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md),
[inventaire relu](commands-inventory.md), [catalogue](commands.md),
[schémas d’entrée](commands.json), [historique des lots](architecture-phase-2.md).
Les tableaux de migration restent un relevé de départ ; ils ne constituent pas
une seconde définition des paramètres à côté du registre Rust.

## 1. État vérifié

- 49 commandes typées et 19 requêtes, soit **68 entrées** générées depuis les enums.
- 31 variantes de `UiCommand`, 41 définitions communes, deux actions de Courrier
  et un modèle fourni : 44 présentations pour Courrier, 41 pour Compte-rendu.
  Ce dernier type reste sans modèle fourni et sa création graphique indisponible.
- 38 commandes peuvent être étapes de `document.batch`. Fichiers, récupération,
  export, historique, requêtes et lots imbriqués sont exclus de ses étapes.
- Les 67 références de tests de l’inventaire existent encore aux emplacements indiqués.
- Aucun `Deref/DerefMut` vers `Session` ni écriture `engine.session` dans l’application.
  `App::apply` compose des actions UI ou des appels documentaires au moteur.
- La CI du code de la première relecture (`39245c4`) est verte : [Rust](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40038633290)
  (matrice Linux/macOS/Windows) et [couverture](https://github.com/klutchnikoff/typstOgraphe/actions/runs/40038633323).
  Ces liens ne prouvent pas le résultat CI de la clôture : ses tests locaux sont
  détaillés au §6, la nouvelle CI sera déclenchée par la poussée finale.

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

Les 19 requêtes sont implémentées et testées. Le [contrat des lectures](engine-queries.md)
précise les cinq ajouts et les deux réponses enrichies. Les noms du relevé initial
qui désignent la même vue ne sont pas dupliqués.

| Lecture ou promesse initiale | État à la clôture | Limite ou extension explicite |
| --- | --- | --- |
| `commands.list` | Catalogue et schémas, règles statiques, disponibilité contextuelle avec motif ou `requires_parameters` | Paramètres réels revalidés à l’exécution ; pas de filtre paramétrable par type ni de promesse d’acceptation d’un appel incomplet |
| `document.inspect` | État de session, sélection, rendu, type, capacités et style de frappe | Les capacités sont les IDs des étapes documentaires du type ; le style de frappe n’est pas un résumé d’une sélection aux styles mélangés |
| `document.selection` | Sélection dans `document.inspect.selection`, style de frappe dans `typing_style` | Pas d’alias dédié |
| `metadata.fields` | Descripteurs complets et valeurs dans `metadata.get.fields` | Pas d’alias dédié |
| `document.options` | Valeurs dans `metadata.get.metadata.options`, contraintes dans `models.presentation` | Pas d’alias dédié |
| `document.structure` / `document.text` | Blocs, zones, identités et texte ciblé, sans objet image dans la concaténation lisible | Lecture exhaustive des styles internes reportée ; les indices de concaténation ne sont jamais des cibles publiques |
| `search.find` / remplacements | Recherche exacte documentaire, casse optionnelle | Restriction à une zone/sélection, mots entiers et regex hors périmètre retenu, extensions du chantier B |
| `document_types.list` | Types, modèles, création effective et capacités, même sans document ouvert | Compte-rendu sans modèle fourni, donc création indisponible |
| `document.source` | Source Typst du corps publié avec garde | Aucune exécution de code fourni ; modèle et `main.typ` ne sont pas ce corps |
| `document.assets` | Inventaire des ressources retenues et dépendances, chemins internes et tailles | Politique commune à sauvegarde/export, prudente si le modèle invalide empêche de prouver qu’une ressource est inutile ; aucun parcours de l’historique |
| `document.validate` | Rapport gardé : erreurs, avertissements, validité du brouillon et préparation à l’export | Les contrôles réutilisés peuvent s’arrêter à leur première erreur par catégorie ; pas de promesse d’exhaustivité des erreurs d’un programme invalide |
| `render.compile` | Compilation synchrone sans UI, référence de rendu et diagnostics, sans rasterisation ni export | Cache courant réutilisé ; un transport pourra appeler la lecture sur un worker |
| Autres requêtes | Compteurs, modèles, présentation, orthographe, récupération et géométrie du rendu déjà livrés | Services globaux V5 toujours distincts des mutations documentaires |

**Périmètre validé et livré :** cinq requêtes supplémentaires et deux réponses
enrichies avant la clôture. Aucune de ces cinq fonctions n’est reportée à CLI/MCP.
La représentation détaillée des styles, les recherches supplémentaires et les
schémas exhaustifs de sortie restent des extensions identifiées, sans être
annoncées comme terminées. Les schémas nécessaires aux transports seront précisés
avant leur livraison.

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
| Inventaire classé et catalogue relu | Conforme ; cinq lectures ajoutées et deux réponses enrichies selon le périmètre validé |
| Gel des identifiants et contrats d’entrée | Effectué après les compléments : 49 commandes et 19 requêtes, références générées et tests synchronisés |

Les fuites transitives d’egui/Fluent et la séparation des crates appartiennent
à la phase 3 déjà prévue. Elles ne sont pas présentées comme résolues par ce lot.
Le contrat IME est testé avec les événements egui ; le japonais a été confirmé
par l’utilisateur, mais la matrice native complète (candidats, annulation,
presse-papiers et dépôt système) reste au chantier I. Les tests Copy/Cut/Paste
ajoutés ici ne testent pas le presse-papiers réel des trois OS.

## 6. Validation de clôture

- `cargo test --locked --all-targets --features editor-model-spike` : **347 tests
  application et 211 tests diagnostic passent**. Huit scénarios ajoutés dans
  `engine/tests/discovery.rs` et `validation.rs` couvrent les lectures complémentaires.
  Logs locaux : `/tmp/typstographe-phase2-completion-tests.log`.
- `cargo clippy --locked --all-targets --features editor-model-spike -- -D warnings` :
  succès ; formatage et différences vérifiés. Log :
  `/tmp/typstographe-phase2-completion-clippy.log`.
- Les 68 entrées ont des schémas d’entrée stricts, un test d’aller-retour et un
  catalogue généré comparé aux références. La génération statique est séparée de
  `commands.list`, dont la réponse dépend du contexte. Après actualisation de
  l’en-tête de clôture, les tests du catalogue sont exécutés à nouveau.
- Les huit captures natives correspondent exactement aux références :
  `output/baseline/run-8091w1lx/`. Le worker graphique partage la compilation
  Typst de la requête sans UI et conserve ses protections contre les réponses périmées.
- Les liens et noms des 67 tests historiques de l’inventaire ont été vérifiés ;
  ils restent un index de départ, complété par les suites du moteur. Les références
  IME T66–T69 distinguent les anciennes collisions T60–T63.

## 7. Périmètre stabilisé

1. Les **49 identifiants de commandes et 19 de requêtes**, leurs paramètres,
   défauts et invariants documentés sont stabilisés. Pas de renommage, suppression
   ou modification incompatible sans décision explicite.
2. Les ajouts compatibles restent possibles pour les types et capacités futurs.
   Aucune requête supplémentaire n’est créée pour une vue déjà couverte.
3. Les champs de réponse documentés sont à préserver. Les diagnostics comportent
   codes et sévérités ; leur détail textuel et les identités temporaires ne sont
   pas des valeurs constantes entre exécutions. Plusieurs réponses sont encore
   assemblées en `serde_json::Value` : leurs schémas exhaustifs seront précisés
   avant les transports, sans les prétendre générés aujourd’hui.
4. La phase 3 peut commencer : découpage en crates et retrait des dépendances
   graphiques/localisées du cœur. CLI/MCP, tableaux et matrice système complète
   restent dans leurs phases ou chantiers respectifs.
