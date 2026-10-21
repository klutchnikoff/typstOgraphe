# typstOgraphe

Application native d’édition visuelle de documents administratifs en Rust,
avec egui/eframe et le moteur Typst embarqué. Le POC validé évolue vers
l’application : Courrier et Compte-rendu de réunion sont les deux premiers types.
Le projet reste en développement préliminaire.

La [feuille de route](ROADMAP.md) regroupe les chantiers, les priorités proposées,
leurs dépendances et les décisions à prendre ensemble avant chaque étape.
Ce README décrit les fonctionnalités actuellement disponibles.

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

La barre compacte utilise des icônes monochromes Lucide embarquées, nettes à la
densité de l’écran et disponibles hors ligne. **typstOgraphe** apparaît au début
de cette barre ; sur macOS avec les menus natifs, l’ancienne ligne d’en-tête
disparaît. Windows et Linux conservent une ligne réservée aux menus de l’application.
La barre regroupe **Titres**, puis
**Gras / Italique / Lien**, puis **Listes / Citation / Image**. Les deux menus
associent icônes et libellés explicites ; les boutons affichent des info-bulles traduites
avec les raccourcis actifs et possèdent des noms accessibles. Les états actifs
utilisent un fond gris ; un petit trait indique une sélection de styles mixtes.
Le menu Titres permet aussi de revenir au paragraphe normal. La barre réutilise
les commandes du registre, comme les menus de l’application et la palette.

- Sélectionner un passage, y compris entre deux paragraphes, puis appliquer
  gras, italique, un titre ou une liste. La sélection reste disponible après
  l’utilisation d’un bouton.
- Utiliser `⌘B`, `⌘I`, `⌘K` sur macOS, ou `Ctrl+B`, `Ctrl+I`, `Ctrl+K`.
- Annuler avec `⌘Z` / `Ctrl+Z`. Rétablir avec `⌘⇧Z` / `Ctrl+Y`.
- Dans le texte courant, Entrée ajoute un retour simple, traité comme une
  espace par Typst dans le même paragraphe. Deux retours créent un nouveau
  paragraphe. Les retours supplémentaires se limitent à une seule ligne vide,
  repérée par un petit **¶** discret dans l’éditeur. Ce repère visuel reste
  absent du contenu, du comptage et des exports.
- Ouvrir la palette avec `⌘⇧P` / `Ctrl+Shift+P`. Rechercher une commande,
  utiliser les flèches, puis Entrée.
- En début de paragraphe : `= `, `== `, `=== ` pour les titres, `- ` pour
  les puces, `+ ` ou `1. ` pour une liste numérotée, `/ ` pour une liste de descriptions,
  `> ` pour une citation en bloc.
- Le menu **Listes** regroupe les listes à puces, numérotées et de descriptions.
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
- Le bouton **Citation en bloc**, séparé des titres, active ou retire la citation.
  La commande reste accessible dans la palette.
  La citation peut couvrir plusieurs paragraphes ; une bordure gauche épaisse
  la repère dans l’éditeur, y compris les lignes repliées. La zone facultative
  **Attribution / source** apparaît sous son texte, dans un style distinct.
  `Tab` rejoint cette zone et `Maj+Tab` revient au début de la citation.
  Entrée poursuit la citation ; sur une ligne vide ou dans l’attribution,
  Entrée crée un paragraphe normal après le bloc.
- Saisir `*texte*` ou `_texte_` pour convertir la saisie en gras ou italique.
- Entrée sur un élément de liste vide permet de sortir de la liste.
- Modifier les informations du courrier ou changer de présentation. Les
  en-têtes et les coordonnées en marge sont produits par le modèle Typst.

