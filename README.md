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
  les puces, `+ ` ou `1. ` pour une liste numérotée, `/ ` pour une liste de descriptions,
  `> ` pour une citation en bloc.
- Le menu **Liste** regroupe les listes à puces, numérotées et de descriptions.
  Choisir à nouveau le type actif retire la liste. Les trois commandes restent
  accessibles dans la palette.
- Dans une liste de descriptions, chaque élément possède une puce et une étiquette
  encadrée avec un fond et un texte distincts. L’étiquette vide affiche
  **Description** comme texte indicatif. Saisir le terme, puis `Tab` (ou `:` suivi
  d’une espace) pour passer au texte. `Tab` rejoint le début du texte depuis
  n’importe où dans l’élément, même avec une sélection ; `Maj+Tab` revient au terme.
  Entrée crée un nouvel élément avec une étiquette vide ; Entrée sur un élément
  entièrement vide termine la liste.
  Les puces, encadrés, couleurs et textes indicatifs appartiennent uniquement
  à l’éditeur ; le modèle garde la maîtrise de la présentation finale.
- Choisir **Citation en bloc** dans le sélecteur de style ou dans la palette.
  La citation peut couvrir plusieurs paragraphes ; une bordure gauche épaisse
  la repère dans l’éditeur, y compris les lignes repliées. La zone facultative
  **Attribution / source** apparaît sous son texte, dans un style distinct.
  `Tab` rejoint cette zone et `Maj+Tab` revient au début de la citation.
  Entrée poursuit la citation ; sur un paragraphe vide ou dans l’attribution,
  Entrée crée un paragraphe normal après le bloc.
- Saisir `*texte*` ou `_texte_` pour convertir la saisie en gras ou italique.
- Entrée sur un élément de liste vide permet de sortir de la liste.
- Modifier les informations du courrier ou changer de présentation. Les
  en-têtes et les coordonnées en marge sont produits par le modèle Typst.

Les **Informations du courrier** se déplient avec le chevron sous la barre de
mise en forme, dans la colonne de rédaction. Le panneau reste fixé au-dessus du
texte pendant son défilement. Ses champs sont regroupés en Général, Correspondance et
Expéditeur ; sa hauteur est limitée, avec un défilement propre pour garder le
texte accessible. Les modifications actualisent l’aperçu et sont enregistrées
dans le courrier. La commande Informations du menu Affichage ou de la palette
permet également de déplier ou replier ce panneau, sans bloquer la rédaction.

**Général** regroupe le type du document (en lecture seule), le modèle, le format
du papier, l’orientation, la langue du document et les puces des listes.
Le choix du modèle remplace
le modèle embarqué par l’un des deux modèles fournis, en conservant le texte et
les métadonnées ; les puces reviennent au mode automatique de la nouvelle charte.
Les formats proposés sont A3, A4, A5, Lettre US et Légal US,
avec les orientations portrait et paysage. Les langues proposées sont le
français, l’anglais, l’allemand, l’espagnol, l’italien, le portugais et le néerlandais.
Un nouveau courrier utilise A4, portrait et français, indépendamment de la langue
de l’interface. La langue du document règle la composition Typst et les libellés
du modèle ; elle ne traduit pas le texte ou les métadonnées saisis.

Les accents et les caractères Unicode sont traités avec les mêmes indices de
caractères que les curseurs egui. Le gras et l’italique utilisent de véritables
variantes de police dans l’éditeur.

L’éditeur emploie une représentation commune à tous les documents, indépendante
du type et du modèle Typst. Pour une taille courante `x`, les titres utilisent
`1.2^(4-h) × x`, où `h` est leur niveau : H1 et H2 sont bleus et gras, H3 est
bleu et italique. Un filet bleu fin occupe la largeur du texte sous les H1,
y compris lorsque leur titre se répartit sur plusieurs lignes. L’espace avant
les titres est plus grand que l’espace après eux.

Le gras est rouille, l’italique vert bouteille ; le gras italique conserve la
couleur rouille. Les teintes sont plus lumineuses en thème sombre. Les titres
et les liens gardent leur couleur bleue ; les liens restent soulignés. Les
filets, les espacements et l’italique visuel des H3 n’ajoutent aucun contenu ou
style au source : le modèle reste responsable de toute la présentation produite
dans l’aperçu et les exports.

