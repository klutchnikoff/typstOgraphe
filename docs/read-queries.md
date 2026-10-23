# Lectures du rendu et des compteurs

Lot de phase 2 réalisé le 24 octobre 2026. Les opérations sont décrites dans le
[catalogue généré](commands.md) et ses [schémas JSON](commands.json).
Elles utilisent `Engine::query` / `query_call` ; les adaptateurs Rust partagent
leurs méthodes typées. Elles ne modifient ni sélection, ni contexte de frappe,
ni révision, ni historique, ni fichier.

## Compteurs

`document.statistics` prend une `target` facultative :

- absente : totaux du document, sans compteur de sélection ;
- `current` : sélection publiée détenue par le moteur ;
- `all` : document entier également dans le compteur de cible ;
- `selection` : sélection logique explicite, avec garde de document/révision.

La réponse contient `guard`, `document` et `target`. Chaque compteur fournit
`words`, `signs`, `signs_without_spaces`. Une cible réduite à un curseur renvoie
`target: null`. Les cibles périmées ou étrangères sont refusées ; aucun document
ouvert produit `no_document`.

Le comptage conserve `unicode-segmentation` et les conventions existantes :
mots Unicode, signes en graphèmes étendus, espaces inclus dans `signs`, exclus
de `signs_without_spaces`. Les retours, contrôles, tabulations structurelles et
objets images ne comptent pas comme signes. Les légendes, attributions, termes
et définitions sont du texte rédigé et comptent ; les métadonnées, propriétés
alternatives d’image, signatures et textes du modèle sont exclus.

Les totaux sont mis en cache dans le moteur par identité et révision de document,
la sélection par plage résolue. Le brouillon IME ne fait jamais partie de ces
lectures avant sa validation.

## Référence de rendu

Le worker natif compile un instantané préparé par le moteur. Son enveloppe
conserve la garde complète, aussi dans les réponses en erreur. L’application
refuse une réponse étrangère ou périmée avant d’installer ses textures.

Le moteur détient les correspondances Typst immuables et attribue à chaque rendu
accepté une `RenderReference { guard, id }`. `document.inspect` expose le rendu
courant dans `render: { reference, pages }`, ou `null` si aucun rendu ne correspond
à la session publiée. Les dimensions de `pages` sont en points Typst.

Une requête doit fournir cette référence. Deux compilations d’une même révision
obtiennent des identifiants différents ; seule la dernière acceptée est utilisable.
Une édition rend la référence précédente périmée. Remplacer ou fermer le document
libère le rendu détenu par le moteur. Rien n’est enregistré dans le `.typsto`.

## Localisation et surlignage

`render.locate` prend `render` et `position` :

- `{ direction: "to_preview", target }` résout la cible logique et localise sa
  tête dans le rendu. `target` vaut `current` par défaut. La réponse fournit la
  cible gardée et un éventuel `point: { page, point: [x, y] }` ;
- `{ direction: "to_source", point: { page, point: [x, y] } }` renvoie une cible
  logique réduite à un curseur, ou `target: null` pour un point sans contenu
  éditable. L’interface exécute ensuite `text.select` pour publier la navigation.

`render.selection_boxes` prend `render` et `target` (`current` par défaut).
La réponse contient `render` et `boxes`, une liste de `{ page, corners }`.
Chaque quadrilatère décrit quatre coins après les transformations du modèle.
Une sélection inversée produit les mêmes quadrilatères ; un curseur produit
une liste vide. Une sélection peut traverser plusieurs zones et pages.

Les pages sont indexées à partir de zéro ; les coordonnées sont des points Typst,
indépendants du zoom. Une page inconnue, une coordonnée non finie ou extérieure
à la page produit `invalid_params`. Une référence remplacée, absente ou inconnue
produit `render_unavailable` ; les gardes étrangères/périmées conservent les erreurs
`foreign_document` / `stale_revision`. Une session fermée produit `no_document`.

Les pixels, le zoom, le focus et les défilements restent dans l’interface. Les
cartes de source et les algorithmes de `RenderSync` sont conservés derrière ces
requêtes ; aucun indice global du tampon egui ne devient un paramètre JSON.
Ce lot n’ajoute ni `render.compile` public, ni transport CLI/MCP. Le gel du
catalogue et la relecture exhaustive de l’inventaire restent à faire.
