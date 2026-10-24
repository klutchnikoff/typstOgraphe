# Feuille de route technique — architecture, commandes, CLI et MCP

Document de travail établi le **21 octobre 2026**, relevé actualisé sur le commit
`f0c81a0` (143 tests sur macOS avant la phase 0).
Il complète la [feuille de route générale](ROADMAP.md) et suit sa méthode : **chaque
phase commence par les décisions de la section 3, validées ensemble avant
implémentation**. Il constitue le détail du chantier J de `ROADMAP.md`.
Les décisions D1 à D14 et les précisions D15 à D20 ci-dessous sont validées ;
les choix supplémentaires restent à discuter au début de chaque phase.

Les estimations sont des tailles relatives (S, M, L), pas un calendrier. Chaque
tâche numérotée vise une unité de travail vérifiable. Les déplacements restent
isolés dans des commits ; plusieurs déplacements cohérents peuvent être regroupés
dans une même demande de fusion (PR), avec les tests au vert.

## 1. Objectif et périmètre

1. **Découper proprement `src/app.rs`** (7 223 lignes) puis le dépôt en crates, sans
   changer le comportement.
2. **Rendre les commandes strictes** : toute action de l'utilisateur est une commande
   à identifiant stable et paramètres typés, exécutée par un moteur sans interface.
3. **Exposer ce moteur** par une ligne de commande (CLI) puis par un serveur MCP
   (Model Context Protocol), à partir du même registre.

**Ce qui n'est pas dans ce document** : collaboration, compilation WASM, nouvelles
fonctions d'édition (tableaux, notes), correction grammaticale. La section 9 liste
des points repérés en chemin et volontairement laissés de côté.

**Règle de toutes les phases** : les déplacements et réorganisations de code ne
changent pas le comportement, et les tests existants (143 attributs `#[test]` relevés)
restent verts à chaque commit.

**Assouplissement (décidé le 21 octobre 2026)** : un changement de comportement
*volontaire* est permis à deux conditions. Il est décidé avec vous, et consigné dans la
section 3 ou dans la PR. Il vit dans un commit séparé, sans déplacement de code, avec les
tests et les captures de référence régénérés. Il n'y a aucun utilisateur actuel : aucune
compatibilité de format ou d'interface n'est à préserver (`ROADMAP.md` le prévoit déjà).
Ce qui reste interdit, c'est de mêler un changement de comportement à un déplacement :
la comparaison des captures ne serait plus un signal fiable.

## 2. État des lieux (relevé)

### 2.1 `src/app.rs`

| Zone | Lignes | Observation |
| --- | --- | --- |
| Constantes, énumérations, `struct App` | 1–124 | `App` compte environ 70 champs qui mêlent état du document, des boîtes de dialogue, de la recherche, de l'orthographe, de la récupération, de l'aperçu et des captures |
| `impl App` | 125–3677 | 93 méthodes, dont `text_shortcuts` (306 lignes), `dialogs` (278), `editor_ui` (194), `search_ui` (179) |
| `impl eframe::App` | 3678–3716 | boucle principale |
| Tests | 3717–7223 | environ 3 500 lignes, une cinquantaine de tests, assistants partagés (`frame`, `search_draw`, `recovery_click`, `seed_recovery`, `icon_rect`) |

`App::apply` (lignes 739–884) répartit les commandes mais **mélange** deux familles :
les opérations sur le document (`Bold`, `Block`, `Model`, `InsertPostscript`, `Undo`…)
et l'état d'interface (`Palette`, `Preferences`, `Find` qui ouvre la barre, `Information`).

### 2.2 Commandes

- `commands.rs` : énumération `Command` de 31 variantes (44 entrées concrètes pour le Courrier) ; `Definition` porte un
  `id` textuel, une clé de libellé Fluent, la portée et les raccourcis (types egui) ;
  `Registry` par type de document ; `Keymap` pour les raccourcis.
- Identifiants actuels (`Definition.id`) : déjà de la forme `domaine.action`
  (`text.bold`, `block.heading_1`, `edit.replace_all`, `spelling.next`, `document.save_as`,
  `application.quit`, `commands.palette`), avec deux exceptions en tirets
  (`letter.insert-postscript`, `letter.insert-attachment`) et `courrier.model.personal`.
  Les raccourcis personnalisés enregistrés (`ShortcutOverrides`) en dépendent.
- Une seule entrée côté interface : `App::apply`, appelée par les menus, la palette,
  la barre d'outils et les raccourcis.
- **Actions qui contournent le registre** : sortie d'annotation et navigation par
  Tab (`leave_annotation`, `annotation_shortcuts`), actions sur image (`ImageAction`
  dans `editor.rs`), comportements de `text_shortcuts`, édition des champs de
  métadonnées et des options, choix de signature, correction orthographique d'un mot,
  recherche (texte de la requête), ouverture et enregistrement sous un chemin donné.
- Les commandes sont des variantes **sans paramètres** (sauf `Block` et `Model`) et
  ne sont exposées nulle part hors de l'interface.

### 2.3 Couplage avec egui

Modules **sans** egui : `annotations`, `dates`, `packages`, `profile`, `session`,
`spelling`, `statistics`, `storage`, `i18n`, `document_types/{models,courrier,compte_rendu}`,
et `document` (une seule mention, dans un commentaire).

Fuites à traiter avant tout découpage en crates :

| Module | Fuite | Traitement prévu |
| --- | --- | --- |
| `rendering.rs:490` | `RenderWorker::spawn(egui::Context)` : sert à demander un rafraîchissement | fonction de réveil `Arc<dyn Fn() + Send + Sync>` |
| `recovery.rs:218` | `start(.., ctx: egui::Context)` : même usage | idem |
| `assets.rs:107` | `thumbnail` renvoie `egui::ColorImage` | renvoyer largeur, hauteur et octets RGBA ; conversion dans l'application |
| `preferences.rs` | `EditorFont::font_id`, `Theme::theme` utilisent des types egui | le module entier passe côté application |
| `document_types/mod.rs` | `editor_group_ui`, `fields_ui` (interface des champs) | côté application ; les descripteurs `TextField` (données) restent |
| `commands.rs` | `Key`, `KeyboardShortcut`, `Modifiers` dans `Definition` | séparer la définition (cœur) de la présentation (application) |
| `session.rs` | aucune : les conversions avec `CCursorRange` sont dans `editor.rs` | rien à faire |

### 2.4 Autres faits utiles

- `tr!` est utilisée environ 90 fois dans des modules candidats au cœur
  (`document` 21, `storage` 27, `document_types` 9, `recovery` 8, `rendering` 6,
  `packages` 5, `spelling` 4, `assets` 3, `preferences` 3, `annotations` 2, `profile` 2).
- Chemins relatifs à la racine du crate : sept `include_bytes!`/`include_str!` vers
  `tests/fixtures`, `crates/core/examples/`, `crates/core/templates/`, `assets/` et `crates/i18n/i18n/`, plus deux
  `#[folder = ...]` de `RustEmbed` (`crates/core/templates/packages/`, `crates/core/templates/capabilities/`).
  Ils casseront au passage en espace de travail.
- `main.rs` analyse les arguments à la main : sept options `--capture-*`, `--render-demo`,
  `--help`. Pas de `clap`.
- CI : `fmt`, `test`, `clippy -D warnings` sur trois systèmes ; couverture sur Linux avec
  `cargo +nightly llvm-cov --workspace` ; `#![cfg_attr(coverage_nightly, feature(coverage_attribute))]`
  est déclaré dans `main.rs`.
- Pas de `clippy.toml` ni de `rustfmt.toml`.
- Les tests d'interface appellent des méthodes privées d'`App` : le découpage en
  fichiers impose de rendre ces méthodes `pub(super)`.

## 3. Décisions validées et précisions avant implémentation