## Menus de l’application

Sur macOS, les menus sont natifs, dans la barre système, grâce à **muda**.
Sur Windows et Linux, une barre **egui** les présente en haut de la fenêtre.
Les deux présentations utilisent les mêmes identifiants, libellés traduits,
raccourcis et actions du registre :

- **Fichier** : Nouveau, Ouvrir, Enregistrer, Enregistrer sous, Exporter et Fermer.
- **Édition** : Annuler, Rétablir, Retirer la mise en forme et Palette de commandes.
- **Affichage** : Informations du courrier, avec indication de son état déplié.

Sur macOS, **Quitter typstOgraphe** se trouve dans le menu de l’application ; sur
Windows et Linux, dans Fichier. Fermer utilise `⌘W` / `Ctrl+W`, Quitter `⌘Q` /
`Ctrl+Q`. Dans cette version à une fenêtre, les deux commandes suivent la même
protection des modifications non enregistrées. Les rôles système macOS
(Services, Masquer…) sont fournis par muda.

Annuler/Rétablir agit sur le texte du courrier ou le champ de métadonnées actif.
Les commandes des menus sont suspendues pendant les dialogues. Sur macOS, les
raccourcis installés dans les menus natifs sont retirés du traitement clavier
egui pour éviter une double exécution ; les raccourcis secondaires restent
disponibles. Les raccourcis natifs suivent aussi les personnalisations en mémoire.

Le nom du fichier et l’étoile signalant des modifications apparaissent uniquement
dans le titre de la fenêtre, sans suffixe contenant le nom de l’application.
Le menu application macOS conserve le nom fixe « typstOgraphe ». L’en-tête ne
contient plus le sélecteur de modèle ni la mention de l’organisation ; ces choix
appartiennent au document. Le module `src/menus.rs`
sépare le modèle commun des menus de leur intégration AppKit ; muda n’est une
dépendance que sur macOS et n’ajoute pas de dépendance GTK sous Linux.

## Préférences de rédaction et colonnes

Les **Préférences…** sont accessibles dans le menu **typstOgraphe** sur macOS
(`⌘,`) et dans **Édition** sur Windows/Linux (`Ctrl+,`), ainsi que par la palette.
On peut choisir la police de l’éditeur (Libertinus Serif, New Computer Modern ou
DejaVu Sans Mono), sa taille de 10 à 36 points et le thème Clair, Sombre ou
Système. Le mode Système suit les changements d’apparence du système.
Les variantes grasses et italiques restent disponibles dans les trois polices.
Ces réglages s’appliquent immédiatement ; la mise en page du document reste
définie par le modèle Typst.

La séparation des colonnes se déplace à la souris. Leur proportion est conservée
lors du redimensionnement de la fenêtre ; le double-clic sur la séparation
rétablit deux largeurs égales. L’aperçu s’adapte à l’espace choisi, sans les
anciennes limites de 300 et 1 100 points. Une petite zone de 48 points est gardée
à chaque bord pour que les deux colonnes restent accessibles.

Les préférences et la proportion des colonnes sont enregistrées par eframe
entre les sessions, dans les données locales de l’application. Elles restent
indépendantes du fichier de courrier et ne déclenchent pas d’état « modifié ».

## Types intégrés et modèles Typst

Les types sont définis en Rust, dans `src/document_types/`. Le premier est
**Courrier**, dans `courrier.rs`. Il possède ses métadonnées typées, ses champs
et groupes d’édition, les champs requis pour l’export, son contrat de composition
Typst et ses commandes spécifiques. Il ne charge pas de définition YAML.

`courrier::FIELDS` relie chaque champ à son paramètre Typst et à son édition.
`courrier::CONTRACT` génère l’appel de la fonction publique `courrier`, avec ces
arguments nommés, le dictionnaire commun `options` et le corps du document.
Le moteur Typst compile cet appel : un modèle qui ne fournit pas la fonction
ou ne prend pas en charge ses arguments produit une erreur de composition.

Le registre actif est l’union de `commands::COMMON_COMMANDS` (fichiers, édition,
préférences, styles…) et des commandes du type. Courrier ajoute
`courrier.model.institutional` et `courrier.model.service`. Palette, sélecteurs,
menus et raccourcis utilisent cette même vue ; les conflits de raccourcis sont
vérifiés sur l’ensemble. Les actions spécifiques sont représentées par
`Command::Courrier(courrier::Command)`.

