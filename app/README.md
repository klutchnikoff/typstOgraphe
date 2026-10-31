# Développer typstOgraphe

Tauri héberge Tiptap/ProseMirror et Yjs ; Rust/Yrs possède l’état confirmé,
valide les éditions et compile Typst. Les [contrats et bilans](../docs/README.md)
expliquent les responsabilités. Toutes les commandes ci-dessous partent de la
**racine du dépôt**.

## Prérequis et premier lancement

- Rust stable, version minimale du workspace : 1.92 ; Cargo, rustfmt et Clippy.
- Node.js 26 et npm (versions de la CI) ; dépendances JS fixées par `package-lock.json`.
- [Prérequis natifs Tauri](https://v2.tauri.app/start/prerequisites/) : outils Xcode
  sur macOS, chaîne MSVC/WebView2 sur Windows, bibliothèques WebKitGTK 4.1 sur Linux.
  Les paquets exacts utilisés sous Linux figurent dans `.github/workflows/desktop.yml`.

```sh
npm ci
npm start
```

L’application s’ouvre à l’accueil, sans projet implicite. `npm start` lance Vite
et Tauri avec rechargement du frontend. Pour transmettre les options Cargo :

```sh
npm start -- -- --locked --offline
```

Le mode hors ligne suppose que les dépendances de construction ont déjà été
obtenues. L’application et les projets n’ont ensuite pas besoin d’un service
réseau pour ouvrir, éditer, compiler ou exporter.

## Frontend embarqué et application normale

```sh
npm run build:web
npm run check:distribution
cargo run --locked --offline
```

Cargo embarque `app/dist/`, sans Vite. Reconstruire le frontend après une
modification HTML/CSS/TypeScript. La compilation normale ne possède ni diagnostics
natifs, ni transport de recette `--stdio`, ni pont de test JavaScript.
L’accueil animé et les icônes viennent de `assets/branding/` et `app/src-tauri/icons/`.

`npm run build` compile le binaire optimisé. Le packaging macOS de test utilise
`npm run tauri -- build --bundles app` ; voir la [CI](../docs/ci.md) pour les
formats Linux/Windows. Signature, notarisation et installation restent dans H.
L’identifiant natif est conservé pour retrouver préférences et récupération.
Les langues natives sont générées depuis Fluent, y compris pour un lancement
Cargo sans `.app`. Sur macOS :

```sh
node app/scripts/check-native-locales.mjs target/debug/typstographe
```

## Vérifications courantes

```sh
npm test
npm run check:format
npm run build:web
npm run check:distribution
cargo test --locked --workspace --features typstographe/recipe-driver
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked -p typstographe --all-targets --features recipe-driver -- -D warnings
cargo fmt --all -- --check
python3 -m unittest discover -s scripts/tests -v
python3 scripts/check_file_length.py
python3 scripts/check_core_dependencies.py
```

Les [choix de CI](../docs/ci.md) évitent de répéter les recettes lourdes sur
chaque PR. `npm run test:coverage` produit la couverture web ; le workflow dédié
produit celle du workspace Rust. Le catalogue est vérifié par les tests du cœur.

## Recettes isolées

```sh
npm run build:recipe -- --offline
npm run test:contract --workspace @typstographe/app
npm run test:browser:smoke
```

`build:recipe` produit le frontend explicite dans `app/dist-recipe/`, puis compile
l’application avec `recipe-driver` et la CLI, via l’overlay Tauri `TAURI_CONFIG`.
Le frontend normal `app/dist/` n’est pas modifié. `--release` choisit les binaires
optimisés ; les autres options sont transmises à Cargo. Les exécutables sont
copiés dans `target/recipes/debug/` ou `target/recipes/release/` et leur capacité
STDIO est vérifiée. Une compilation normale ne les remplace donc pas, sans dupliquer
le cache Cargo. `TOG_SPIKE_BINARY` / `TOG_CLI_BINARY` permettent aussi de choisir
explicitement les exécutables (notamment avec `CARGO_TARGET_DIR`).

Le navigateur utilise le mode Vite `browser-test`, un serveur distinct sur le
port 1421 et un processus Rust STDIO isolé. `npm run test:browser` lance toute la
suite Chromium/WebKit ; `:smoke` sélectionne les parcours critiques. Installer
les moteurs au premier usage avec `npx playwright install chromium webkit`.

Les recettes natives exigent un bureau disponible et des **dossiers nouveaux ou
vides**. Elles ne touchent qu’à leurs propres fichiers et processus :

```sh
npm run test:closure-native -- /chemin/vers/un/dossier-vide
npm run test:draft-conservation-native -- /chemin/vers/un/autre-dossier-vide
npm run test:publication-native -- /chemin/vers/un/troisieme-dossier-vide
```

La première fait circuler Courrier et Article entre Tauri, CLI et MCP, avec
Versions, comparaison/reprise, annulation, sauvegarde/réouverture et exports.
Sous Linux, la CI utilise Xvfb et une session D-Bus. `npm run test:native` conserve
la recette historique des deux fenêtres. Pour la mesure embarquée optimisée,
construire avec `npm run build:recipe -- --release`, puis lancer
`npm run test:preview-native`. Voir la [méthode](../docs/tauri-preview-performance.md).
Aucune recette active ne crée de capture d’écran, vidéo ou trace Playwright.

## Répertoires et conservation

| Emplacement | Rôle |
| --- | --- |
| `crates/core` | Contrat, propriétaire Yrs, validation, CAS, modèles/capacités et Typst ; sans GUI ni Fluent |
| `crates/cli`, `crates/mcp` | Adaptateurs sans fenêtre et transport MCP |
| `crates/i18n` | Catalogues Fluent, messages et langues natives |
| `app/src`, `app/src-tauri` | Interface web et hôte natif ; diagnostics sous feature/mode explicites |
| `examples/`, `fixtures/` | Exemples utilisables et corpus de tests ; ne pas modifier un original pendant une recette |
| `experiments/`, `docs/` | Résultats, contrats, provenance et références historiques |
| `output/`, `target/`, `app/dist*` | Sorties ignorées et reconstruisibles ; ne pas confondre avec des projets personnels |

Un `.typsto` est un projet autonome CAS 11, avec état courant, modèle, ressources
et versions. Les préférences globales et formulaires privés restent à part.
Voir le [format](../docs/a4-project-format.md), la [récupération](../docs/a3-conservation.md)
et le [comparateur](../docs/a4-comparison-contract.md). Une composition est une
édition annulable ; enregistrer et créer une version sont des choix distincts.