La ligne **Paramètres du courrier :** sous la barre de mise en forme propose
trois boutons egui : **Général**, **Correspondance** et **Expéditeur**. Un clic
affiche les champs de la rubrique ; un autre bouton change de rubrique et un
second clic sur le bouton actif replie le panneau. Le libellé devient
**Paramètres :** quand la colonne est étroite. Le panneau reste fixé au-dessus du
texte, avec une hauteur limitée et un défilement propre à chaque rubrique.
Les modifications actualisent l’aperçu et sont enregistrées dans le courrier.
La commande Informations du menu Affichage ou de la palette ouvre également la
dernière rubrique consultée ou replie le panneau, sans bloquer la rédaction.

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
couleur rouille. En thème clair, les teintes sont soutenues et saturées pour se
distinguer du texte courant ; en thème sombre, elles sont plus lumineuses. Les titres
et les liens gardent leur couleur bleue ; les liens restent soulignés. Les
filets, les espacements et l’italique visuel des H3 n’ajoutent aucun contenu ou
style au source : le modèle reste responsable de toute la présentation produite
dans l’aperçu et les exports.

## Rechercher et remplacer

**Édition → Rechercher…**, la palette ou `⌘F` / `Ctrl+F` ouvre une barre compacte
au-dessus du texte, sous les informations du document. La recherche est exacte,
insensible à la casse par défaut ; **Aa** permet de respecter la casse. Les accents
restent significatifs. Il n’y a ni recherche approximative ni syntaxe de regex :
`.*`, par exemple, recherche ces deux caractères littéraux.

La saisie surligne immédiatement toutes les occurrences dans l’éditeur, sans
déplacer son curseur ou son défilement. Un clic sur un résultat sélectionne ce
seul passage ; un clic sur un autre remplace la sélection précédente. Les autres
résultats gardent leur fond de recherche. La sélection native réutilise le
défilement et le surlignage existants dans l’aperçu Typst.

Les flèches de la barre, Entrée / Maj+Entrée dans le champ de recherche et les
commandes Occurrence suivante / précédente parcourent les résultats en boucle.
Les raccourcis sont `⌘G` / `⌘⇧G` sur macOS, `F3` / `Maj+F3` sous Windows/Linux.
Échap ou la croix ferme la barre et retire ses surbrillances en conservant la
dernière sélection.

Le chevron déplie **Remplacer par…**. On peut également utiliser **Édition →
Rechercher et remplacer…**, `⌘⌥F` sur macOS ou `Ctrl+H` sous Windows/Linux.
**Remplacer** agit sur l’occurrence sélectionnée puis rejoint la suivante ;
**Tout remplacer** traite les résultats présents en une seule opération annulable.
Un remplacement vide supprime les passages trouvés. Le nouveau texte reprend
le style du premier caractère remplacé, y compris ses éventuels attributs de lien.

Le périmètre couvre le corps éditable, les titres, termes de description, légendes
et attributions. Les styles ne coupent pas une occurrence. Les métadonnées, textes
alternatifs, chemins d’images et contenus ajoutés par le modèle sont exclus.
Cette première version recherche et remplace du texte sur une seule ligne,
sans franchir les séparateurs de structures ; elle recherche dans tout le corps,
sans option de restriction à la sélection ni de mots entiers.

`src/search.rs` réutilise le moteur Unicode de `regex`, déjà utilisé par Typst,
en échappant systématiquement la saisie. Les résultats sont mis en cache par
révision, texte recherché et option de casse. Les remplacements utilisent la
session et son historique commun ; les surbrillances ne modifient ni le source,
ni les exports, ni l’état enregistré.

## Statistiques de rédaction

La barre d’état affiche les **mots et signes du document**, puis ceux de la
**sélection** lorsqu’elle existe. Survoler chaque compteur indique son nombre
de signes **hors espaces** et les conventions de comptage. Les messages
d’ouverture ou d’enregistrement restent visibles à côté.

