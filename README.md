# typstOgraphe

Prototype natif d’édition visuelle de courriers administratifs en Rust,
avec egui/eframe et le moteur Typst embarqué. Il sert à valider le confort de
rédaction, de sélection et de mise en forme avant de construire l’application
complète.

## Lancer

Prérequis : Rust **1.92 ou supérieur**. Typst, les polices et le modèle sont
embarqués. Aucune installation de Typst ni connexion réseau n’est nécessaire
pendant l’utilisation. Cargo télécharge les dépendances lors de la première
compilation. `Cargo.lock` est versionné.

```sh
cargo run --locked
```

L’application ouvre un courrier fictif. Elle propose un en-tête compact, une
zone de rédaction continue et un aperçu de toutes les pages réellement composées
par Typst. La séparation entre les colonnes est redimensionnable.

Sur Linux, installer les bibliothèques de développement X11/Wayland et OpenGL
nécessaires à eframe. Par exemple, sous Debian/Ubuntu :

```sh
sudo apt install build-essential pkg-config libx11-dev libxi-dev libxcursor-dev \
  libxrandr-dev libxinerama-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

La base de code vise macOS, Windows et Linux. Les essais sur cette première
version ont lieu sur macOS. Une compilation et une relecture sur Windows et
Linux restent nécessaires avant de déclarer ces plateformes validées.

## Essayer la rédaction

- Sélectionner un passage, y compris entre deux paragraphes, puis appliquer
  gras, italique, un titre ou une liste. La sélection reste disponible après
  l’utilisation d’un bouton.
- Utiliser `⌘B`, `⌘I`, `⌘K` sur macOS, ou `Ctrl+B`, `Ctrl+I`, `Ctrl+K`.
- Annuler avec `⌘Z` / `Ctrl+Z`. Rétablir avec `⌘⇧Z` / `Ctrl+Y`.
- Ouvrir la palette avec `⌘⇧P` / `Ctrl+Shift+P`. Rechercher une commande,
  utiliser les flèches, puis Entrée.
- En début de paragraphe : `= `, `== `, `=== ` pour les titres, `- ` pour
  les puces, `+ ` ou `1. ` pour une liste numérotée.
- Saisir `*texte*` ou `_texte_` pour convertir la saisie en gras ou italique.
- Entrée sur un élément de liste vide permet de sortir de la liste.
- Modifier les informations du courrier ou changer de présentation. Les
  en-têtes et les coordonnées en marge sont produits par le modèle Typst.

Les accents et les caractères Unicode sont traités avec les mêmes indices de
caractères que les curseurs egui. Le gras et l’italique utilisent de véritables
variantes de police dans l’éditeur.

## Composition et export

Le moteur travaille sur un thread dédié, après une courte pause de saisie
(220 ms). Les demandes d’aperçu obsolètes sont regroupées, et une réponse ancienne
ne remplace pas un aperçu plus récent. Les polices et les sources sont mises
en cache. En cas d’erreur, le dernier aperçu réussi reste visible.

Le bouton **Exporter** produit dans un nouveau dossier :

- `courrier.pdf` et les pages PNG ;
- `main.typ`, dont l’en-tête renseigne `#show: courrier.with(...)` ;
- `contenu.typ`, `modele.typ` et `metadonnees.json`.

Un dossier non vide est refusé pour préserver les fichiers
existants. Pour tester le moteur sans ouvrir de fenêtre :

```sh
cargo run --locked -- --render-demo output/premier-essai
```

Le modèle de `templates/courrier.typ` est entièrement écrit en Typst et expose
la fonction `courrier`, avec des arguments nommés pour les métadonnées et le
corps du document. Les exemples et le catalogue d’organisation sont fictifs.

## Source et périmètre

Le source Typst restreint est la référence pour le corps du document. Le parseur
officiel `typst-syntax` en dérive une projection éditable en mémoire. Les actions
de rédaction mettent à jour le source. Le source est normalisé lors d’une
édition : les styles sont écrits avec `#strong[...]`, `#emph[...]` et
`#link("...")[...]`. Les métadonnées restent séparées et les chaînes sont
échappées avant d’être transmises à Typst.

Le sous-ensemble de ce prototype comprend les paragraphes, les titres 1–3,
le gras, l’italique, les liens, les puces et les listes numérotées simples.
Le corps n’expose jamais de code à l’utilisateur. Le parseur refuse les fonctions
et les expressions calculées qu’il ne sait pas éditer.

Le travail reste en mémoire jusqu’à un export. Ce prototype n’inclut pas encore
la réouverture des exports, l’archive unique du document, l’enregistrement
automatique, les préférences personnelles, l’image de signature, le manifeste
extensible des types de document ni le travail collaboratif. Le repérage de la
sélection dans les pages de l’aperçu reste également à construire.

Les paragraphes vides successifs et les espaces sont normalisés à la relecture
d’un source. L’indentation des listes sur plusieurs lignes et les entrées IME
devront faire l’objet d’essais manuels complémentaires.

## Vérification

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Les tests couvrent la projection du source, les sélections entre paragraphes,
les remplacements Unicode dans le composant egui, les styles superposés,
l’annulation, l’échappement des caractères Typst et des métadonnées, la
compilation des deux présentations, le PDF et le rendu des pages.

Une capture de l’application peut être produite après le premier rendu réussi :

```sh
cargo run --locked -- --capture output/interface.png
```

La fenêtre se ferme après la capture. Les fichiers de `output/` et `target/`
ne sont pas versionnés.