Les modèles restent du pur Typst. Le catalogue Courrier fournit la présentation
institutionnelle et la présentation de service. Cette dernière combine
`templates/courrier.typ` et `templates/courrier-service.typ` en un source
`modele.typ` autonome. Le choix est ainsi enregistré dans le modèle lui-même :
le moteur commun ne transmet plus de drapeau `compact`. Une implémentation
indépendante peut employer des fonctions privées et une mise en page complexe.

Le corps éditable et ses conventions visuelles restent communs dans
`document.rs` et `editor.rs`. Ajouter un autre type demandera du Rust pour ses
données, son contrat, son registre et ses éventuels composants spécifiques ;
ajouter un modèle pour Courrier demande une implémentation du contrat en Typst.

## Registre des commandes et raccourcis

`src/commands.rs` définit le socle des actions publiques et le registre actif : identifiant stable
(par exemple `document.export`), libellé, action, contexte et raccourcis par
défaut. Les menus, la palette, les infobulles et le traitement des touches
utilisent ce même registre.

Cela comprend les fichiers, les informations du courrier, les deux modèles
fournis, les styles de paragraphes, la mise en forme, les liens (dont leur
retrait), l’annulation/rétablissement, les préférences et l’ouverture de la palette. Les boutons
et listes déroulantes déclenchent les mêmes commandes que la palette et les
raccourcis. Les clés de traduction des libellés complets et de leurs variantes
courtes (« Informations », « G », « I »…) sont définies dans le registre.
La commande Fermer est également
accessible dans la palette et garde la protection des modifications non enregistrées.

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
L’écran de personnalisation des raccourcis et l’enregistrement de ces choix seront ajoutés plus
tard ; l’application utilise actuellement les raccourcis par défaut.
Les nouvelles commandes `document.information`,
`courrier.model.institutional`, `courrier.model.service` et
`text.remove_link` n’ont pas de raccourci par défaut ; elles peuvent recevoir
les mêmes personnalisations que les autres commandes.

Les touches de navigation des composants (flèches, Entrée, Échap…) et les
conversions textuelles (`= `, `*texte*`…) restent traitées par l’éditeur et les
dialogues, séparément des commandes personnalisables.

## Langue de l’interface

L’interface utilise **Fluent**, avec `i18n-embed` et `i18n-embed-fl`. Au lancement,
elle choisit une traduction disponible selon les langues préférées du système,
y compris leurs variantes régionales. Le **français est la langue de repli** si
aucune langue demandée n’est disponible, ou si un message manque dans la traduction
choisie. Les catalogues français et anglais sont fournis et embarqués dans le
binaire, même en compilation de développement ; aucun fichier de traduction
externe n’est nécessaire à l’exécution.

Ce choix concerne uniquement l’interface. Il ne change ni le texte du courrier,
ni ses métadonnées, ni son modèle ou les réglages linguistiques Typst du document.
Les identifiants des commandes, les raccourcis et le format `.typsto` restent
indépendants des libellés traduits. Les fenêtres natives conservent les boutons
standard du système ; les détails techniques provenant de Typst ou du système
peuvent rester dans la langue de ces composants. Le choix manuel de la langue
dans les préférences viendra ultérieurement.

Pour contribuer aux traductions :

- `i18n.toml` définit la langue de repli et le dossier des ressources.
- `i18n/fr/typstographe.ftl` contient les messages de référence ;
  `i18n/en/typstographe.ftl` contient leur traduction anglaise.
- Dans le code Rust, utiliser `crate::tr!("identifiant")`, avec des arguments
  nommés si nécessaire. `i18n-embed-fl` vérifie les clés littérales et leurs
  paramètres à la compilation. Les phrases et les pluriels sont définis dans
  Fluent, plutôt que construits à partir de fragments traduits.
- Pour une commande, renseigner ses clés de libellé dans le registre. Les tests
  vérifient leur présence dans chaque catalogue, ainsi que la cohérence des clés
  et paramètres entre français et anglais.
- Une langue supplémentaire s’ajoute dans `i18n/<langue>/typstographe.ftl` ; elle
  est découverte automatiquement après recompilation. Étendre les tests de
  catalogue aux langues ajoutées.

