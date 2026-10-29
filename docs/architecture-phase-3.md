# Phase 3 — Préparation et extraction des crates

État du **24 octobre 2026** : **phase 3 réalisée et vérifiée localement**.
Le workspace contient les trois crates prévues ; les lots 3.1 à 3.8 sont livrés.
Le catalogue conserve ses **49 commandes et 19 requêtes**, ses contrats d’entrée
et ses références générées. La CI vérifiera les trois systèmes après la poussée.

Références : [feuille technique](https://github.com/klutchnikoff/typstOgraphe/blob/7b0d6fd3e6ae25e5704ed43ce2f4efefa9c2fa0b/ROADMAP-ARCHITECTURE.md#phase-3--espace-de-travail-taille-m),
[bilan de phase 2](architecture-phase-2-review.md), [contrat des lectures](engine-queries.md).

## Découpage confirmé

| Crate | Contenu | Dépendances internes |
| --- | --- | --- |
| `crates/core` : `typstographe-core` | Modèle documentaire, types, modèles, Engine, historique, fichiers, ressources, orthographe, récupération, compilation et export Typst | Aucune autre crate du projet |
| `crates/i18n` : `typstographe-i18n` | Fluent, catalogues français/anglais et mise en phrases des erreurs structurées du cœur | `core` |
| `crates/app` : `typstographe` | egui, éditeur et adaptateur natif, menus, commandes graphiques, dialogues, préférences et profil | `core`, `i18n` |
| Future `crates/cli` : `typstographe-cli` | CLI puis MCP, phases 4 et 5 | `core`, `i18n` |

Trois crates sont livrées dans cette phase ; aucune CLI vide n’est anticipée.
Le cœur conserve compilation, stockage et ressources ensemble : aucun découpage
supplémentaire n’est introduit à ce stade. Il ne dépend ni d’egui/eframe, winit,
rfd, muda, ni de Fluent/i18n-embed, même transitivement.

Le **profil global et sa persistance restent dans l’application**, avec les
préférences. La création transmet au moteur des valeurs et ressources explicites ;
le cœur ignore d’où elles viennent et ne consulte aucun profil personnel.
La mention initiale de `profile` parmi les modules à extraire dans le cœur est
corrigée par D22.

La langue suit le système, avec **repli anglais**, appliqué dans le commit distinct
`4df98fe`. Un système sans préférence française ou anglaise utilise donc désormais
l’anglais. La langue documentaire demeure indépendante. Les captures et les tests
graphiques choisissent explicitement le français pour rester reproductibles.

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
`crates/i18n/src/lib.rs` et `crates/i18n/src/errors.rs`, avec un chargeur explicite possible pour les futurs
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
- À la clôture de 3.2, le repli reste français ; son passage à l’anglais est ensuite
  appliqué dans le commit distinct de 3.4b. La langue documentaire est inchangée.

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

## Livraison 3.3 à 3.7 — Workspace et extraction

Le `Cargo.toml` racine est un workspace virtuel avec `resolver = "3"`, versions
centralisées dans `workspace.dependencies` et métadonnées/lints partagés. Le membre
par défaut est l’application : **`cargo run --locked` fonctionne depuis la racine**.
Le verrou ne change aucune version de dépendance externe ; il décrit désormais
les trois paquets locaux et leurs dépendances propres. Les **824 ressources
déplacées sont identiques octet pour octet** à leurs versions précédentes,
catalogues, polices et packages Typst compris.

- **Cœur** : `crates/core/src/lib.rs` expose les modules documentaires et garde
  `session` et `identity` privés. Compilation, rendu, E/S et ressources restent
  ensemble. Le profil et sa persistance restent dans l’application. Les modèles,
  exemples, dictionnaires et polices se trouvent sous `crates/core` ; leurs
  inclusions et dossiers RustEmbed sont résolus depuis ce crate.
- **Traduction** : `crates/i18n/src/lib.rs` porte chargeur, macro et présentations
  des erreurs. Il dépend du cœur ; le cœur ne le connaît pas. Les catalogues
  restent uniques sous `crates/i18n/i18n/`. `initialize` fixe une fois la langue
  demandée ; `loader_for` permet un chargeur explicite. L’application ne déclare
  plus de dépendance directe à Fluent/i18n-embed. Son petit `i18n.toml` pointe sur
  le catalogue partagé pour la vérification des appels de macro à la compilation.
- **Application** : `crates/app/src/lib.rs` réunit l’adaptateur egui, ses menus,
  préférences et profil ; `main.rs` conserve le lancement et les options natives.
  Le diagnostic réutilise cette bibliothèque et le cœur plutôt que d’inclure leurs
  sources. Il ne réexécute plus leurs suites unitaires.
- **Tests et ressources** : `fixtures/` est partagé à la racine. Les helpers
  bornés `Recovery::make_due/wait` et `Checker::wait` sont disponibles pour les
  tests via la fonctionnalité `test-support` du cœur, activée seulement en
  dépendance de développement par l’application. La fonctionnalité homonyme
  d’i18n fixe le français des tests graphiques. Aucun accès à la session n’est
  exposé ; les échantillons d’erreurs restent dans les tests du cœur.
- **CI** : tests et Clippy couvrent le workspace, la couverture continue avec
  `--workspace`, les chemins des contrôles de taille sont adaptés. Le script
  `scripts/check_core_dependencies.py` bloque toute dépendance normale vers
  egui/eframe, winit, rfd, muda, Fluent ou i18n-embed, sur les trois systèmes.
  Le contrôle WASM Linux installe sa cible puis lance `cargo check`, en étape
  informative et non bloquante.

### Vérifications finales de l’extraction

| Vérification locale | Résultat |
| --- | --- |
| `cargo test --locked --workspace --all-targets --features editor-model-spike` | **212 tests cœur, 137 application, 7 traduction, 6 diagnostic réussis** |
| `cargo clippy --locked --workspace --all-targets --features editor-model-spike -- -D warnings` | Succès |
| `cargo fmt --all --check` et `git diff --check` | Succès |
| `cargo check --locked -p typstographe-core` | Succès, cœur compilé indépendamment |
| `python3 scripts/check_core_dependencies.py` | Aucune dépendance interdite dans l’arbre normal du cœur |
| Tests des scripts et contrôles de taille | **14 tests réussis**, seuils respectés |
| `cargo run --locked -- --commands-catalogue-json` | Les 68 entrées correspondent à `docs/commands.json` |
| Huit captures natives | **Identiques aux références** : `output/baseline/run-0h915vs_/` |
| Réouverture d’un `.typsto` antérieur, format 9 | Vérifiée visuellement : japonais/chinois présents dans l’éditeur et l’aperçu ; `output/workspace-existing-cjk.png` |

Les 356 tests répartis entre les trois crates correspondent aux scénarios déjà
présents ; les 6 tests du diagnostic s’y ajoutent, sans les anciennes copies de
suites. Les catalogues `docs/commands.md` et `docs/commands.json` et le format
`.typsto` restent inchangés. Le repli anglais est le changement visible volontaire.

Logs locaux : `/tmp/typstographe-workspace-final-tests.log`,
`/tmp/typstographe-workspace-final-clippy.log`,
`/tmp/typstographe-workspace-final-scripts.log`,
`/tmp/typstographe-core-alone-check.log` et
`/tmp/typstographe-workspace-captures.log`.
Le Rust Homebrew local ne dispose pas de la cible WASM : le contrôle local s’arrête
sur `E0463` (bibliothèque standard absente), sans conclure à la compatibilité du
cœur. La nouvelle étape CI installera cette cible pour mesurer les obstacles réels.


## Diagnostic WASM — 24 octobre 2026

Le contrôle CI reste **informatif**, avec `continue-on-error: true`. L’échec actuel
est identifié dans `getrandom 0.4.3`, sans affecter les cibles natives. Pour la cible
`wasm32-unknown-unknown`, `cargo tree -p typstographe-core --target
wasm32-unknown-unknown -i getrandom@0.4.3 -e normal` montre :

```text
typstographe-core → atomic-write-file 0.3.1 → rand 0.10.3 → getrandom 0.4.3
```

`atomic-write-file` génère des noms temporaires aléatoires pour publier les archives.
Le graphe natif inclut aussi `getrandom` via `tempfile`, mais cette seconde branche
n’est pas active pour la cible WASM testée. `getrandom` refuse par défaut cette cible
qui ne précise ni système hôte ni disponibilité de JavaScript. L’option `wasm_js`
choisit `Crypto.getRandomValues` via `wasm-bindgen` pour un environnement Web ; elle
n’est pas ajoutée au cœur pour masquer cette erreur. Voir la [documentation getrandom
0.4.3](https://docs.rs/getrandom/0.4.3/getrandom/#webassembly-support).

Le cœur est indépendant de l’interface, mais contient encore stockage natif, fichiers
temporaires, verrous et worker de récupération. La [cible Rust générique WASM](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html)
ne fournit pas `std::fs` fonctionnel ni `std::thread::spawn`. Lever la première erreur
ne prouverait donc pas la portabilité de ces services. Une construction complète
avec `wasm_js` n’a pas été validée ; le Rust Homebrew local n’a toujours pas cette cible.

**Décision :** conserver le contrôle informatif et les dépendances natives actuelles.
Une future cible Web demandera d’isoler stockage/verrous/récupération derrière une
frontière adaptée, en conservant édition, métadonnées et composition en mémoire.
Ce diagnostic ne lance pas un chantier Web et ne remet pas en cause les choix natifs.
Trace de l’échec CI : `output/phase-5-files-ci-linux.log`.

**Mise à jour du 29 octobre 2026 :** l’échec `getrandom` n’apparaît plus ; la seule
erreur restante était `yrs::undo::Options::default()`, absent en WebAssembly parce
qu’il utilise l’horloge système. Les historiques de participants utilisent désormais
une horloge constante (le groupement est explicite). **Le contrôle devient bloquant**
sur Linux pour empêcher toute régression silencieuse. Il prouve seulement la
compilation : fichiers, verrous, récupération et dates (`chrono::Local::now`) restent
natifs, et les aléas de yrs (`fastrand`, sans `getrandom` adapté au Web) devront être
vérifiés à l’exécution avant tout client Web collaboratif, car des identifiants de
client identiques casseraient la convergence.

## Suite

La prochaine phase est la **phase 4 : CLI**. Ses choix principaux et son premier
lot seront validés ensemble avant implémentation. Les critères de séparation des
crates seront conservés pendant cette extension. La CLI et MCP restent absents
de ce lot ; aucune nouvelle opération documentaire n’est ajoutée au catalogue.
