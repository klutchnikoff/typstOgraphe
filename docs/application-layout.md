# Application principale — chantier A.1

Le 27 octobre 2026, Tauri devient l’application principale. Le frontend et son
hôte Rust sont dans `app/`, membre du workspace npm et du workspace Cargo de la
racine. Les dépendances sont verrouillées par un seul `package-lock.json` et un
seul `Cargo.lock` ; les compilations Rust partagent `target/`.

| Emplacement | Rôle |
| --- | --- |
| `app/src/` | Interface Tiptap/ProseMirror, Yjs et présentation de l’aperçu |
| `app/src-tauri/` | Exécutable `typstographe`, hôte Tauri et adaptateurs natifs |
| `assets/branding/` | Sources de l’accueil animé et de l’identité tO |
| `assets/ui/` | Polices et icônes partagées entre interfaces |
| `crates/app/` | Référence `typstographe-egui`, lancement explicite uniquement |
| `experiments/web-editor/results/` | Résultats historiques conservés |
| `output/` | Rapports et captures des recettes locales |

Les commandes courantes sont dans le [guide de développement](../app/README.md).
`npm start` lance le développement Tauri depuis la racine ; `cargo run` lance
le même hôte avec le frontend construit et embarqué. Les deux voies génèrent
les langues natives à partir de Fluent. Le manifeste Tauri, ses variantes
macOS/Windows et les fichiers d’icônes sont conservés. L’identifiant de bundle
existant reste inchangé pour retrouver les données locales.

La CI Tauri construit le frontend et vérifie l’hôte sur les trois systèmes,
puis les recettes Chromium/WebKit sous Linux. La CI Rust et la couverture
conservent leur périmètre cœur/CLI/MCP/référence egui ; elles excluent explicitement
l’hôte Tauri testé par sa CI dédiée, qui nécessite Node et les bibliothèques
système WebKit. Le contrôle de frontière du cœur reste indépendant de Tauri.

Les résultats et captures antérieurs sont conservés à leur emplacement. Le
[guide du prototype](archive/web-editor-prototype-guide.md) est archivé ; ses
anciennes commandes ne sont pas le parcours de lancement courant.

Le retrait d’egui et la séparation supplémentaire des diagnostics embarqués
restent à décider après vérification des parcours nécessaires, détaillés dans
l’[inventaire avant retrait](egui-retirement.md). La convergence
du contrat CLI/MCP autonome avec l’éditeur structuré relève de A.2.

## Vérifications du lot

- Installation npm depuis la racine et construction TypeScript/Vite réussies.
- 66 tests frontend et déclarations de langues, 72 parcours Chromium/WebKit.
  Accueil clair/sombre inspecté : logo, trois actions, animation et mouvement
  réduit conservés ; les ressources graphiques et icônes natives sont inchangées.
- 10 tests de l’hôte Tauri réussis, un diagnostic de performance ignoré comme
  auparavant ; 604 tests du reste du workspace, doctests compris. Le test du
  catalogue généré est aussi rejoué après correction de sa commande de référence.
- 14 tests des scripts d’architecture, frontière du cœur, formatages et Clippy
  sur le workspace et toutes ses cibles, avertissements refusés.
- Recette native macOS complète et réouverture : rendu réel, édition,
  sauvegarde, ressources et proposition MCP. Lancements réels depuis la racine
  avec `npm start` et `cargo run --locked --offline`, dans une session isolée.
- Bundle macOS construit depuis la racine : exécutable `typstographe`, langues
  natives issues de Fluent et icône `.icns` identique à la source contrôlés.
  La traduction effective de chaque menu système n’est pas déduite de ce seul
  contrôle de métadonnées.

Rapports et captures locaux : `output/a1-validation/` ; recette native dans
`output/a1-native/`. Les vérifications natives Windows/Linux et la mesure de
couverture restent celles des workflows distants, pas des essais effectués sur
ce poste macOS.
