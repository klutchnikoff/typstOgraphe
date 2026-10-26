# Diagnostic Tauri / Tiptap / Yjs

Prototype isolé, sur `codex/spike-tauri-editor`. Les
[choix validés](../../docs/architecture-web-editor-spike.md) précèdent son code.
Il réutilise `typstographe-core` sans modifier l'application egui, le catalogue
documentaire ni le format `.typsto`.

## Lancer

Depuis ce répertoire, avec Rust, Node.js et les
[prérequis Tauri](https://v2.tauri.app/start/prerequisites/) :

```sh
npm ci
npm run tauri dev
```

« Deuxième fenêtre » ouvre un deuxième participant. Chaque fenêtre possède son
propre `Y.Doc` ; les mises à jour et présences passent par un relais Tauri local.
« Hors connexion » permet de travailler sans recevoir les éditions du voisin,
puis « Connecté » fusionne les états. L'annulation est celle de Tiptap/Yjs,
propre à chaque participant ; le moteur Rust ne maintient pas un second historique
de frappe. « Essai Rust » insère dans ce même circuit un paragraphe préparé par Rust.

Le diagnostic s'ouvre sur une fixture ou la session expérimentale enregistrée.
Le dossier par défaut est `output/web-editor/default/` à la racine du dépôt.
Pour un essai indépendant :

```sh
npm run tauri -- dev -- -- --session-dir /chemin/absolu/vers/un/dossier-essai
```

## Édition et rendu

- Paragraphes, titres 1–3, gras, italique, liens, listes non imbriquées.
- Dans le corps libre : Entrée reste dans le paragraphe (espace dans le rendu),
  deux Entrées créent un paragraphe. Maj+Entrée force un saut de ligne Typst.
  Listes, cellules, titres et légendes gardent leur navigation structurelle.
- Image avec légende directement éditable ; paramètres pour le texte alternatif.
- Tableau sans fusion ; commandes de lignes et colonnes lorsque le curseur y est.
- Note en ligne avec bandeau de texte, Entrée pour appliquer, Échap pour fermer.
- Cmd/Ctrl+Entrée ouvre les paramètres d'une note ou d'une image ciblée.
- Cmd/Ctrl+S enregistre. Le séparateur des colonnes se déplace à la souris ou
  avec les flèches après Tab ; double-clic pour revenir à moitié.

Rust valide l'instantané structuré, génère un sous-ensemble Typst et le compose
avec le monde, les polices, paquets et modèle personnel du cœur. Les réponses
de rendu périmées sont refusées. Un contenu invalide reste éditable et n'écrase
pas le dernier aperçu valide.

Les exports comprennent PDF et projet Typst autonome. Les écritures de sources
de l'adaptateur sont expérimentales : le projecteur de production ne connaît pas
encore les tableaux et notes. Cet essai conserve toutes les ressources de la session,
y compris celles d'images retirées, pour les retours en arrière et clients différés.

## Sauvegarde expérimentale

`session.json` est écrit atomiquement après validation et compilation. Il contient
le schéma versionné, l'instantané structuré, le Typst validé, l'état binaire Yjs et
les images. Une demande périmée est refusée ; les exports utilisent un nouveau
dossier et n'écrasent pas les précédents.

À la reprise, Rust vérifie source, instantané et ressources ; le client vérifie
que l'état Yjs reconstruit le même document. Les identités CRDT sont conservées,
mais les piles d'annulation propres aux vues ne sont pas persistées. Ce fichier
n'est pas un `.typsto` de production. Aucun import de source Typst arbitraire
ni migration de document existant n'est livré par ce diagnostic.

## Vérifier

```sh
npm test
npm run build
npm run test:rust
npm run check:rust
npm run check:format
```

Les tests de collaboration utilisent deux éditeurs et deux Y.Doc distincts :
initialisation unique, fusion hors ligne, annulation locale, cellules/légendes,
reprise binaire et presse-papier HTML. Les tests Rust vérifient refus, sauvegarde
et véritable compilation/export du corpus avec tableaux, notes et image.
La composition IME du système reste un essai manuel à distinguer des événements
simulés et des tests du navigateur.

Pour les parcours graphiques Chromium et WebKit, avec le **vrai** cœur Rust via
un transport de recette STDIO/SSE (aucun rendu factice) :

```sh
cargo build --locked --manifest-path src-tauri/Cargo.toml
npx playwright install chromium webkit
npm run test:browser
```

Le serveur de test est strictement local et n'est activé qu'en mode `browser-test`.
Les captures et mesures de saisie se trouvent dans `test-results/`. La CI dédiée
compile et teste l'adaptateur sur les trois systèmes ; les parcours de navigateur
s'exécutent sous Linux. Cela ne vaut pas un essai graphique natif sur ces systèmes.

Pour automatiser deux **fenêtres Tauri natives**, fermer l'application puis
rouvrir le fichier enregistré, avec un affichage disponible :

```sh
npm run build
cargo build --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml
npm run test:native
```

La recette crée sa propre session dans `output/web-editor/`. Elle garde deux
rapports, les journaux et le pic de RSS échantillonné du seul processus Rust
(hors processus WebKit, donc pas la consommation totale de l'application).
Le [bilan](../../docs/web-editor-spike-results.md) distingue les résultats
automatisés, les limites et les vérifications manuelles restantes.

## Dépendances et icône

Les versions JavaScript sont exactes et verrouillées dans `package-lock.json` ;
le projet Rust possède son propre `Cargo.lock`. Aucun paquet Universe supplémentaire
n'est introduit. L'icône de diagnostic est générée par `tauri icon` depuis
`crates/app/assets/icons/lucide/pen-line.svg`, sous les licences Lucide présentes
dans ce même répertoire. Les fontes Inter utilisent les fichiers et la licence
du projet ; les polices de composition proviennent du cœur existant.
