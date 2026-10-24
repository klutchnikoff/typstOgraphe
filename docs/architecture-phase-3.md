# Phase 3 — Préparation et extraction des crates

État du **24 octobre 2026** : choix principaux confirmés, **lots 3.1 et 3.2
terminés** dans le crate actuel. Le workspace n’est pas encore créé. La phase 2 conserve
ses 49 commandes et 19 requêtes, ses contrats d’entrée et ses références générées.

Références : [feuille technique](../ROADMAP-ARCHITECTURE.md#phase-3--espace-de-travail-taille-m),
[bilan de phase 2](architecture-phase-2-review.md), [contrat des lectures](engine-queries.md).

## Découpage confirmé

| Crate | Contenu | Dépendances internes |
| --- | --- | --- |
| `crates/core` : `typstographe-core` | Modèle documentaire, types, modèles, Engine, historique, fichiers, ressources, orthographe, récupération, compilation et export Typst | Aucune autre crate du projet |
| `crates/i18n` : `typstographe-i18n` | Fluent, catalogues français/anglais et mise en phrases des erreurs structurées du cœur | `core` |
| `crates/app` : `typstographe` | egui, éditeur et adaptateur natif, menus, commandes graphiques, dialogues, préférences et profil | `core`, `i18n` |
| Future `crates/cli` : `typstographe-cli` | CLI puis MCP, phases 4 et 5 | `core`, `i18n` |

Trois crates seront livrées dans cette phase ; aucune CLI vide n’est anticipée.
Le cœur conserve compilation, stockage et ressources ensemble : aucun découpage
supplémentaire n’est introduit à ce stade. Il ne dépendra ni d’egui/eframe, winit,
rfd, muda, ni de Fluent/i18n-embed, même transitivement.

Le **profil global et sa persistance restent dans l’application**, avec les
préférences. La création transmet au moteur des valeurs et ressources explicites ;
le cœur ignore d’où elles viennent et ne consulte aucun profil personnel.
La mention initiale de `profile` parmi les modules à extraire dans le cœur est
corrigée par D22.

La langue suit le système, avec **repli anglais** confirmé. Ce changement reste
à appliquer dans un commit distinct du lot i18n ; le repli actuel est encore
français. La langue documentaire demeure indépendante.

## Lot 3.1 — Frontières graphiques

Les modifications précèdent les déplacements de fichiers :

- `RenderWorker::spawn` et `Recovery::start` reçoivent un rappel
  `Arc<dyn Fn() + Send + Sync>` ; seule l’application y capture son contexte egui.
  Les tâches publient leur réponse avant de réveiller l’appelant. La coalescence
  des aperçus, les gardes et la file de récupération sont conservées.
- Vignettes et pages utilisent `assets::RgbaImage` : dimensions en pixels et
  octets RGBA à alpha prémultiplié, rangées sans padding. `PageImage` en reste un
  alias. Les conversions en `egui::ColorImage` sont effectuées dans l’application,
  avant le chargement des textures ; décodage et rasterisation sont inchangés.
- Les présentations Courrier et modèle personnel résident dans
  `commands/document_types.rs`. Les données de modèles ne contiennent plus de
  `Presentation`, de `UiCommand`, de raccourci ni de référence à `commands`.
  `Model::id()` conserve l’identité publique ; l’application associe les libellés
  et les actions à ces identités. Les listes continuent à dériver du catalogue.
- Les tests de correspondance catalogue/modèles/présentations sont côté
  `commands`; les tests du catalogue de modèles ne dépendent plus de l’interface.
  L’interface des champs et la séparation `CommandSpec`/`Presentation` avaient
  déjà été réalisées lors des phases précédentes. Les préférences restent côté
  application ; aucun déplacement physique de `profile` n’est encore nécessaire.

Le test du worker Typst utilise un appelant sans contexte graphique et vérifie
succès, erreur, garde et publication avant réveil. Le test de récupération
vérifie également le rappel après un checkpoint. Les tests de vignettes conservent
les proportions PNG/JPEG/SVG et contrôlent dimensions, longueur et alpha des pixels.

## Vérifications du lot 3.1

- `cargo test --locked --all-targets --features editor-model-spike` : **348 tests
  application et 212 tests diagnostic passent**. Log local :
  `/tmp/typstographe-phase3-boundaries-tests.log`.
- Clippy sur les mêmes cibles avec `-D warnings`, formatage et différences : succès.
  Log Clippy : `/tmp/typstographe-phase3-boundaries-clippy.log`.
- Test ciblé des vignettes relancé après adaptation aux recommandations de Clippy.
- Catalogues de commandes inchangés et validés par leurs tests de référence.
- Seuils de taille des modules `app` et `engine` respectés.
- Les **huit captures natives correspondent exactement** aux références :
  `output/baseline/run-b6eiw_3h/`. Aucun changement visuel volontaire dans ce lot.

## Lot 3.2 — Erreurs et libellés sans Fluent

Le module `errors.rs` définit les familles `SourceError`, `StorageError`,
`PackageError`, `TemplateError`, `ExportError`, `RecoveryError` et `SpellingError`,
réunies dans `CoreError`, ainsi que les erreurs d’image déjà typées. Les variantes
portent leurs paramètres typés ; une archive invalide conserve sa cause imbriquée.
Les diagnostics externes de Typst, des décodeurs et du système sont transportés
comme détails techniques, sans inventer de traduction de leurs messages.

Les opérations de source, packages, stockage, export, rendu, récupération et
orthographe renvoient ces données plutôt que des phrases Fluent. Les workers
transportent les mêmes erreurs que les appels synchrones. Les libellés des types,
formats de papier, langues et annotations sont des clés constantes ; leur
traduction est effectuée par l’adaptateur. Les deux anciennes voies d’image
(localisée/typée) sont réunies en une API typée `read` / `decode`.

La mise en phrases de `CoreError` et de `CommandError` se trouve désormais dans
`i18n.rs` et `i18n/errors.rs`, avec un chargeur explicite possible pour les futurs
transports. `App::command_error_text` y délègue ; il ne contient plus de catalogue
parallèle d’erreurs. Le profil reste applicatif et adapte ses erreurs d’image à
cette frontière, conformément à D22.

Les six codes de `CommandError`, `params.reason` et le champ textuel `detail`
restent en place. Quand une cause typée est disponible, `params.cause` s’y ajoute.
Les diagnostics de validation/compilation et les entrées de `recovery.list`
peuvent également porter `cause`, sans supprimer leurs champs existants.
La sérialisation des erreurs typées décrit `domain`, puis la variante `kind` et
ses paramètres ; elle ne contient pas de message traduit. Les détails textuels
restent informatifs et ne sont pas des valeurs à figer entre langues ou systèmes.
Aucun contrat d’entrée ni identifiant du catalogue n’est modifié.

Les déclarations d’erreurs produisent aussi leurs contrats de messages : clés et
noms d’arguments, sans importer Fluent. Les tests vérifient les **44 messages
distincts** et les libellés directement dans chacun des catalogues français et
anglais, sans repli. Ils contrôlent sérialisation, paramètres Unicode, causes
imbriquées, erreurs de lots et séparation entre texte localisé et données machine.

### Vérifications du lot 3.2

- **353 tests application et 217 tests diagnostic** passent :
  `cargo test --locked --all-targets --features editor-model-spike`, log local
  `/tmp/typstographe-phase3-errors-final-tests.log`.
- Clippy sur toutes les cibles avec `-D warnings`, formatage et différences : succès.
  Log : `/tmp/typstographe-phase3-errors-clippy.log`.
- Les tests de référence du catalogue passent ; `docs/commands.md` et
  `docs/commands.json` sont inchangés (49 commandes et 19 requêtes).
- L’audit des modules destinés au cœur ne trouve plus d’appel `tr!`, de référence
  à `crate::i18n` ni d’import de Fluent/i18n-embed. Le crate unique dépend encore
  de ces bibliothèques pour l’interface : le contrôle des dépendances transitives
  appartiendra à 3.6, après extraction.
- Les huit captures correspondent exactement aux références :
  `output/baseline/run-jgrspqm2/`.
- Le repli reste français dans ce lot ; son passage à l’anglais reste prévu dans
  un commit distinct lors de l’extraction i18n. La langue documentaire est inchangée.

## Suite

1. Préciser la surface publique nécessaire aux adaptateurs : brouillons natifs
   opaques et validation commune, sans accès à `Session`. Adapter les fixtures
   graphiques à cette frontière, sans exposer un accès mutable réservé aux tests.
2. Découper les gros modules de document et d’éditeur par responsabilité, puis
   créer le workspace et déplacer les trois crates avec leurs ressources.
3. Appliquer le repli anglais dans le lot i18n, adapter CI/scripts et bloquer toute
   dépendance graphique ou Fluent dans l’arbre du cœur. Vérifier compilation,
   lancement, ouverture d’un `.typsto` existant et captures.

Les dépendances transitives du crate unique, les restrictions de visibilité Rust
et les inclusions de ressources restent à traiter. Les lots 3.1 et 3.2 ne présentent
pas le workspace et l’extraction des crates comme réalisés.
