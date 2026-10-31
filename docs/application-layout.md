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
| `assets/ui/` | Polices et icônes de l’application |
| `experiments/web-editor/results/` | Résultats historiques conservés |
| `output/` | Rapports des recettes locales ; captures historiques conservées |

Les commandes courantes sont dans le [guide de développement](../app/README.md).
`npm start` lance le développement Tauri depuis la racine ; `cargo run` lance
le même hôte avec le frontend construit et embarqué. Les deux voies génèrent
les langues natives à partir de Fluent. Le manifeste Tauri, ses variantes
macOS/Windows et les fichiers d’icônes sont conservés. L’identifiant de bundle
existant reste inchangé pour retrouver les données locales.

Sur macOS, les trois parcours doivent disposer des déclarations de langues
issues de Fluent : `npm start`, l’exécutable autonome de `cargo run` et le bundle
`.app`. Tauri embarque lui-même `Info.plist` en développement ; avec
`custom-protocol` (frontend embarqué, activé par défaut), `native_locales.rs`
assure cet embarquement, car l’exécutable peut être lancé sans bundle.
Le fichier `.app` conserve sa fusion habituelle de ce même `Info.plist`.

Régression identifiée après la promotion de Tauri dans A.1 : le générateur de
langues était toujours exécuté par Cargo, mais Tauri n’insérait son résultat dans
le Mach-O qu’en mode développement. Les tests du générateur ne détectaient pas
l’absence de cette section dans un exécutable autonome. Le contrôle macOS de CI
lit désormais la section `__TEXT,__info_plist` de l’application réellement
construite et compare ses langues et son repli aux catalogues Fluent. Les tests
natifs légers vérifient aussi la négociation Foundation français/anglais/repli,
sans fenêtre ni changement des préférences système. Ils ne remplacent pas une
vérification visuelle des libellés fournis par les services WebKit de chaque OS.

Les actions standard créées par Tauri/Muda (`About`, `Hide`, `Cut`, etc.) ont des
libellés anglais par défaut, indépendamment de ces déclarations. Leurs libellés
sont désormais fournis explicitement par Fluent, avec les actions natives
préservées. Un test vérifie français, anglais et repli. Le titre et le filtre du
sélecteur de projet viennent également de Fluent ; ses commandes internes
restent celles de macOS. **Problème confirmé, reporté le 31 octobre 2026 :**
avec `npm start`, ces boutons restent anglais chez l’utilisateur. La première
sonde pouvait confondre le titre « Ouvrir » avec un bouton : elle ne prouvait
pas la traduction des commandes du service système. Le
[diagnostic détaillé](native-localization-investigation.md) consigne cette
limite, les essais non concluants retirés et la piste `dbv-typst-editor` à examiner.

Recette de cette correction : cinq tests de langues natives (dont négociation
Foundation en français/anglais/repli), onze tests i18n, quinze tests des scripts,
Clippy de l’hôte/i18n sur toutes leurs cibles et contrôles de formatage réussis.
Le binaire autonome avec frontend embarqué a été construit hors ligne ; son
unique section de langues correspond aux catalogues Fluent. Les libellés
standard de Tauri étaient déjà laissés en anglais dans la première version de
ses menus : leur défaut de traduction est distinct du défaut d’embarquement
introduit par le parcours Cargo autonome.

Lors de cette vérification, le disque a atteint sa capacité. Le nettoyage
`cargo clean --offline --profile dev -p typstographe -p typstographe-core
-p typstographe-i18n -p typstographe-cli -p typstographe-mcp` a retiré 51,6 Gio
d’artefacts accumulés, en conservant les dépendances tierces. `target/` est passé
d’environ 63 à 13 Go avant reconstruction. Sources, projets, ressources,
rapports et résultats d’essais ont été conservés.
La reconstruction hors ligne du mode développement (`--no-default-features`,
comme `npm start`) a ensuite réussi en réutilisant les dépendances conservées ;
son binaire passe le contrôle des langues embarquées. Après reconstruction,
`target/` pèse environ 14 Go.

La CI Tauri construit le frontend et vérifie l’hôte sur les trois systèmes,
puis les recettes Chromium/WebKit sous Linux, sans captures. La CI Rust vérifie
le cœur, la CLI et le MCP indépendamment des bibliothèques de l’interface.
La couverture Linux instrumente désormais tout le workspace Rust, hôte Tauri
compris ; elle construit donc aussi le frontend et installe WebKit.
Le contrôle de frontière du cœur reste indépendant de Tauri.

Les résultats et captures antérieurs sont conservés à leur emplacement. Le
[guide du prototype](archive/web-editor-prototype-guide.md) est archivé ; ses
anciennes commandes ne sont pas le parcours de lancement courant.

L’interface egui a été retirée après les reprises décrites dans le
[bilan du nettoyage](egui-retirement-results.md). La convergence du contrat
CLI/MCP autonome avec l’éditeur structuré reste le chantier A.2.
Les chiffres ci-dessous sont ceux du lot initial A.1 ; le bilan du nettoyage
précise les vérifications suivantes.

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