Les tests d’interface utilisent explicitement le français pour être reproductibles
sur tout système. Les tests de négociation créent des chargeurs séparés pour
vérifier le français, l’anglais, les variantes régionales et le repli, sans
modifier la langue globale pendant les tests parallèles.

## Enregistrer et rouvrir un courrier

Le menu **Fichier** propose Nouveau, Ouvrir, Enregistrer et Enregistrer sous,
avec les fenêtres natives de choix de fichier. Les commandes sont aussi
accessibles dans la palette. Raccourcis :

- `⌘N` / `Ctrl+N` : nouveau courrier ;
- `⌘O` / `Ctrl+O` : ouvrir ;
- `⌘S` / `Ctrl+S` : enregistrer ;
- `⌘⇧S` / `Ctrl+Shift+S` : enregistrer sous.

Un courrier est conservé dans un seul fichier **`.typsto`**. Son nom apparaît
dans le titre de la fenêtre ; une étoile signale les
modifications non enregistrées. Avant de fermer la fenêtre, d’ouvrir un autre
fichier ou de créer un courrier, l’application propose d’enregistrer, de ne pas
enregistrer ou d’annuler. Un brouillon incomplet peut être enregistré.

Le fichier est une archive ZIP standard contenant :

- `contenu.typ` : source de référence du corps du courrier ;
- `metadonnees.json` : type des métadonnées et données typées dans `fields`,
  dont les réglages généraux dans `fields.options` ;
- `modele.typ` : copie exacte du modèle utilisé, réutilisée à la réouverture
  et lors de l’export ;
- `main.typ` : en-tête généré à partir des métadonnées ;
- `manifest.json` : identifiant du format, version 5 et type « courrier » ;
- `images/` : images PNG, JPEG ou SVG embarquées dans le courrier.

Le modèle embarqué permet de retrouver la présentation du courrier indépendamment
des modèles fournis par une future version de l’application. L’enregistrement
écrit un fichier temporaire voisin puis remplace atomiquement la cible. Une
archive invalide est refusée sans remplacer le courrier ouvert. Le format
accepte ces cinq fichiers et les images sous des chemins relatifs contrôlés,
pour un total décompressé de 64 Mio maximum (16 Mio par image). Les images sont
lues directement dans l’archive, sans extraction ni dépendance au fichier original.

Le prototype évolue sans exigence de rétrocompatibilité. Le format de fichier et
les signatures des fonctions Typst sont affinés au fil du développement ; seul
le format actuel est pris en charge, sans mécanisme de migration des anciens
fichiers. Cette règle privilégie la simplicité tant que l’application reste
dans sa phase préliminaire, sans utilisateurs.

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

La commande **Exporter** du menu Fichier ou le raccourci `⌘E` sur
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

La fonction reçoit un argument nommé `options`, indépendant des métadonnées
propres au type de document. L’en-tête exporté lui transmet par
exemple `options: (paper: "a4", landscape: false, language: "fr", bullet-marker: auto)`. Le modèle
applique ces valeurs avec `set page(paper: options.paper, flipped: options.landscape)`
et `set text(lang: options.language)`. Il reste responsable des libellés et de
la mise en page dans chaque langue ; aucun code d’interface n’est inséré dans
le modèle. Les réglages sont conservés à l’enregistrement et
utilisés de la même façon dans l’aperçu et l’export PDF/Typst.

## Puces et présentation contrôlée par le modèle

Les informations du document affichent **Puces des listes**. Par défaut,
**Automatique (modèle)** laisse au modèle le choix selon la langue du document,
indépendamment de celle de l’interface. Les deux modèles fournis utilisent un
cadratin en français et une puce ronde dans les autres langues proposées.
Ces conventions sont définies dans `marqueurs-listes` en Typst ; le créateur
peut les modifier pour suivre la charte de son organisation. L’éditeur conserve
ses propres marqueurs visuels, indépendants de la présentation finale.

Le modèle peut déclarer les seules personnalisations qu’il autorise avec une
variable publique Typst `editor-options` :

```typst
// Charte entièrement automatique : aucun choix typographique à faire.
#let editor-options = (bullet-markers: (), custom-bullet-marker: false)

// Variante : autoriser quelques marqueurs, en conservant Automatique par défaut.
#let editor-options = (bullet-markers: ("—", "•", "▪"), custom-bullet-marker: false)
```

