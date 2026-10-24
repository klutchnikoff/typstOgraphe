# Phase 3 — Préparation et extraction des crates

État du **24 octobre 2026** : choix principaux confirmés, **lots 3.1 et 3.2
terminés**, frontière publique des adaptateurs préparée et **découpage 3.8 terminé**
dans le crate actuel. Le workspace n’est pas encore créé. La phase 2 conserve
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

## Frontière publique — préparation de 3.4 et 3.5

Le [contrat Rust des adaptateurs](engine-adapters.md) précise les points d’entrée
natifs, distincts du catalogue machine. `NativeEdit` et `NativeInput` sont publics,
avec état initial opaque. La publication reste transactionnelle, vérifie la garde
et passe par le validateur commun. Les conversions de curseurs et les tampons
`TextBuffer` sont des types et fonctions locaux à l’éditeur ; ils ne créent plus
d’implémentations de traits interdites une fois egui et le cœur dans deux crates
externes. `TextInput::reset_projection` conserve la première erreur d’un brouillon
IME. Le champ d’erreur de projection est privé, avec une lecture explicite.

Les méthodes natives de sélection, les lectures utiles à l’adaptateur et les
échanges du worker de rendu sont publics ; les instantanés de session et
`install_stored` restent internes. `Engine::from_stored` valide des données
possédées et crée une nouvelle identité, sans historique. La démonstration
l’utilise sans accès privilégié à la session. Les parcours d’ouverture,
enregistrement et récupération continuent de passer par les commandes.

Les fixtures graphiques construisent des données détachées et utilisent ce
constructeur. Leurs modifications réelles utilisent les commandes ou les
brouillons natifs, leurs contrôles d’historique lisent des compteurs immuables.
Les faux rendus périmés sont construits côté adaptateur sans manipuler la révision
de session. Le scénario de ressources volontairement invalides est déplacé dans
les tests internes du moteur. `engine/test_support.rs` et `Session::changed`
sont supprimés ; aucun accès mutable de test à la session n’est rendu public.

## Lot 3.8 — Découpage des modules

Le premier commit ne fait que déplacer le code et ajuster les visibilités entre
modules enfants ; les modifications d’API suivent dans des commits séparés.

| Façade | Responsabilités extraites | Plus gros module de production du dossier |
| --- | --- | --- |
| `document.rs` : 194 lignes | `projection`, `serialization`, `parsing`, `input`, `options` ; conservation des modules `structure` et `edits` | 418 lignes |
| `editor.rs` : 67 lignes | `buffer`, `fonts`, `layout`, `images`, `descriptions`, `geometry`, `decorations` ; tests dans `editor/tests.rs` | 365 lignes |

Les chemins explicites des modules conservent la réutilisation par le diagnostic
isolé. L’interface de rédaction et le format Typst ne changent pas. Les seuils de
800 lignes sont vérifiés pour `app`, `engine`, `document` et `editor`, désormais
aussi dans la CI pour ces quatre dossiers. Les limites globales restent en phase 7.

### Vérifications de cette préparation

- **356 tests application et 221 tests diagnostic** passent avec
  `cargo test --locked --all-targets --features editor-model-spike`.
  Log local : `/tmp/typstographe-phase3-adapters-final-tests.log`.
- Clippy sur les mêmes cibles avec `-D warnings`, formatage et différences : succès.
  Log : `/tmp/typstographe-phase3-adapters-final-clippy.log`.
- Les 11 tests des scripts d’architecture passent ; les quatre dossiers respectent
  le seuil. Le catalogue généré conserve ses 49 commandes et 19 requêtes.
- Tests ajoutés : conservation de direction et affinité des curseurs, erreur
  persistante à travers une remise à zéro IME, constructeur validant les données
  et créant des identités indépendantes. Les contrôles des brouillons périmés,
  étrangers, fermés et de leur historique sont conservés.
- Les **huit captures natives correspondent exactement** aux références :
  `output/baseline/run-5su3hjya/`. Aucun changement visuel dans ce lot.

## Suite

1. Créer le workspace puis déplacer les trois crates et leurs ressources. Déclarer
   les réexportations publiques de bibliothèque en gardant `session` privé ;
   raccorder les helpers de synchronisation des workers et les échantillons
   d’erreurs nécessaires aux tests inter-crates, sans exposer de session mutable.
2. Adapter les chemins du diagnostic, les inclusions de ressources et les outils
   de contrôle aux nouveaux emplacements.
3. Appliquer le repli anglais dans le lot i18n, adapter la CI et bloquer toute
   dépendance graphique ou Fluent dans l’arbre du cœur. Vérifier compilation,
   lancement, ouverture d’un `.typsto` existant et captures.

Les dépendances transitives restent à contrôler après l’extraction physique. Les
préparations 3.1, 3.2 et 3.8 ne présentent pas le workspace et les crates comme réalisés.