Le compteur Rust commun utilise `unicode-segmentation` (Unicode UAX #29) :
`l’été` compte pour un mot, `porte-monnaie` pour deux et `12 500` pour un seul.
Un signe est un graphème Unicode : un accent combiné ou un emoji composé
reste une unité. Les espaces, y compris insécables et fines insécables, sont
compris dans le total des signes ; le détail hors espaces les exclut.
Les sauts de paragraphe, caractères de contrôle, tabulations structurelles des
descriptions et objets internes des images ne sont pas des signes rédigés.

Le périmètre comprend le texte éditable, les titres, légendes et attributions.
Les métadonnées, coordonnées et éléments ajoutés par le modèle, textes alternatifs,
puces automatiques et textes indicatifs restent hors décompte. Les statistiques
ne dépendent ni du rendu ni de la langue de l’interface. Elles sont mises en cache
jusqu’à la prochaine édition ou au changement de sélection, et ne sont pas enregistrées.

## Menus de l’application

Sur macOS, les menus sont natifs, dans la barre système, grâce à **muda**.
Sur Windows et Linux, une barre **egui** les présente en haut de la fenêtre.
Les deux présentations utilisent les mêmes identifiants, libellés traduits,
raccourcis et actions du registre :

- **Fichier** : Nouveau, Ouvrir, Enregistrer, Enregistrer sous, Exporter et Fermer.
  **Récupérer des documents…** permet de retrouver les copies laissées pour plus tard.
- **Édition** : Annuler, Rétablir, Retirer la mise en forme, Rechercher,
  Rechercher et remplacer, Occurrence suivante / précédente et Palette de commandes.
- **Affichage** : Informations du document et Importer un modèle…

Sur macOS, **Quitter typstOgraphe** se trouve dans le menu de l’application ; sur
Windows et Linux, dans Fichier. Fermer utilise `⌘W` / `Ctrl+W`, Quitter `⌘Q` /
`Ctrl+Q`. Dans cette version à une fenêtre, les deux commandes suivent la même
protection des modifications non enregistrées. Les rôles système macOS
(Services, Masquer…) sont fournis par muda.

Annuler/Rétablir suit un historique commun au corps, aux métadonnées et aux
changements de modèle. La saisie rapprochée dans un même champ est regroupée ;
changer de champ, de modèle ou de catégorie d’édition crée une autre transaction.
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
**Courrier**, dans `courrier.rs`, suivi de **Compte-rendu**, dans `compte_rendu.rs`.
Chaque type possède ses métadonnées typées, ses champs
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
`Command::Courrier(courrier::Command)` et
`Command::CompteRendu(compte_rendu::Command)`. Les raccourcis personnalisés des
commandes d’un autre type sont conservés pendant un changement de document,
sans devenir actifs dans le nouveau type.

Les modèles restent du pur Typst. Le catalogue Courrier fournit la présentation
institutionnelle et la présentation de service. Cette dernière combine
`templates/courrier.typ` et `templates/courrier-service.typ` en un source
`modele.typ` autonome. Le choix est ainsi enregistré dans le modèle lui-même :
le moteur commun ne transmet plus de drapeau `compact`. Une implémentation
indépendante peut employer des fonctions privées et une mise en page complexe.

**Fichier → Nouveau** propose Courrier ou Compte-rendu. Le choix crée un vrai
document vierge, sans les coordonnées fictives de la démonstration, et fixe son
type. Le compte-rendu propose l’intitulé, la date, le lieu ou la visioconférence,
la rédaction, les personnes présentes et excusées ; son corps est libre.
Seul l’intitulé est obligatoire pour son export. Son modèle initial autonome
est `templates/compte-rendu.typ`, avec les libellés français et anglais (repli
français pour les autres langues de document).

**Affichage → Importer un modèle…**, également dans la palette, accepte un
fichier `.typ` autonome de 1 Mio maximum. L’application vérifie que la fonction
publique attendue existe et que son appel avec les métadonnées et le contenu
courants compile. Une erreur conserve le modèle et le document précédents.
Cette validation vérifie le contrat d’appel, pas le respect visuel d’une charte.
Les images référencées doivent déjà être embarquées dans le document ; les
imports de fichiers externes et de paquets ne sont pas résolus. Le modèle choisi
est copié dans le document et son remplacement peut être annulé. La gestion
d’une bibliothèque persistante de modèles d’organisation reste à développer.

Le contrat Courrier expose `courrier(expediteur, fonction, service, email,
destinataire, adresse, objet, reference, lieu, date, options, body)` ; Compte-rendu
expose `compte-rendu(titre, date, lieu, redacteur, participants, excuses, options,
body)`. Les champs et `options` sont nommés, `body` est positionnel. Les
constructions communes (titres, listes, `strong`, `emph`, `link`, `quote`,
`figure` et `image`) sont celles de Typst ; le modèle en définit les règles de
composition. Les tests des types compilent aussi les citations, descriptions et
images, puis vérifient l’archive et l’export de chaque type.

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
- `manifest.json` : identifiant du format, version 5 et type du document ;
- `images/` : images PNG, JPEG ou SVG embarquées dans le courrier.

Le modèle embarqué permet de retrouver la présentation du courrier indépendamment
des modèles fournis par une future version de l’application. L’enregistrement
utilise `atomic-write-file` : écriture temporaire voisine, synchronisation puis
remplacement atomique de la cible. Une
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

## Récupération après incident

Toutes les cinq secondes, si le document est modifié, une copie de récupération
est demandée en arrière-plan. Une révision déjà copiée n’est pas réécrite. Cela
protège aussi un document qui n’a encore jamais été enregistré. Les dernières
secondes de saisie ou une écriture encore en cours peuvent ne pas être présentes
dans la dernière copie terminée.

La copie contient le source, les métadonnées, le modèle et les images, mais pas
l’historique Annuler/Rétablir. Elle reste distincte du fichier utilisateur et
n’attend pas que le modèle produise un rendu réussi. Pour éviter toute perte
sans recomposer le document, les copies de récupération conservent prudemment
les ressources en mémoire ; les fichiers enregistrés et les exports gardent
leur nettoyage habituel des images inutilisées.

Au lancement, **Documents à récupérer** présente les copies des sessions arrêtées,
avec leur nom lorsqu’il existe et leur date en UTC :

- **Récupérer** ouvre un document marqué comme récupéré et non enregistré.
  Enregistrer propose un nouveau nom et passe par Enregistrer sous, pour éviter
  d’écraser implicitement le fichier d’origine.
- **Plus tard**, la croix ou Échap conserve les copies sans date d’expiration.
  **Fichier → Récupérer des documents…** ou la palette permet de les retrouver
  pendant la session courante.
- **Supprimer la copie** retire uniquement la copie choisie.

La copie active est retirée après un enregistrement réussi, un retour à l’état
enregistré ou un abandon explicite des modifications. Annuler une fermeture ou
échouer à enregistrer la conserve. Récupérer un autre document passe par la même
protection des modifications en cours que Nouveau et Ouvrir. Une nouvelle
interruption pendant la reprise conserve la copie, sans en créer un doublon.

`src/recovery.rs` possède le worker d’écriture et les verrous de session. Les
autres instances encore ouvertes sont ignorées. Les archives de récupération
réutilisent la lecture/écriture commune de `storage.rs`, avec une entrée privée
pour leur nom et leur horodatage. Une copie illisible est signalée et reste
supprimable explicitement. Les erreurs de protection apparaissent dans la barre
d’état, avec leurs détails au survol.

Les copies sont stockées sous `recovery/` dans le dossier choisi par
`eframe::storage_dir("typstOgraphe")` :

- macOS : `~/Library/Application Support/typstOgraphe/recovery/` ;
- Windows : `%APPDATA%/typstOgraphe/data/recovery/` ;
- Linux : `$XDG_DATA_HOME/typstographe/recovery/`, ou
  `~/.local/share/typstographe/recovery/` si cette variable n’est pas définie.

L’intervalle utilise les mécanismes de persistance et de rafraîchissement d’eframe ;
les écritures documentaires utilisent `atomic-write-file`, séparément du stockage
des préférences. L’enregistrement automatique directement dans le fichier de
travail et les conflits avec des modifications externes restent à développer.

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

- `courrier.pdf` ou `compte-rendu.pdf`, selon le type, et les pages PNG ;
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

## Annotations facultatives dans l’éditeur

Les légendes d’images et les attributions de citations partagent une notion
d’annotation facultative, décrite dans `src/annotations.rs`. Chaque structure
déclare son corps principal, la plage de son annotation et son texte indicatif
traduit. La navigation, le focus natif, le défilement vers la cible, le style du
texte et l’affichage des textes indicatifs sont communs :

- `Tab` rejoint le début de l’annotation, même vide ;
- `Maj+Tab` revient au début du corps cité ou sélectionne l’objet image ;
- `Entrée` dans l’annotation poursuit la rédaction dans un paragraphe normal
  après toute la structure, en conservant son contenu ;
- le texte indicatif disparaît à la saisie et reste absent du source Typst.

Les présentations conservent leurs règles propres : attribution sous le texte
cité, légende dans le cadre de l’image. De futures structures pourront déclarer
leur annotation en réutilisant ce comportement.

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
la hauteur de la légende quand elle occupe plusieurs lignes. La légende
reprend le style de l’attribution des citations : texte plus petit, italique et
bleu atténué, indépendant du modèle Typst.
**Ajouter une légende…** est un texte
indicatif, jamais enregistré dans le source. La légende accepte les styles et
les liens. Un clic sur l’image la sélectionne et affiche **Remplacer…**,
**Informations** et **Supprimer**. Remplacer conserve la légende et le texte
alternatif ; supprimer le bloc retire aussi sa légende. Ces actions s’annulent
et se rétablissent avec l’historique du document.

La légende utilise la navigation commune aux annotations facultatives.
`Entrée` quitte également le bloc quand l’objet image est sélectionné.
Au début de la légende, `Retour arrière`
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

L’enregistrement du fichier utilisateur est manuel ; des copies de récupération
sont produites automatiquement. Ce prototype n’inclut pas encore l’import des
anciens exports en dossier, l’enregistrement automatique, les préférences
personnelles, l’image de signature ni le travail collaboratif.

Les paragraphes vides successifs et les espaces sont normalisés à la relecture
d’un source. L’indentation des listes sur plusieurs lignes et les entrées IME
devront faire l’objet d’essais manuels complémentaires.

## Session et opérations d’édition

`src/session.rs` possède le document, les métadonnées, le modèle, la révision,
l’état enregistré et l’historique commun. Ses sélections utilisent des indices de
caractères, sans curseurs egui. `App` conserve la présentation, le focus et le
pilotage des fichiers et du rendu. L’adaptateur `TextBuffer` et les conversions
de curseurs vivent dans `editor.rs` ; la projection et ses mutations dans
`document.rs` ne dépendent plus d’egui.

Les opérations de bloc raisonnent sur les paragraphes : un retour simple ne
coupe pas une sélection de lien. Transformer un paragraphe sur plusieurs lignes
en titre ou en élément de liste joint ces lignes par des espaces, en conservant
les indices de caractères. Les lignes vides bornent les paragraphes ; les titres,
éléments de liste, images et annotations gardent leurs frontières propres.

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

Le workflow `.github/workflows/ci.yml` exécute formatage, tests et Clippy sur
les runners GitHub macOS, Windows et Linux à chaque push et pull request.
La réussite de ces contrôles ne remplace pas les essais manuels sur chaque OS
(presse-papiers, saisie IME, dialogues, menus et accessibilité).

Une capture de l’application peut être produite après le premier rendu réussi :

```sh
cargo run --locked -- --capture output/interface.png
```

La fenêtre se ferme après la capture. Les fichiers de `output/` et `target/`
ne sont pas versionnés.

Pour vérifier le dialogue de récupération avec un dossier de copies de test :

```sh
cargo run --locked -- --capture-recovery output/recuperation.png DOSSIER_RECUPERATION
```

Ce mode utilise uniquement le dossier indiqué pour la récupération ; les captures
ordinaires n’accèdent pas aux copies personnelles. Les tests couvrent également
une sortie brutale d’un sous-processus, les verrous, la récupération différée,
l’échec d’écriture, la reprise sans écrasement et la suppression explicite.

Pour capturer la recherche et sa première occurrence sélectionnée :

```sh
cargo run --locked -- --capture-search output/recherche.png salle
```

Les tests couvrent la recherche littérale Unicode, les changements de casse,
les clics successifs avec une seule sélection et la synchronisation de l’aperçu,
les fonds dans les deux thèmes, les légendes et attributions, les remplacements
de tailles différentes et l’annulation/rétablissement du remplacement global.