`custom-bullet-marker: true` permet en plus la saisie d’un caractère Unicode
visible. Sans déclaration, le modèle reste entièrement automatique. Ces valeurs
sont évaluées par le moteur Typst, sur son thread de rendu, et non extraites par
analyse textuelle. Une déclaration invalide est signalée comme erreur de modèle.
L’application vérifie les autorisations à la composition, y compris à l’export ;
un fichier modifié extérieurement ne peut pas imposer un marqueur interdit.

Le réglage du document est conservé dans `options.bullet_marker` : `null` pour
Automatique, une chaîne pour un choix explicite. L’en-tête transmet respectivement
`options.bullet-marker: auto` ou le caractère comme chaîne Typst. Le modèle
applique la valeur avec `set list(marker: ...)` et reste responsable des retraits,
espacements et styles des listes de descriptions (`terms`). Changer de modèle
rétablit le mode automatique pour appliquer la nouvelle charte.

Les choix d’interface autorisés et les conventions typographiques sont ainsi
définis dans le modèle, sans ajouter une nouvelle commande par forme de puce.

## Citations en bloc

Le contenu utilise l’élément natif Typst `quote`, avec `block: true` et un
argument facultatif `attribution` contenant du texte, des styles et des liens :

```typst
#quote(block: true, attribution: [Service des équipements, _Rapport_])[
  Premier paragraphe cité.

  Deuxième paragraphe cité.
]
```

Le modèle définit les guillemets, les retraits, la typographie et la présentation
de l’attribution avec ses règles `set` / `show`. Les deux modèles fournis ont
une règle `show quote.where(block: true)` : texte en italique, retrait et filet
fin, attribution alignée à droite. L’éditeur conserve sa propre présentation
commune, avec une bordure plus large et un champ d’attribution plus petit.
Les textes indicatifs de l’éditeur ne sont jamais ajoutés au source ou au PDF.

L’attribution est un seul contenu libre ; auteur, titre, date, page ou lien
peuvent y être réunis. Le prototype prend en charge une attribution sur un seul
paragraphe, les citations sans attribution et les citations successives. Il
n’expose pas encore les références bibliographiques, les citations dans le fil
du texte ni les blocs imbriqués à l’intérieur d’une citation. Les choix visuels
du rendu se règlent dans le modèle.

La correspondance entre caractères visibles et source reste exacte pour le
texte et l’attribution, même si l’attribution précède le corps dans l’appel Typst.
La sélection, l’annulation, l’enregistrement et la synchronisation avec l’aperçu
utilisent ces mêmes correspondances.

## Images et légendes

**Insertion → Insérer une image…**, la palette de commandes et le dépôt d’un
fichier dans la fenêtre permettent d’ajouter une image PNG, JPEG ou SVG à la
position de rédaction. L’image est copiée dans le courrier ; déplacer ou supprimer
son fichier d’origine ne change donc pas le document enregistré.

Dans l’éditeur, chaque image forme un bloc avec une vignette proportionnelle
et une légende facultative juste dessous, à l’intérieur du même cadre. Un filet
fin centré sur 80 % de la largeur sépare l’image de la légende ; le cadre suit
la hauteur de la légende quand elle occupe plusieurs lignes.
**Ajouter une légende…** est un texte
indicatif, jamais enregistré dans le source. La légende accepte les styles et
les liens. Un clic sur l’image la sélectionne et affiche **Remplacer…**,
**Informations** et **Supprimer**. Remplacer conserve la légende et le texte
alternatif ; supprimer le bloc retire aussi sa légende. Ces actions s’annulent
et se rétablissent avec l’historique du document.

`Tab` rejoint le début de la légende. `Entrée` poursuit la rédaction dans un
paragraphe normal après l’image. Au début de la légende, `Retour arrière`
sélectionne l’objet ; une seconde pression supprime le bloc. L’éditeur reste
une zone de sélection continue, avec un caractère objet interne par image.
Ce caractère n’est jamais exporté dans le source Typst.

Le dialogue **Informations** contient un **Texte alternatif**, indépendant de
la légende, transmis à l’image pour les lecteurs d’écran. Il n’apparaît pas dans
la composition imprimée. Le source de référence utilise les éléments natifs :

