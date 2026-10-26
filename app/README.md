# Application typstOgraphe

Tauri héberge l’éditeur Tiptap/ProseMirror et Yjs ; le cœur Rust valide les
documents et fournit le rendu Typst. Toutes les commandes ci-dessous se lancent
depuis la **racine du dépôt**, avec Rust, Node.js et les
[prérequis Tauri](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm start
```

`npm start` génère les déclarations de langues depuis Fluent, lance Vite puis
l’application Tauri avec rechargement du frontend. `npm run tauri -- …` donne
accès aux autres commandes Tauri. Par exemple :

```sh
npm start -- -- --locked --offline
```

Les arguments après le second `--` sont transmis à Cargo. L’application conserve
son identifiant natif afin de retrouver les préférences et la récupération
existantes. L’accueil animé vient de `assets/branding/` ; les icônes natives et
leurs variantes par système sont dans `app/src-tauri/icons/`.

## Frontend embarqué et lancement Cargo

```sh
npm run build:web
cargo run --locked --offline
```

Cargo lance désormais Tauri avec les fichiers web embarqués, sans serveur Vite.
Reconstruire le frontend après une modification HTML/CSS/TypeScript ; `npm start`
est le parcours habituel pour travailler sur l’interface. Le script de build
Rust génère les mêmes langues natives que le lanceur Tauri.

`npm run build` produit le binaire optimisé. Pour produire un paquet natif :

```sh
npm run tauri -- build --bundles app
```

`app` désigne le bundle macOS ; choisir les formats adaptés aux autres systèmes.

## Vérifications

```sh
npm test
npm run check:format
npm run build:web
npm run test:rust
npm run check:rust
cargo build --locked --offline -p typstographe
npm run test:browser
```

Les recettes de navigateur utilisent un port et une session séparés de la
rédaction. `npm run test:native` vérifie le parcours natif avec une session
isolée ; `npm run test:preview-native` mesure le binaire optimisé. Voir la
[méthode de mesure](../docs/tauri-preview-performance.md).

La référence egui reste accessible explicitement avec
`cargo run --locked --offline -p typstographe-egui` jusqu’à son retrait.
La CLI documentaire conserve son contrat historique ; sa convergence avec
l’éditeur structuré constitue le chantier A.2.