| N° | Décision | Recommandation | Phase |
| --- | --- | --- | --- |
| D1 | Découpage en crates | **Validée le 21 octobre 2026, complétée par D7.** Quatre crates : `typstographe-core` (modèle, moteur, E/S, rendu Typst), `typstographe-i18n` (catalogues Fluent et mise en phrases des codes ; dépend du cœur), `typstographe` (application egui) et `typstographe-cli` (CLI + MCP). Pas plus fin pour l'instant : `Session` dépend de `Metadata`, `StoredDocument` et `Dependencies`, donc un cœur « modèle seul » ne tient pas | 3 |
| D2 | Schéma des identifiants de commande | **Validée le 21 octobre 2026.** Garder le schéma existant : `domaine.action` en minuscules ASCII, tirets bas dans l'action (`text.bold`, `block.heading_1`, `edit.replace_all`, `document.save_as`). Corriger seulement `letter.insert-postscript` et `letter.insert-attachment` (tirets). La convention en tirets concerne les identifiants Typst, pas ceux de l'application | 2 |
| D3 | Représentation des paramètres | **Validée le 21 octobre 2026.** Énumération Rust typée avec données, `serde` pour le JSON et `schemars` pour générer les schémas d'entrée | 2 |
| D4 | Lectures et écritures | **Validée le 21 octobre 2026.** Deux familles distinctes : `Command` (modifie la session) et `Query` (lecture seule : texte, structure, erreurs d'orthographe, métadonnées, catalogue) | 2 |
| D5 | Commandes d'interface seule | **Validée le 21 octobre 2026.** Une énumération `UiCommand` (palette, préférences, ouverture de dialogues, focus) non exposée par la CLI ni MCP | 2 |
| D6 | Erreurs | **Validée le 21 octobre 2026, reprise le même jour.** Type `CommandError` avec code stable et paramètres structurés ; énumérations `thiserror` par module converties vers lui. La phrase lisible n'est pas produite par le cœur : voir D7. Le cœur n'a qu'un `Display` à usage de développeur (journaux, tests) | 2 |
| D7 | Internationalisation | **Validée le 21 octobre 2026** (option C, choisie après examen de trois options). Le cœur ne dépend pas de Fluent : il renvoie des codes, des paramètres et des clés Fluent comme données (environ 90 appels de `tr!` à convertir). Un crate `typstographe-i18n`, **en aval** du cœur (il en connaît les codes, le cœur l'ignore), porte le chargeur, la macro `tr!`, un catalogue unique français et anglais (aujourd'hui `crates/i18n/i18n/`, 310 clés) et la mise en phrases des erreurs. L'application et la CLI en dépendent. L'application et la CLI suivent la langue du système avec **repli en anglais** (l'application avait le français pour repli) ; MCP répond en anglais par défaut. Un autre front-end (web, terminal) réutilise le même catalogue | 3 |
| D8 | Réveil de l'interface | **Validée le 21 octobre 2026.** Rappel `Arc<dyn Fn() + Send + Sync>` à la place de `egui::Context` | 3 |
| D9 | Emplacement des ressources | **Validée le 21 octobre 2026.** `crates/core/templates/`, `crates/core/examples/` et `crates/core/assets/dictionaries/` passent dans `crates/core` ; `crates/app/assets/icons/` dans l'application ; `crates/i18n/i18n/` dans `crates/i18n` (D7) ; `fixtures/` est partagé à la racine de l'espace de travail, utilisé par les tests des crates | 3 |
| D10 | CLI | **Validée le 21 octobre 2026.** Binaire séparé `typstographe-cli`, `clap` en mode dérivation, sortie `--json` au contrat stable | 4 |
| D11 | MCP | **Validée le 21 octobre 2026.** SDK Rust officiel `rmcp` après un prototype d'une journée qui vérifie : liste d'outils dynamique dérivée du registre, taille du binaire, temps de compilation, version courante de l'API (versions majeures successives constatées sur docs.rs). `tokio` uniquement dans la CLI. Repli : JSON-RPC minimal sur stdio. Transport stdio d'abord | 5 |
| D12 | Sécurité MCP par défaut | **Validée le 21 octobre 2026.** Aucun accès aux fichiers hors racines déclarées (`--read-root`, `--write-root` séparées), enregistrement explicite, mode `--read-only`, aucune action réseau | 5 |
| D13 | Seuils de lint | **Validée le 21 octobre 2026.** `clippy::too_many_lines` à 120 (`clippy.toml`) ; fichier de production maximal 800 lignes, contrôlé par un script de CI (tests exclus), en avertissement à la fin de la phase 1 et bloquant en phase 7 | 1, 7 |
| D14 | Canal de contrôle de l'application en cours d'exécution | **Validée le 21 octobre 2026.** Reporté ; à rouvrir après la phase 5 seulement si un besoin de test visuel automatisé ou d'essais assistés apparaît | 6 |
| D15 | Historique | **Validée après relecture.** Une mutation documentaire validée constitue une transaction ; sélection, navigation, recherche et enregistrement n’ajoutent pas d’étape d’annulation. La frappe conserve ses groupes naturels ; un lot d’édition se défait en une étape | 2 |
| D16 | Atomicité des lots | **Validée après relecture.** D’abord un document, modifié sur une session temporaire puis enregistré une seule fois par écriture atomique. Les étapes ne peuvent ni enregistrer ni exporter, ni modifier le profil ou le dictionnaire global. Export et autres effets externes sont des opérations séparées ; une atomicité multi-fichiers demanderait une autre décision | 2, 4, 5 |
| D17 | Saisie native | **Validée après relecture.** L’adaptateur egui peut préparer une édition native, mais validation, normalisation, révision et historique utilisent la même voie du cœur que les commandes. Une erreur ne publie aucune mutation partielle ; tests de parité avec `text.insert` | 2 |
| D18 | Modèle d’édition | **Validée après relecture.** Prototype borné de tableaux et vérification de la composition IME après le rangement initial, avant le gel des paramètres publics du moteur en phase 2. **Choix validé le 22 octobre 2026 après le prototype : blocs et cellules explicites en mémoire**, avec des zones de texte réutilisant la projection actuelle et la saisie egui. Cette représentation est reconstruite depuis le Typst et sérialisée en Typst, qui reste la référence documentaire | 1, 2 |
| D19 | Captures comparables | **Validée après relecture.** Figer en mode capture la langue, le thème, le curseur, les animations et le temps de rendu affiché. Fixtures et dates fixes, attente des résultats asynchrones. Les empreintes PNG ne servent de contrôle qu’après répétition réussie sur le même environnement | 0 |
| D20 | Granularité des changements | **Validée après relecture.** Un déplacement par commit, plusieurs commits cohérents possibles par PR. Les limites portent sur le code de production ; le découpage de `document.rs` et d’`editor.rs` est planifié avant leur application globale | 1, 3, 7 |
| D21 | Périmètre de clôture et gel du catalogue | **Validée et mise en œuvre le 24 octobre 2026.** Ajouter les cinq lectures manquantes et enrichir disponibilité/inspection avant le gel : 49 commandes et 19 requêtes. Stabiliser leurs IDs et contrats d’entrée ; tout changement incompatible exige une décision explicite, les ajouts compatibles restent possibles. Les réponses documentées et les extensions restantes sont décrites dans [le bilan de sortie](docs/architecture-phase-2-review.md) et [le contrat des lectures](docs/engine-queries.md) | 2, 3, 4 |
| D22 | Précisions de phase 3 | **Validées le 24 octobre 2026.** Gestion et persistance du profil personnel avec les préférences dans l’application ; le cœur ne reçoit que des valeurs et ressources explicites. Confirmer le repli anglais de D7, dans un commit distinct lors de l’extraction i18n. Trois crates livrées en phase 3 (`core`, `i18n`, application), CLI/MCP dans les phases 4/5, sans crate vide anticipée | 3, 4, 5 |
| D23 | Contrat CLI | **Validé le 24 octobre 2026.** Une invocation indépendante de l’UI ; commandes de confort et entrées `query`/`run` dérivées du registre. Sauvegarde explicite avec `--save` ou `--output`, destination existante refusée par défaut ; export vers un répertoire absent ou vide. Scripts JSON versionnés, étapes `{id, params}`, chemins relatifs au script, un document et une sauvegarde atomique finale, export séparé. `check` distingue validité, préparation à l’export et orthographe facultative. JSON version 1, langue système/repli anglais/`--lang`, aucun profil GUI implicite | 4 |
| D24 | Accès concurrents aux fichiers locaux | **Validé le 24 octobre 2026.** Un seul détenteur du verrou d’écriture, conservé pendant la session ; lectures de la dernière version enregistrée autorisées. Compagnon stable verrouillé par le système, libération à la fermeture ou au plantage, empreinte SHA-256 vérifiée avant sauvegarde. Ouverture GUI en lecture seule proposée en cas de verrou ; copie vers un autre fichier possible. La politique des sessions d’agents MCP sera définie en phase 5 | 4, 5 |


## 4. Vue d'ensemble

```
Phase 0  garde-fous ───────────────────────────────┐
Phase 1  découper app.rs (un seul crate)           │ déplacements purs
Phase 2  commandes strictes + moteur (Engine)      │ cœur de la refonte
Phase 3  espace de travail : core / i18n / app     │ déplacements + changements volontaires isolés
Phase 4  CLI                                       │ première exposition
Phase 5  MCP                                       │ deuxième exposition
Phase 6  canal de contrôle (facultatif, reporté)   │
Phase 7  consolidation, documentation, ROADMAP.md ─┘
```

Dépendances : 1 avant 2 (on range avant de réformer) ; 2 avant 3 (le moteur est le
contenu du crate `core`) ; 3 avant 4 ; 4 avant 5 (MCP réutilise les commandes, les
schémas et les politiques de chemins de la CLI). Les phases 1 et 2 peuvent être
interrompues entre deux PR sans laisser le dépôt dans un état intermédiaire.

Autres chantiers : la livraison des tableaux et notes (étape 5 de `ROADMAP.md`)
vient après la phase 7. Le prototype de décision du modèle d’édition est avancé
avant le gel du moteur (D18), sans livrer les tableaux à ce stade.
L'harmonisation de la présentation des blocs de l'éditeur (chantier B) vient après la
phase 2, car les tâches 2.4.2 (structure et annotations) et 2.4.3 (images) réécrivent
les mêmes comportements.

---

## Phase 0 — Garde-fous (taille S)

**But** : pouvoir prouver qu'une étape ne change pas le comportement.

- [x] **0.1** Relever la base : `cargo test --locked`, `clippy -D warnings`, couverture.
  Consigner les commandes, le commit, la plateforme, le nombre de tests et la couverture
  dans `docs/architecture-phase-0.md`. La mesure Linux de la CI existante peut servir
  de référence, avec son exécution et son commit exacts ; ne pas la présenter comme
  une mesure locale. Relever et corriger séparément toute CI déjà en échec.
- [x] **0.2** Créer `scripts/capture-baseline.sh` qui produit, dans `output/baseline/`
  (déjà ignoré par git), les captures d'écran de référence avec les options existantes :
  `--capture`, `--capture-new-document`, `--capture-search <requête>`,
  `--capture-spelling`, `--capture-profile`, `--capture-profile-letter`,
  `--capture-recovery <dossier>`, plus un courrier représentatif ouvert depuis un
  fichier autonome. Appliquer D19 avant de comparer les empreintes SHA-256 des PNG.
  Conserver un manifeste des scénarios et de l’environnement, les références et les
  nouvelles captures séparément ; ne jamais écraser les références en mode comparaison.
  Faire deux passages consécutifs pour vérifier la reproductibilité. La comparaison
  est locale : les polices, la densité d’écran et le rendu varient entre environnements.
- [x] **0.3** Étiqueter le commit de départ (`pre-refactor-architecture`).
- [x] **0.4** Décisions D1 à D20 de la section 3 : validées.

**Critère de sortie** : base chiffrée consignée, script de captures utilisable.

**État : terminé.** Voir le [bilan de phase 0](docs/architecture-phase-0.md) pour
les résultats, leur provenance et la procédure de comparaison. La phase 1 est
détaillée ci-dessous.

---

## Phase 1 — Découper `app.rs` (taille L)

**But** : aucun fichier de production d’`app/` au-dessus de 800 lignes et aucune
fonction d’`app/` au-dessus de 120, tests rangés par thème. L’application globale
des seuils attend les tâches complémentaires de 3.8 et la phase 7. **Un seul crate**, pour limiter le risque.

**Méthode** : déplacements purs d'abord, réorganisation ensuite. Un commit ne déplace
**ou** ne modifie, jamais les deux. Les méthodes appelées entre fichiers deviennent
`pub(super)` ; les champs d'`App` aussi, le temps du regroupement de l’état en 1.4.

### 1.1 Sortir les tests (zéro risque pour la production)

- [x] Créer `crates/app/src/app/` et convertir `app.rs` en `crates/app/src/app/mod.rs`.
- [x] Déplacer les lignes 3717–7223 vers `crates/app/src/app/tests/` : `mod.rs` (déclarations),
  `support.rs` (assistants partagés), puis un fichier par thème :
  `files.rs` (enregistrer, rouvrir, fermer), `recovery.rs`, `search.rs`, `spelling.rs`,
  `images.rs`, `annotations.rs`, `preview.rs` (synchronisation, sélection, échelle),
  `toolbar.rs`, `commands.rs` (raccourcis, historique, modèles), `information.rs`,
  `profile.rs`, `text_input.rs` (Tab, Entrée, descriptions, citations).
- [x] Au passage, factoriser l'amorce répétée dans les tests
  (contexte et application de démonstration, entrée de frame standard) dans
  `support.rs`. Garder explicites les thèmes et dimensions propres à chaque scénario.
  Ne rien changer aux assertions.
- [x] Mettre à jour les chemins des `include_bytes!` des fixtures.

### 1.2 Fichiers par responsabilité (déplacements)

Les numéros de ligne datent du 21 octobre ; les ajuster au moment du déplacement.

| Fichier cible | Contenu actuel d'`app.rs` |
| --- | --- |
| `app/mod.rs` | constantes, énumérations, `struct App`, `ui_frame`, `capture`, `impl eframe::App` |
| `app/setup.rs` | `new`, `with_context` et configuration du contexte ; déplacés séparément pour garder `mod.rs` sous 400 lignes |
| `app/state.rs` | structures d’état de 1.4 |
| `app/files.rs` | `stored_document`, `is_dirty`, `save_path`, `save_document`, `open_path`, `install_document`, `request_action`, `close_document`, `perform_action`, `create_document`, `handle_file_command`, `file_dialogs` |
| `app/recovery.rs` | `start_recovery`, `clear_recovery`, `checkpoint_recovery`, `recovered_filename`, `recover_document`, `recovery_dialog` (312–455) |
| `app/commands.rs` | `range`, `changed`, `remember`, `apply`, `apply_history`, `edit`, `shortcuts`, `track_history_focus`, `menu_statuses`, `dispatch_action` |
| `app/text_input.rs` | `annotation_shortcuts`, `focus_annotation_target`, `leave_annotation`, `text_shortcuts` |
| `app/models.rs` | `install_model`, `import_model`, `template_selector` |
| `app/images.rs` | `choose_image`, `import_image`, `check_asset_budget`, `remove_image` |
| `app/signature.rs` | `set_document_signature`, `use_profile_signature`, `choose_signature`, `signature_preview`, `document_signature_ui` |
| `app/links.rs` | `open_link` |
| `app/preview.rs` | `request`, `rendering`, `preview_ui`, `sync_is_current`, `navigate_backward` |
| `app/chrome.rs` | `toolbar`, `status_ui` |
| `app/information.rs` | `information_ui`, `general_information_ui`, `bullet_marker_ui` |
| `app/search.rs` | `refresh_search`, `capture_search`, `close_search`, `search_focus`, `select_search_result`, `replace_search`, `search_ui` |
| `app/spelling.rs` | `refresh_spelling`, `selected_spelling_issue`, `valid_spelling_target`, `correct_spelling`, `open_spelling_target`, `spelling_context_menu`, `capture_spelling` |
| `app/editor_view.rs` | `editor_ui`, `columns_ui` |
| `app/preferences.rs` | `preferences_ui`, `appearance_preferences_ui`, `spelling_preferences_ui`, `profile_preferences_ui`, `persist_preferences`, `capture_profile` |
| `app/dialogs.rs` | `dialogs`, `welcome_ui`, `modal_open` |

- [x] Un commit de déplacement par responsabilité extraite (les structures de
  `state.rs` relèvent de 1.4), regroupables dans une même PR, dans l'ordre croissant
  du risque : `links`,
  `images`, `signature`, `models`, `search`, `spelling`, `preferences`, `information`,
  `chrome`, `recovery`, `files`, `preview`, `editor_view`, `commands`, `text_input`,
  `dialogs`, `mod`.
- [x] À chaque PR : tests, `clippy`, puis `scripts/capture-baseline.sh --compare`.

### 1.3 Couper les fonctions trop longues

- [x] `text_shortcuts` (306 lignes) en une fonction par comportement : Entrée, Tab,
  deux-points des descriptions, citations, listes, texte littéral annulable.
- [x] `dialogs` (278 lignes) en une fonction par fenêtre : nouveau document, informations
  d'image, lien, palette, export.
- [x] `editor_ui` (194), `search_ui` (179) : isoler les sous-parties (barre de recherche,
  rangée de remplacement, zone de texte, retour d'éditeur).
- [x] Aucun changement de signature publique.

### 1.4 Regrouper l'état (réorganisation)

Séparer les environ 70 champs d'`App` en structures nommées, sans changer la logique :

| Structure | Champs concernés |
| --- | --- |
| `DocumentState` | `document_open`, `session`, `selection`, `force_selection`, `statistics`, `history_field`, `presentation` |
| `FileState` | `file_command`, `pending_action`, `file_error`, `allow_close`, `window_title`, `export_open`, `export_path` |
| `SearchState` | `search`, `search_open`, `replace_open`, `search_focus_pending` |
| `SpellingState` | `spelling`, `spelling_target`, `spelling_menu_pending`, `spelling_menu_position`, `capture_spelling` |
| `RecoveryState` | `recovery`, `recovery_entries`, `recovery_open`, `recovery_error`, `recovered_name` |
| `PreviewState` | `worker`, `pages`, `submitted`, `rendering_time`, `rendering_error`, `rendered_revision`, `sync`, `forward_pending`, `editor_scroll_pending` |
| `DialogState` | `palette_*`, `new_document_*`, `link_*`, `image_info`, `image_alt`, `information_*`, `preferences_page`, `preferences_open`, `profile_error`, `message` |
| `CaptureState` | `capture`, `screenshot_requested`, `started` |

- [x] Un commit de réorganisation par structure, regroupables par PR, dans l'ordre : `CaptureState`, `RecoveryState`,
  `SpellingState`, `SearchState`, `PreviewState`, `FileState`, `DialogState`,
  `DocumentState`.
- [x] Retirer les `pub(super)` devenus inutiles.

### 1.5 Seuils (D13)

- [x] Ajouter `clippy.toml` avec `too-many-lines-threshold = 120`.
- [x] Ajouter `scripts/check-file-length.sh` : échoue si un fichier de production de
  `src/` dépasse 800 lignes (fichiers de tests exclus). En CI, en avertissement
  (non bloquant) jusqu'à la phase 7 ; `app/` est déjà contrôlé de façon bloquante.
  Le lint des fonctions est activé sur la production d’`app/` ; les scénarios de tests
  sont exemptés pour conserver leurs assertions. L’activation globale attend la phase 7.

### 1.6 Prototype de décision du modèle d’édition (D18)

- [x] Après le rangement initial, faire un prototype borné de tableau simple avec le
  tampon actuel : cellules, navigation, sélection, insertion/suppression, annulation.
- [x] Vérifier touches mortes et composition IME ; noter les limites observées.
  Six essais du diagnostic valident les opérations et la saisie accentuée/IME via
  les événements egui. L’utilisateur confirme les accents et touches mortes sur
  macOS. Une vraie méthode IME du système reste à essayer lors des vérifications
  multilingues du chantier I ; ce chemin n’est pas présenté comme testé.
- [x] Décider ensemble si le moteur garde ce modèle ou accueille des blocs et séquences
  en ligne en mémoire. **Blocs et cellules explicites validés le 22 octobre 2026.**
  Le source Typst reste la référence du contenu et du stockage.
- [x] Reporter la décision et ses conséquences sur les cibles et paramètres des commandes
  dans les entrées de l’inventaire 2.1 ci-dessous, avant leur gel en fin de phase 2.

### 1.7 Critères de sortie

- Aucun fichier de production d'`app/` au-dessus de 800 lignes ; `app/mod.rs` sous 400.
- Nombre de tests inchangé par plateforme (143 sur macOS, 142 sous Linux et Windows),
  tous verts ; mêmes captures qu'à la phase 0 sur le même environnement.
- `cargo clippy --locked --all-targets -- -D warnings` sans nouvelle exception injustifiée ;
  `clippy::too_many_lines` est activé dans `app/` avec le seuil de 120 lignes.

**État : terminé.** Rangement et six essais du diagnostic validés ; accents et
touches mortes confirmés par l’utilisateur sur macOS. L’IME est vérifiée via les
événements egui ; l’essai d’une méthode IME réelle reste au chantier I. Voir le
[bilan de phase 1](docs/architecture-phase-1.md).

---

## Phase 2 — Commandes strictes et moteur (taille L)

**But** : une action de l'utilisateur est une commande à identifiant et paramètres,
exécutable **sans egui** par un `Engine`. C'est la phase qui rend la CLI et MCP possibles.

### 2.1 Inventaire (taille S)

- [x] Produire `docs/commands-inventory.md` : un tableau exhaustif de toutes les actions
  actuelles. Colonnes : chemin actuel (fonction), identifiant cible, paramètres,
  famille (commande, requête, interface seule), nécessite un dialogue (oui/non),
  utilisable sans interface (oui/non), testée par (nom du test).
- [x] Y classer au minimum : les 31 variantes de `Command`, les contournements de la
  section 2.2, les opérations de fichier (nouveau, ouvrir, enregistrer sous, exporter
  PDF, exporter Typst), la saisie de texte, la sélection, le choix de métadonnées,
  d'options (papier, langue, marqueur de puce), l'import d'image, l'ajout de lien.
- [x] Faire valider les orientations de l’inventaire avant d’écrire du code (V1–V6,
  22 octobre 2026) ; préciser les signatures pendant la migration avant le gel.

**État : orientations V1–V6 validées le 22 octobre 2026 ; implémentation autorisée.** Voir
[docs/commands-inventory.md](docs/commands-inventory.md), avec les 44 entrées du
registre, les actions hors registre, les cibles et les tests existants.
La forme exacte des identifiants, les signatures et les jonctions entre types de
paragraphes seront précisées et testées pendant la migration, avant le gel.
L’isolation IME entre frames et sa publication unique sont testées via les
événements egui ; une vraie méthode IME du système reste à essayer au chantier I.
V5 confirme que les réglages globaux restent des actions d’application, hors du
premier périmètre CLI/MCP. V6 valide la création avec valeurs initiales explicites,
les dates fixes et les formats d’export séparés, avec conservation de l’export
complet par défaut. Le travail commence par la représentation structurée (2.1 bis).

**Entrées issues du choix de 1.6 (22 octobre 2026) :**

- Prévoir des blocs et cellules explicites en mémoire. Typst reste la source de
  référence ; cette structure ne devient pas un second format documentaire.
- Une cible textuelle doit pouvoir désigner une zone de texte et une position
  Unicode dans cette zone. Un indice global ne doit pas être la seule cible possible.
- Une sélection continue peut traverser plusieurs zones ; son remplacement doit
  avoir une sémantique structurelle validée avant toute publication (D17).
- Les opérations sur les cellules ou blocs sont distinctes de la saisie de texte ;
  les mutations structurelles et leur sélection se restaurent ensemble par l’historique.
- La normalisation du corps ne doit pas supprimer des lignes ou du contenu d’une
  cellule. Préserver la composition IME dans la zone éditée.
- Les orientations de l’inventaire sont validées ; les contrats précis restent à tester. La durée
  de vie des identifiants est validée en V1 ; leur forme exacte et les signatures
  Rust/JSON restent à décider en 2.1. Le prototype n’ajoute aucune commande publique
  de tableau à ce stade.

### 2.1 bis Représentation structurée en mémoire (D18)

Après validation de l’inventaire et de ses cibles, avant le gel des signatures :

- [x] Définir les blocs, leurs zones de texte et leurs identifiants ; réutiliser la
  projection actuelle pour l’édition du texte dans chaque zone.
- [x] Migrer les structures déjà prises en charge, avec aller-retour Typst et
  ressources identiques ; aucune structure nouvelle livrée à l’utilisateur ici.
- [x] Adapter les positions, la sélection continue, les cartes de source, la recherche
  et la synchronisation de l’aperçu ; préciser les règles entre zones avant de coder.
- [x] Faire porter par les transactions du cœur les changements de structure, de
  contenu et de sélection, y compris l’adaptateur de saisie native (D15, D17).
- [x] Tester frontières, normalisation locale, IME, historique et Unicode. Préparer
  le modèle pour les cellules sans avancer la livraison des tableaux prévue à l’étape 5.

**Implémentation relue le 24 octobre :** tous les blocs existants sont représentés et alimentent le
sérialiseur via un adaptateur vers la projection. Les identités, les positions
locales Unicode et les gardes session/révision sont testées ; l’historique conserve
les sélections logiques. Les frontières de rôles sont protégées ; les éditions
refusées ne publient aucune mutation partielle. La saisie native travaille sur un
brouillon et rejoint la validation/normalisation/publication commune de `Session`,
comme les commandes et remplacements. L’UI conserve seulement une projection et
un handle natif opaque ; le moteur possède l’instantané d’origine et valide sa
publication. `Deref/DerefMut` et les écritures directes dans la session sont supprimés.
La préédition IME reste dans
un brouillon entre frames et sa validation publie une seule étape d’historique ;
les annulations et événements tardifs ne changent pas la session publiée.
Le tampon graphique reste plat. Les cartes de source Typst alimentent désormais
les requêtes gardées `render.locate` et `render.selection_boxes` du moteur ;
l’interface garde le zoom, la peinture, le focus et les défilements. Les compteurs
Unicode sont lus via `document.statistics`, avec un cache détenu par le moteur.
La recherche utilise des cibles logiques gardées, converties en indices seulement
pour sa peinture. Les règles des descriptions, listes, notes, légendes/attributions
et les transformations de frappe sont exécutées par le moteur.
Voir [le suivi de phase 2](docs/architecture-phase-2.md) pour les vérifications
et les limites de ces premiers lots. Aucune commande de tableau n’est ajoutée.

### 2.2 Types du registre (taille M)

Dans `crates/core/src/engine/` (futur crate `core`) :

- [x] `CommandId` (chaîne validée au démarrage : unicité, forme `domaine.action` avec
  tirets bas ; D2). Renommer `letter.insert-postscript` et `letter.insert-attachment`
  en `letter.insert_postscript` et `letter.insert_attachment`.
- [x] `Command` : énumération avec données (`Block { kind }`, `SetLink { url }`,
  `InsertImage { path, alt }`, `SelectText { range | occurrence }`, `InsertText { text }`,
  `SetMetadata { field, value }`, `SetOption { option, value }`, `ReplaceAll { with }`…).
  Les variantes actuelles sans paramètres restent telles quelles.
  **Relu :** 49 commandes typées implémentées, dont les lots atomiques ;
  leurs identifiants et contrats d’entrée sont stabilisés à la clôture de phase 2. Les espaces de noms
  hiérarchiques existants (`courrier.model.personal`) sont conservés, sans autre renommage que D2.
- [x] `Query` : lectures seules (`document.inspect`, `document.text`, `spelling.issues`,
  `search.find`, `metadata.get`, `commands.list`, `models.list`).
  **Clôture :** dix-neuf requêtes implémentées, dont les cinq lectures complémentaires
  `document_types.list`, `document.source`, `document.assets`, `document.validate`
  et `render.compile`. `commands.list` expose la disponibilité contextuelle et
  `document.inspect` le type, les capacités et le style de frappe. Voir
  [le contrat des lectures](docs/engine-queries.md), [rendu et compteurs](docs/read-queries.md)
  et [le bilan de sortie](docs/architecture-phase-2-review.md).
- [x] `UiCommand` : palette, préférences, ouverture des dialogues, focus. Reste dans l'application.
- [x] `CommandSpec` (cœur, sans egui) : identifiant, description **en anglais** (pour les
  agents), famille, schéma des paramètres (`schemars`), règle de disponibilité
  et admissibilité explicite comme étape de lot (`batch_step`).
- [x] `Presentation` (application) : clé de libellé Fluent, raccourcis, position de menu
  (`menu_position`), icône. Séparer ce qui est actuellement dans `Definition`.
- [x] `Command::from_call(id, json) -> Result<Command, CommandError>` et
  `Command::to_call()` ; test d'aller-retour pour chaque variante.
- [x] `CommandError { code, params }` avec des codes stables en snake_case
  (`unavailable`, `invalid_params`, `not_found`, `io`, `document`, `template`) ; le texte
  affiché vient du crate `typstographe-i18n` (D6, D7).
  Les types et leur adaptation Fluent dans l’application sont en place ; le
  déplacement du convertisseur dans le crate i18n reste en phase 3.

### 2.3 Moteur (taille L)

- [x] `Engine` autour de `Session` : `run(&mut self, Command) -> Result<Outcome, CommandError>`
  et `query(&self, Query) -> Result<Value, CommandError>`.
- [x] `Outcome` : `changed`, garde et sélection résultantes, valeur de retour
  éventuelle, effets à déclencher (rendu à relancer, sélection à rendre visible).
- [x] **Une mutation documentaire validée = une transaction d’historique** (D15).
  Sélection, navigation, recherche, ouverture et enregistrement ne créent pas d’entrée
  d’annulation. Préserver le regroupement de la frappe et l’annulation d’un lot en une étape.
  Les commandes migrées et les groupes de frappe sont vérifiés. `document.batch`
  prépare ses étapes sur une session isolée et publie une seule transaction ; un lot
  sans changement conserve le rétablissement.
- [x] **Lots atomiques d’un document** (D16) : commandes admises déclarées au registre,
  aucune écriture dans les étapes, sauvegarde finale facultative et explicite.
  Un échec, y compris de sauvegarde, conserve la session publiée. Limites : 256 étapes,
  4 Mio de paramètres JSON, 64 Mio de ressources supplémentaires. Le contrat des
  gardes, les résultats et les exemples sont dans [docs/atomic-batches.md](docs/atomic-batches.md).
- [x] **Session encapsulée** : suppression de `Deref/DerefMut`. Les lectures d’affichage
  utilisent des accesseurs immuables ; le moteur ne fournit aucun `&Session` ou
  `&mut Session`. La saisie native reçoit un `NativeEdit` opaque et rejoint la voie
  commune de commit. Les préparations par mutation/restauration sont limitées aux
  fixtures compilées sous `cfg(test)` ; les captures passent par les voies validées.
- [x] La sélection (`Caret`, `Selection`, déjà indépendantes d'egui dans `session.rs`)
  appartient au moteur : les agents et la CLI en ont besoin pour `text.insert`
  et `text.select`.
- [x] `available(&Engine) -> Availability { Enabled, Disabled(reason) }` remplace la
  logique documentaire de `menu_statuses` (`app/commands.rs`). Les intentions
  avant dialogue partagent les contrôles des commandes complètes. Menus, barre,
  palette, boutons de recherche et dispatch utilisent `App::command_enabled` ;
  seuls les états de fenêtres/préférences restent graphiques.
  Voir [le contrat d’aiguillage](docs/command-routing.md).
- [x] La saisie native d’egui peut préparer une édition via son `TextBuffer` (D17).
  Son commit passe par la même validation, normalisation, mise à jour de révision et
  transaction du cœur que les commandes ; aucune publication partielle en cas d’erreur.
  Documenter cet adaptateur et tester sa parité avec `text.insert`.

### 2.4 Migration famille par famille (taille L)

Pour chaque famille : déplacer la logique d'`App::apply` vers le moteur, faire appeler
le moteur par l'interface, **ajouter un test qui exécute la commande par identifiant et
JSON** et compare avec le chemin d'interface.

- [x] 2.4.1 Mise en forme : `Bold`, `Italic`, `Clear`, `Block`, `Link`, `RemoveLink`.
- [x] 2.4.2 Structure : `InsertPostscript`, `InsertAttachment`, annotations (entrée, sortie,
  navigation), listes et descriptions.
  P.-S./P.J., listes, descriptions et conversion en citation sont raccordés ; la
  navigation légende/attribution et Entrée dans ces structures sont migrées.
  Termes/définitions, scissions et sorties de listes/notes, ainsi que préfixes et
  délimiteurs de frappe sont migrés. `text.insert` distingue explicitement les modes
  littéral (défaut) et `typing` ; l’IME reste littérale.
- [x] 2.4.3 Images : insérer, remplacer, supprimer, texte alternatif, légende ;
  signature de document. Cibles de bloc gardées, imports préparés avant publication,
  budget et historique communs ; copie du profil explicite. L’initialisation des
  valeurs à la création reste dans 2.4.8, conformément à V6.
- [x] 2.4.4 Recherche : requête, suivant, précédent, remplacer, tout remplacer.
  `search.find` fournit des occurrences logiques gardées et leur texte attendu ;
  les clics utilisent `text.select`. Navigation et remplacements passent par les
  appels typés/JSON, avec parité egui testée. Tout remplacer reste une transaction.
  Seuls les rectangles de surbrillance utilisent encore le tampon graphique.
- [x] 2.4.5 Orthographe : corriger un mot, ignorer, réinitialiser les exceptions
  documentaires ; apprendre/oublier un mot et activer la correction restent des
  actions globales de l’application (V5), hors du premier périmètre CLI/MCP.
  `spelling.next/previous/correct/ignore/reset` et les lectures
  `spelling.issues/suggestions` sont dans le moteur, avec cibles gardées et
  validation du mot attendu et de sa langue. Le worker et le moteur réutilisent
  l’analyse et les suggestions locales ; le dictionnaire personnel est transmis
  comme configuration de lecture, jamais modifié par une commande documentaire.
- [x] 2.4.6 Métadonnées et options : champs, papier, orientation, langue, marqueur de puce.
  `metadata.set` et `document.set_option` valident les gardes, champs du type actif
  et contraintes du modèle. `metadata.get` décrit les champs anglais validés ;
  `models.presentation` expose les règles de page/puces. La saisie groupe par champ
  typé, sans identifiant egui ; les dates restent fixes et l’horloge reste dans l’UI.
- [x] 2.4.7 Modèles : changer, importer.
  `document.set_model` accepte une source ou un fichier UTF-8 gardé ; le préréglage
  `courrier.model.personal` appelle la même opération. Dépendances contrôlées,
  compilation du contrat et du document, budget des ressources puis publication
  atomique. `models.list` suit le catalogue du type. Dialogues, libellés et focus
  restent dans App ; annuler/rétablir restaure aussi la présentation du modèle.
- [x] 2.4.8 Fichiers, en gardant les identifiants existants : `document.new { type, model }`,
  `document.open { path }`, `document.save { path? }`, `document.save_as { path }`,
  `document.export { format: "pdf" | "typst" | "both", directory, page_images? }`
  (V6 : dossier neuf/vide, export complet avec PNG par défaut).
  Les dialogues de choix de fichier restent dans l'application, qui appelle le moteur
  avec le chemin choisi. Création sans valeurs implicites du profil ni de l’horloge,
  gardes et décision explicite pour remplacer un brouillon ; enregistrement atomique
  sans transaction d’historique. `document.close`, `recovery.list/restore/delete` sont
  aussi raccordés ; la récupération utilise des références opaques du service configuré.
  `save_as` refuse l’écrasement par défaut (`overwrite: true` après confirmation UI) ;
  une première sauvegarde sur un autre chemin ne remplace pas non plus une cible existante.
  L’export graphique exécute en arrière-plan le même instantané validé que le moteur.
- [x] 2.4.9 Historique : `edit.undo`, `edit.redo`.

### 2.5 Tests du registre

Dans le style du test existant sur les fichiers Fluent :

- [x] chaque variante de `Command` a une `CommandSpec` ; identifiants uniques et valides ;
- [x] chaque commande exposée a une description anglaise non vide et un schéma ;
- [x] chaque clé de libellé de `Presentation` existe en français et en anglais ;
- [x] aller-retour `Command → appel → Command` ;
- [x] **test d'aiguillage** : chaque déclencheur d'interface (menu, palette, barre d'outils,
  raccourci) passe par `Engine::run` ou `UiCommand`. Parcours exhaustif des registres,
  correspondance des routes au catalogue, clics/Entrée/raccourci personnalisé,
  garde de sélection et transaction annulable, refus des actions indisponibles ;
- [x] **catalogue contrôlé** : `docs/commands.md` et `docs/commands.json` sont générés depuis le registre et comparés
  dans la CI (échec si le fichier est périmé).
  Les 68 entrées documentaires sont relues et stabilisées ; les actions propres
  à l’interface restent hors catalogue. Le suivi détaillé est dans
  [docs/architecture-phase-2.md](docs/architecture-phase-2.md).

### 2.6 Critères de sortie

**Phase 2 terminée le 24 octobre 2026, après les compléments validés en D21.**
Le [bilan de sortie](docs/architecture-phase-2-review.md) confronte l’inventaire,
les signatures, les limites restantes et les preuves de validation.

- [x] `App::apply` ne contient plus que des `UiCommand` et des appels au moteur.
- [x] Aucune publication directe de session par l’interface ; la saisie native
  utilise la voie commune de commit D17. Le chargement interne des démonstrations
  reste hors catalogue et sous validation du moteur.
- [x] Représentation structurée migrée et cibles validées pour les blocs existants.
  Les zones préparent de futures cellules ; aucune commande de tableau livrée ici.
- [x] Inventaire classé et catalogue généré relu : 49 commandes et 19 requêtes ;
  cinq lectures ajoutées, deux réponses enrichies, vues redondantes sans alias.
- [x] **Gel des identifiants et contrats d’entrée existants** : D21 mise en œuvre
  après les compléments demandés. Pas de changement incompatible sans décision
  explicite ; les ajouts compatibles restent possibles. Les schémas exhaustifs
  de sortie seront précisés avant les transports CLI/MCP.

---

## Phase 3 — Espace de travail (taille M)

**But** : `typstographe-core` sans aucune dépendance à egui, eframe, winit, `rfd` ou `muda`.

**Réalisée le 24 octobre 2026, vérifications locales réussies.** Le workspace
contient `crates/core`, `crates/i18n` et `crates/app`. Le cœur compile seul et son
arbre normal ne contient aucune bibliothèque graphique ou Fluent. Les tests
et les captures sont conservés ; le repli anglais est appliqué dans un commit
distinct. Voir le [bilan de phase 3](docs/architecture-phase-3.md).

- [x] **3.1** Corriger les fuites graphiques **dans le crate actuel** avant les
  déplacements : réveils de rendu/récupération par rappel (D8), images RGBA sans
  egui et conversion dans l’application. Présentations des commandes de types et
  modèles déplacées dans `commands/document_types.rs`, catalogue documentaire pur.
  Les préférences et l’interface des champs sont déjà côté application ; la
  gestion du profil les y rejoindra lors du découpage (D22). `CommandSpec` et
  `Presentation` sont séparés. Voir [le suivi de phase 3](docs/architecture-phase-3.md).
- [x] **3.2** Retirer Fluent des modules destinés au cœur : familles d’erreurs
  typées dans `errors.rs`, variantes, paramètres et causes sérialisables ; libellés
  et textes indicatifs exposés comme clés de données. La conversion vers
  `CommandError` conserve les six codes, les raisons et le champ textuel `detail`,
  et ajoute une cause structurée quand elle est disponible. Les diagnostics et
  lectures de récupération conservent eux aussi cette cause. La mise en phrases
  commune réside dans `crates/i18n/src/lib.rs` / `crates/i18n/src/errors.rs`, utilisée par l’application et
  réutilisable par les futurs transports. Les contrats des 44 messages du cœur
  et des libellés sont vérifiés en français et anglais sans masquer les manques
  par le repli. Aucun appel Fluent ne reste dans les modules destinés au cœur.
- [x] **3.3** Créer l'espace de travail : `Cargo.toml` racine `[workspace]`,
  `resolver = "3"`, `[workspace.dependencies]` pour les versions épinglées
  (`typst =0.15.1`, etc.).
- [x] **3.4** Créer `crates/core` : y déplacer `document`, `annotations`, `search`,
  `statistics`, `session`, `identity`, `errors`, `storage`, `packages`, `spelling`, `recovery`,
  `rendering`, `navigation`, `assets`, `fonts`, `exporting`, `dates`, `document_types`, `engine` et
  `crates/core/src/tests.rs`. Déplacer avec eux `crates/core/templates/`, `crates/core/examples/`, `crates/core/assets/dictionaries/` et
  `crates/core/assets/fonts/` (secours communs éditeur/Typst) ;
  utiliser le dossier partagé `fixtures/` à la racine (D9) ;
  corriger les `include_*` et les `#[folder]`.
- [x] **3.4b** Créer `crates/i18n` (`typstographe-i18n`, dépend de `typstographe-core`) :
  déplacer `crates/i18n/src/lib.rs` et `crates/i18n/src/` (chargeur, macro `tr!`, formateur partagé,
  tests des catalogues) et `crates/i18n/i18n/` (fr, en,
  `crates/i18n/i18n.toml`) ; y transférer la mise en phrases de `CommandError` et des erreurs typées du
  cœur déjà préparée en 3.2. Une seule langue active, choisie une fois par le programme appelant. La langue
  de repli est l'**anglais pour tous les programmes** (application, CLI, MCP) : passer
  `fallback_language` de `"fr"` à `"en"` dans `crates/i18n/i18n.toml`. C'est un changement de
  comportement visible (un système dont la langue n'est ni le français ni l'anglais passe
  de l'interface française à l'interface anglaise), réalisé dans le commit distinct
  `4df98fe` et consigné dans le bilan : changement volontaire au sens de la section 1.
- [x] **3.5** Créer `crates/app` (`typstographe`, dépend de `core` et de `i18n`) : `app/`,
  `editor`, `toolbar`, `menus`, `icons`, `preferences`, `profile`, `commands` (présentation), `main`.
  Reproduire `#![cfg_attr(coverage_nightly, feature(coverage_attribute))]` dans chaque racine.
- [x] **3.6** CI : `cargo test --workspace`, `cargo clippy --workspace --all-targets`,
  couverture inchangée (`--workspace`). Ajouter une étape **bloquante** :
  `cargo tree -p typstographe-core -e normal` ne doit contenir aucun de
  `egui`, `eframe`, `winit`, `rfd`, `muda`, ni `fluent*` et `i18n-embed*`.
- [x] **3.7** Ajouter une étape **informative** (non bloquante) :
  `cargo check -p typstographe-core --target wasm32-unknown-unknown`. Elle échouera au
  début ; elle mesure la distance à une éventuelle cible WASM sans l'engager.
  Étape Linux avec installation de cible, `continue-on-error: true` ; localement
  la cible est absente, ce qui ne permet pas de conclure à la compatibilité.

- [x] **3.8** Avant d’appliquer les limites globales, découper explicitement le code
  de production de `document.rs` (1 382 lignes au relevé) et d’`editor.rs` (1 045 lignes
  avant son module de tests). Isoler projection/sérialisation/parsing et les fonctions
  de composition/décoration, extraire les tests. Commits de déplacement séparés des
  changements de comportement ; ne pas changer de modèle d’édition implicitement.
  **Réalisé :** façades de 194 et 67 lignes, responsabilités dans `document/` et
  `editor/`, tests d’éditeur séparés ; plus gros modules de production de ces
  dossiers à 418 et 365 lignes. Seuils appliqués en CI à ces dossiers et à `engine`.
  Voir aussi le [contrat public des adaptateurs](docs/engine-adapters.md), préparé
  sans exposition de `Session` ni modification du catalogue machine.

**Critères de sortie** : mêmes tests et mêmes captures qu'avant, hors changements
volontaires consignés (par exemple le repli en anglais) ; l'arbre de dépendances
du cœur est propre ; l'application se lance et ouvre un fichier existant.
**Vérifiés localement :** 356 tests entre les trois crates et 6 tests de diagnostic,
Clippy, contrôle du cœur indépendant, 14 tests de scripts et 8 captures identiques.
Un `.typsto` antérieur avec japonais/chinois se rouvre et se compose correctement.
Les vérifications Windows/Linux seront exécutées par la CI après la poussée.

---

## Phase 4 — CLI (taille M)

**But** : tout ce que fait l'interface sur un document se fait aussi sans fenêtre.

**En cours : premier lot livré le 24 octobre 2026.** Catalogue, inspection,
validation et export réutilisent le moteur, avec verrou d’écriture et contrôle
des modifications externes communs à l’UI. **Deuxième lot livré le même jour :**
création depuis le catalogue, `query`/`run`, paramètres JSON bornés et cibles
résolues dans la session ; aperçu, copie ou sauvegarde explicite. **Troisième lot
livré le même jour :** scripts JSON versionnés, cibles résolues au fil du brouillon
et une seule transaction avec sauvegarde finale facultative. Détail et suite dans
[le bilan de phase 4](docs/architecture-phase-4.md).

### 4.1 Squelette

- [x] Crate `typstographe-cli`, `clap` en dérivation (D10).
- [ ] Retirer le diagnostic historique `--render-demo` du lanceur graphique et adapter
  le script de références visuelles ; les options `--capture-*` restent des outils de développement.
- [x] Réutiliser `Command::OpenDocument`, `SaveDocument`, `SaveAs` et `Export`, sans
  ajouter des méthodes parallèles `Engine::open/save`. Lecture CLI d’archives GUI vérifiée ;
  création CLI et réouverture par la même commande que l’application vérifiées.
- [x] `spelling.issues` fournit déjà une lecture synchrone, utilisée par `check --spelling`.
- [x] Verrou interprocessus, contrôle d’empreinte et lecture seule dans le moteur (D24).

### 4.2 Commandes

| Commande | Rôle |
| --- | --- |
| `typstographe-cli commands list [--json] [--query]` | catalogue du registre |
| `typstographe-cli commands describe <id>` | description et schéma des paramètres |
| `typstographe-cli new --type courrier --model <id> -o f.typsto` | création |
| `typstographe-cli inspect f.typsto [--json]` | métadonnées, blocs, statistiques |
| `typstographe-cli query <id> [f.typsto] [--params JSON ou --params-file chemin]` | lecture du registre ; fichier nécessaire pour les requêtes documentaires |
| `typstographe-cli run <id> f.typsto [--params JSON ou --params-file chemin] [--select texte ou --block N] [--save ou --output copie.typsto]` | commande documentaire ; mémoire seule par défaut, cibles résolues dans la session |
| `typstographe-cli script job.json` | lot d’édition d’un document sur session temporaire ; un seul enregistrement atomique après réussite, aucune écriture intermédiaire (D16), taille limitée |
| `typstographe-cli export f.typsto --format pdf\|typst\|both --output dir [--pages]` | exports ; PDF par défaut, PNG seulement avec `--pages` |
| `typstographe-cli check f.typsto [--for-export] [--spelling]` | validité du brouillon ; champs requis et orthographe sur demande ; sortie non nulle si le contrôle demandé échoue |
| `typstographe-cli render f.typsto --page N --png out.png` | aperçu |

### 4.3 Contrat

- [x] Codes de sortie : 0 succès, 1 opération refusée ou contrôle négatif, 2 usage, 3 entrée/sortie.
- [x] Sortie `--json` stable : `{ "version": 1, "ok": true, "outcome": … }` ou
  `{ "version": 1, "ok": false, "error": { "code", "message", "params" } }`.
  Un contrôle négatif a `ok: true`, `outcome.passed: false` et une sortie 1.
- [x] Messages en français ou en anglais via `typstographe-i18n` (D7) : option `--lang fr|en`,
  les codes du JSON restent identiques. **Langue par défaut : celle du système, avec repli
  en anglais**, comme l'application (décidé le 21 octobre 2026).
- [x] Format du script (`job.json`) versionné : `{ "version": 1, "document": …, "steps": [ {"id": …, "params": …} ], "save": … }`.
  Cibles portables facultatives par étape (`target.select/occurrence` ou `target.block`),
  résolues sur le brouillon courant ; chemins relatifs au script, limites de 4 Mio et
  256 étapes. Aperçu sans `save`, sauvegarde courante ou nouvelle copie sans écrasement.

### 4.4 Tests

- [x] Tests d'intégration par processus (`assert_cmd`), inclus dans la CI des trois systèmes ;
  exécutés localement sur macOS, résultats distants à confirmer après la poussée.
- [x] Atomicité : une erreur de validation ou d’exécution, même à la dernière étape,
  laisse intacts la session publiée et le fichier précédent. Refuser `save`, `export`
  et les effets globaux dans les étapes ; tester ces refus et l’échec d’écriture finale.
- [x] Parité : pour un échantillon de commandes, résultat CLI = résultat de l'interface
  (même `Engine`, comparaison du source Typst produit).
- [x] Fichier créé en CLI ouvert dans l'application et inversement.
- [x] `docs/cli.md` généré depuis le registre, vérifié dans les tests exécutés par la CI.
- [x] Création depuis le catalogue et appels génériques : paramètres JSON argument/fichier/stdin,
  limite 4 Mio, gardes automatiques, sélection exacte ou numéro de bloc, refus des références
  périmées et des familles nécessitant une session durable, protections de fichiers conservées.

**Scénario d'acceptation** : créer un courrier, renseigner les métadonnées, insérer
un paragraphe et un post-scriptum dans un script JSON, enregistrer, puis exporter en PDF
par une commande distincte, sans fenêtre. L’export est hors transaction (D16/D23).
**Réussi le 24 octobre 2026** avec `examples/cli/courrier.json`, puis réouverture
native et inspection de la capture. La page PNG dédiée et le retrait de
`--render-demo` restent à réaliser avant de clore la phase.

---

## Phase 5 — MCP (taille M)

**But** : un assistant IA pilote le moteur par le même registre, avec des garde-fous.

### 5.1 Choix de la mise en œuvre (D11)

- [ ] Prototype d'une journée avec `rmcp` (D11) : consulter d'abord la documentation de
  la version courante ; vérifier qu'une liste d'outils **dynamique** (dérivée du registre)
  est possible, mesurer la taille du binaire et le temps de compilation, confirmer
  que `tokio` reste confiné à la CLI. Consigner le résultat ; en cas d'échec, repli sur
  un JSON-RPC minimal sur stdio.

### 5.2 Serveur

- [ ] Sous-commande `typstographe-cli mcp [--read-root …] [--write-root …] [--read-only]`,
  transport **stdio**.
- [ ] Outils dérivés du registre plutôt que redéclarés à la main, plus un petit
  jeu d'outils de confort :

| Outil | Rôle |
| --- | --- |
| `list_commands`, `describe_command` | catalogue et schémas |
| `execute`, `batch` | une commande ; lot d’édition d’un document sur copie temporaire, validé et annulable en une étape (D16) |
| `new_document`, `open_document`, `save_document`, `export` | cycle de vie |
| `get_document` | structure : blocs, plages, styles, métadonnées |
| `get_text` | texte brut avec indices de caractères |
| `check_document` | orthographe, champs requis, validité du modèle |
| `render_page` | page en PNG |
| `list_models` | modèles et paramètres disponibles |

- [ ] Ressources : `typstographe://document`, `typstographe://commands`, `typstographe://models`.
- [ ] Les erreurs MCP reprennent les codes de `CommandError`, lisibles et corrigeables
  par l'agent, avec un texte en anglais par défaut (option explicite pour une autre langue).

### 5.3 Sécurité (D12)

Inspiré de ce que documente WordCraft (racines de lecture et d'écriture séparées,
liens résolus avant contrôle, enregistrement explicite). À notre manière :

- [ ] **Refus par défaut** : sans racine déclarée, aucun accès à un fichier.
- [ ] Chemins canonicalisés ; échappement par lien symbolique ou `..` refusé.
- [ ] Racines de lecture et d'écriture indépendantes ; `--read-only` interdit toute commande
  modifiante et tout `save`.
- [ ] La session MCP travaille sur une copie en mémoire ; rien n'est écrit sans
  `save_document` ou `export`. Les étapes d’un `batch` excluent ces deux opérations
  et les effets globaux ; un échec conserve la session publiée (D16).
- [ ] Limites réutilisées : budget d’images (`engine/images.rs::check_budget`), taille de document,
  taille de script.
- [ ] Aucune commande réseau, aucun processus lancé.
- [ ] Documenter les limites connues (par exemple : `text.insert` peut modifier le document
  ouvert dans la session même sans racine d'écriture ; condition de concurrence sur les
  liens remplacés entre contrôle et ouverture).
- [ ] Dans l'historique, un appel d'agent est **une** transaction annulable, pour tenir le
  principe « le texte de l'utilisateur ne doit pas être réécrit silencieusement ».

### 5.4 Tests

- [ ] Conversation JSON-RPC de bout en bout par processus (initialisation, liste des outils,
  appels).
- [ ] Validation des schémas d'entrée de chaque outil.
- [ ] Confinement : `..`, liens symboliques, chemins absolus hors racine, mode lecture seule.
- [ ] **Test d'acceptation** : un courrier mis en forme écrit uniquement par appels MCP,
  exporté en PDF.
- [ ] Essai manuel avec un client réel :
  `claude mcp add typstographe -- typstographe-cli mcp --write-root ./sortie`
  et avec l'inspecteur MCP.
- [ ] `docs/mcp.md` : outils, politique de sécurité, exemples.

---

## Phase 6 — Canal de contrôle de l'application (facultatif, D14)

Reporté. À décider après la phase 5, seulement si un besoin concret apparaît :
connexion de la CLI ou de MCP à l'application en cours d'exécution (boucle locale, clé de
contrôle, captures d'écran, clics et touches pour tests de bout en bout et essais
assistés). Ne rien construire avant d'avoir tranché.

---

## Phase 7 — Consolidation (taille S)

- [ ] Rendre bloquants `clippy::too_many_lines` (seuil 120) et le script de taille de
  fichiers (800 lignes) en CI (D13).
- [ ] Mettre à jour `README.md` (installation, CLI, MCP) et `ROADMAP.md` : nouveau chantier J,
  principes « commandes à identifiant stable exposées par CLI et MCP » et
  « cœur indépendant de l'interface ». (Le principe de `ROADMAP.md` sur la langue de
  repli, devenue l'anglais, est déjà corrigé.)
- [ ] Vérifier la CI sur trois systèmes, la couverture et les temps de compilation.
- [ ] Rétrospective courte : ce qui a changé de périmètre, ce qui reste.

---

## 8. Risques et parades

| Risque | Parade |
| --- | --- |
| Régression silencieuse pendant les déplacements | déplacements purs par commit, PR cohérentes, comparaison des captures, tests inchangés |
| Visibilité des méthodes (`pub(super)`) qui s'étend | revue en 1.4 : retirer ce qui n'est plus nécessaire |
| Les commandes paramétrées ne couvrent pas la saisie native d'egui | adaptateur natif explicite ; même voie de commit et mêmes règles que `text.insert`, tests de parité (D17) |
| Conversion des `tr!` plus longue que prévu (environ 90 appels) | traiter module par module, test des catalogues étendu |
| Chemins `include_*` après déplacement | tâche 3.4 explicite ; compilation de chaque crate seule |
| Dérive entre catalogue, schémas et documentation | génération depuis le registre et contrôle dans la CI |
| Surface MCP trop permissive | refus par défaut, lecture seule, tests de confinement |
| Verrouillage prématuré des identifiants | gel seulement à la fin de la phase 2, après relecture de l'inventaire |

## 9. Repéré en chemin, hors de ce périmètre

À traiter séparément, chacun avec sa propre décision :

- **Composition de texte et IME** : les vérifications de décision sont avancées en
  1.6 (D18). Une éventuelle réécriture de l’éditeur reste un chantier séparé ; les
  résultats du prototype borné détermineront son périmètre.
- **Performance de `layout_with_search`** (`editor.rs`) : clone de la projection à chaque
  appel, une section par caractère, recherche quadratique de `empty_terms`. À traiter
  avant les documents longs.
- **Module `galley`** : factoriser les décalages de lignes et de glyphes dupliqués
  (citations, images, espacement des blocs).
- **i18n** : « P.-S. » et « P.J. » sont écrits en dur (`editor.rs`, lignes 893 à 898).
- **Audit des `unwrap`/`expect`** en code de production (environ 167 occurrences relevées
  dans `app.rs`, dont une part dans les tests, non triée).
- **`experiments/reformulation/`** : volumineux pour une piste abandonnée ; envisager de
  l'archiver hors de la branche principale.
- **Abstraction de stockage** (fichier local, WebDAV, autre) : prérequis commun d'une
  éventuelle cible WASM, d'un connecteur Nextcloud et de la récupération.

## 10. Définition de « terminé »

La refonte est terminée lorsque :

1. aucun fichier de production n'excède 800 lignes et aucune fonction 120 ;
2. `typstographe-core` compile et passe ses tests sans egui, eframe, winit, `rfd`, `muda`,
   Fluent ni `i18n-embed` ;
3. les actions passent par une `Command`, une `Query` ou une `UiCommand` ;
   l’adaptateur de saisie native partage la voie de commit du cœur, avec tests de parité ;
4. le catalogue de commandes, la documentation de la CLI et celle de MCP sont générés
   depuis le registre et contrôlés dans la CI ;
5. le scénario d'acceptation de la CLI et celui de MCP passent sur trois systèmes ;
6. les captures d'écran et les tests de l'application sont identiques à ceux de la
   phase 0, aux changements volontaires consignés près.