```typst
#figure(
  image("images/image-1.svg", alt: "Trois arbres bordent un chemin."),
  caption: [Le *parc* municipal.],
)
```

Sans légende, `caption` est omis ; sans texte alternatif, `alt` vaut `none`.
Le sous-ensemble éditable ne contient pas de réglages de taille ou de placement
par image : ces choix relèvent du modèle. Les modèles fournis utilisent des
images centrées, limitées au cadre de composition, des légendes plus petites
et sans numérotation. Ils peuvent être remplacés par des modèles qui définissent
d’autres règles `set image`, `set figure` et `show figure.caption`.

Une `figure` Typst reste dans le flux par défaut (`placement: none`). Un modèle
peut la rendre flottante avec `placement: top`, `bottom` ou `auto`, sans modifier
le contenu du document. Le courrier fourni garde le placement dans le flux.
La navigation dans les deux sens et le surlignage de sélection concernent aussi
les images et leur légende.

À chaque enregistrement et export, seules les ressources encore référencées
par le contenu ou nécessaires au modèle sont écrites. Une ancienne image
remplacée ou supprimée reste en mémoire pour **Annuler / Rétablir**, y compris
après un enregistrement ; elle n’alourdit plus le fichier ni sa limite de taille.
L’historique n’est pas enregistré et disparaît à la fermeture du document.

Les chemins d’images déclarés explicitement dans les chaînes Typst du modèle
sont conservés, même dans une variante inactive. Pour les chemins calculés,
le nettoyage vérifie avec Typst que retirer des ressources laisse inchangées
la compilation, les pages et les métadonnées dans la configuration courante.
Un modèle à variantes qui calcule des chemins doit donc aussi déclarer en entier
les chemins des ressources à garder pour ses autres configurations, par exemple
dans une liste Typst. Si le modèle ne compile pas, l’enregistrement conserve
prudemment ses ressources : un problème de composition ne doit pas empêcher
de sauvegarder le travail ni provoquer la perte d’une image.

L’export contient le dossier `images/` nécessaire à la recompilation des sources,
en plus du PDF, des PNG de pages et des fichiers Typst. L’import de modèles avec
des ressources supplémentaires, le recadrage, les renvois à des figures et le
collage d’images depuis le presse-papiers restent à développer.

## Source et périmètre

Le source Typst restreint est la référence pour le corps du document. Le parseur
officiel `typst-syntax` en dérive une projection éditable en mémoire. Les actions
de rédaction mettent à jour le source. Le source est normalisé lors d’une
édition : les styles sont écrits avec `#strong[...]`, `#emph[...]` et
`#link("...")[...]`. Les métadonnées restent séparées et les chaînes sont
échappées avant d’être transmises à Typst.

Le sous-ensemble de ce prototype comprend les paragraphes, les titres 1–3,
le gras, l’italique, les liens, les puces, les listes numérotées, les listes de descriptions simples et les
citations en bloc avec attribution facultative, et les images avec légende et
texte alternatif.
Le corps n’expose jamais de code à l’utilisateur. Le parseur refuse les fonctions
et les expressions calculées qu’il ne sait pas éditer.

L’enregistrement est manuel. Ce prototype n’inclut pas encore l’import des
anciens exports en dossier, l’enregistrement automatique, les préférences
personnelles, l’image de signature, les types de document supplémentaires ni le travail collaboratif.

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
Ils couvrent aussi les réglages généraux, les dimensions réelles des cinq
formats dans les deux orientations, et la langue des éléments de texte Typst.
Les tests du registre vérifient les contextes, les modificateurs sur macOS et
Windows/Linux, les conflits, les remplacements et la désactivation de raccourcis,
ainsi que la cohérence entre le raccourci actif et son affichage.
Les tests des menus vérifient leur répartition par plateforme, l’export par le
menu egui, la protection de Quitter, les accélérateurs natifs, l’absence de double
déclenchement et l’annulation/rétablissement dans les métadonnées via la palette.

Une capture de l’application peut être produite après le premier rendu réussi :

```sh
cargo run --locked -- --capture output/interface.png
```

La fenêtre se ferme après la capture. Les fichiers de `output/` et `target/`
ne sont pas versionnés.
