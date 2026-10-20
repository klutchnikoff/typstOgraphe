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

## Registre des commandes et raccourcis

`src/commands.rs` centralise les commandes dans une table : identifiant stable
(par exemple `document.export`), libellé, action, contexte et raccourcis par
défaut. Les menus, la palette, les infobulles et le traitement des touches
utilisent ce même registre.

Le modificateur principal correspond à `⌘` sous macOS et à `Ctrl` sous Windows
et Linux. Des valeurs par plateforme sont possibles ; par exemple, le raccourci
de rétablissement affiché est `⌘⇧Z` sur macOS et `Ctrl+Y` ailleurs. Les raccourcis
de mise en forme s’appliquent lorsque l’éditeur a le focus. Les fenêtres de
dialogue suspendent les raccourcis de commande.

Le moteur accepte déjà une couche de personnalisation en mémoire. Une commande
absente conserve ses raccourcis par défaut ; une liste vide les désactive ; une
liste non vide les remplace. Exemple de données pour les futures préférences :

```json
{
  "document.export": [{ "key": "E", "primary": true, "shift": true }],
  "text.bold": []
}
```

Ici, l’export serait affecté à `⌘⇧E` / `Ctrl+Shift+E` et le gras resterait
accessible par les boutons et la palette. Les conflits entre commandes sont
refusés avant activation, en tenant compte des équivalences de modificateurs
sur chaque plateforme. Les touches de fonction sont également acceptées.
L’écran des préférences et l’enregistrement de ces choix seront ajoutés plus
tard ; l’application utilise actuellement les raccourcis par défaut.

Les touches de navigation des composants (flèches, Entrée, Échap…) et les
conversions textuelles (`= `, `*texte*`…) restent traitées par l’éditeur et les
dialogues, séparément des commandes personnalisables.

## Enregistrer et rouvrir un courrier

Le menu **Fichier** propose Nouveau, Ouvrir, Enregistrer et Enregistrer sous,
avec les fenêtres natives de choix de fichier. Les commandes sont aussi
accessibles dans la palette. Raccourcis :

- `⌘N` / `Ctrl+N` : nouveau courrier ;
- `⌘O` / `Ctrl+O` : ouvrir ;
- `⌘S` / `Ctrl+S` : enregistrer ;
- `⌘⇧S` / `Ctrl+Shift+S` : enregistrer sous.

Un courrier est conservé dans un seul fichier **`.typsto`**. Son nom apparaît
dans l’en-tête et dans le titre de la fenêtre ; une étoile signale les
modifications non enregistrées. Avant de fermer la fenêtre, d’ouvrir un autre
fichier ou de créer un courrier, l’application propose d’enregistrer, de ne pas
enregistrer ou d’annuler. Un brouillon incomplet peut être enregistré.

Le fichier est une archive ZIP standard contenant :

- `contenu.typ` : source de référence du corps du courrier ;
- `metadonnees.json` : champs du courrier ;
- `modele.typ` : copie exacte du modèle utilisé, réutilisée à la réouverture
  et lors de l’export ;
- `main.typ` : en-tête généré à partir des métadonnées ;
- `manifest.json` : identifiant du format, version 1, type « courrier » et
  choix de présentation.

Le modèle embarqué permet de retrouver la présentation du courrier indépendamment
des modèles fournis par une future version de l’application. L’enregistrement
écrit un fichier temporaire voisin puis remplace atomiquement la cible. Une
archive invalide est refusée sans remplacer le courrier ouvert. Le format v1
accepte uniquement ces cinq fichiers, pour un total décompressé de 16 Mio maximum.
Il ne contient pas encore de pièces jointes ou d’images.

Pour les développeurs : `main.typ` est dérivé du manifeste et des métadonnées ;
une modification manuelle de son en-tête doit rester cohérente avec ces fichiers.
Le contenu du source est préservé à l’ouverture et à l’enregistrement ; les règles
de normalisation lors d’une édition sont décrites plus bas.

On peut également ouvrir un courrier au lancement :

```sh
cargo run --locked -- mon-courrier.typsto
```

## Se déplacer entre le texte et l’aperçu

- Déplacer le curseur dans l’éditeur : l’aperçu défile vers le passage
  correspondant, si celui-ci n’est pas déjà visible.
- Double-cliquer sur un passage de l’aperçu : le curseur revient dans le texte
  et le panneau de rédaction défile pour le rendre visible. Cela fonctionne
  aussi sur le texte des liens.
- Sélectionner du texte dans l’éditeur : une surbrillance bleue translucide
  repère le passage dans l’aperçu, sur toutes les lignes et pages concernées.
  Elle disparaît quand la sélection est vide et suit le redimensionnement du
  panneau. Le PDF exporté conserve sa présentation habituelle.
- Le défilement manuel reste libre. Les coordonnées et les éléments fixes du
  modèle n’ont pas de destination dans le corps du courrier.

Les correspondances suivent les caractères du source Typst et les glyphes du
rendu, y compris sur plusieurs pages et après une mise en forme. Pendant une
modification, la navigation attend le rendu correspondant à la nouvelle version
du document. La navigation et le surlignage sont suspendus si le dernier rendu a
échoué. Une ligature peut être surlignée en entier lorsqu’un de ses caractères
est sélectionné.

## Composition et export

Le moteur travaille sur un thread dédié, après une courte pause de saisie
(220 ms). Les demandes d’aperçu obsolètes sont regroupées, et une réponse ancienne
ne remplace pas un aperçu plus récent. Les polices et les sources sont mises
en cache. En cas d’erreur, le dernier aperçu réussi reste visible.

Le bouton **Exporter**, la commande du menu Fichier ou le raccourci `⌘E` sur
macOS / `Ctrl+E` sur Windows et Linux ouvre la fenêtre d’export. La commande est
aussi accessible dans la palette. L’export produit dans un nouveau dossier :

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

L’enregistrement est manuel. Ce prototype n’inclut pas encore l’import des
anciens exports en dossier, l’enregistrement automatique, les préférences
personnelles, l’image de signature, le manifeste
extensible des types de document ni le travail collaboratif.

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
Ils vérifient également la navigation dans les deux sens, les liens, les passages
répétés, un paragraphe réparti sur plusieurs pages, le défilement des deux panneaux
et le refus des correspondances devenues obsolètes.
Le surlignage est vérifié sur les styles, les liens, les ligatures et plusieurs
pages, ainsi qu’après un redimensionnement ou la disparition de la sélection.
Les tests de fichiers vérifient l’aller-retour du source, des métadonnées et d’un
modèle personnalisé, l’export avec le modèle rouvert, le remplacement atomique,
le refus des archives invalides, les erreurs d’écriture et la protection des
modifications non enregistrées.
Les tests du registre vérifient les contextes, les modificateurs sur macOS et
Windows/Linux, les conflits, les remplacements et la désactivation de raccourcis,
ainsi que la cohérence entre le raccourci actif et son affichage.

Une capture de l’application peut être produite après le premier rendu réussi :

```sh
cargo run --locked -- --capture output/interface.png
```

La fenêtre se ferme après la capture. Les fichiers de `output/` et `target/`
ne sont pas versionnés.
