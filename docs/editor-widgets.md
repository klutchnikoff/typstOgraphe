# Widgets et bandeaux de l’éditeur — D37, lot 6.4a

Choix validés le 25 octobre 2026 après la maquette interactive « Widgets et
bandeaux ». Ce lot précède la relecture MCP de 6.4b. Les types restent intégrés
en Rust, les modèles en Typst ; la présentation de l’éditeur est indépendante
de celle du modèle.

## Enveloppe et contenu

Gras, italique et liens restent des éléments en ligne. Les listes, descriptions,
citations, images, P.-S. et P.J. ont une enveloppe de widget commune : fond,
bordure, espacements, libellé, boutons monochromes et états. La citation garde
sa bordure gauche, l’image sa miniature et sa légende, la description ses zones
terme/définition. Les annotations facultatives partagent leur comportement tout
en conservant leur disposition spécifique.

Une liste complète est un seul widget. Pour l’instant, seules les listes non
imbriquées sont prises en charge, conformément au choix confirmé le 25 octobre.
Si les sous-listes sont ajoutées ultérieurement, elles resteront dans cette
enveloppe : les éléments et niveaux n’auront pas de cadres imbriqués ni de
corbeilles individuelles.
Le regroupement graphique ne change pas à lui seul le contrat documentaire des
listes ni les paramètres pris en charge. Le moteur reste responsable des
frontières et de la structure du document.

Une corbeille propose la suppression du widget entier. La roue dentée ouvre les
paramètres publics qui n’ont pas de contrôle directement dans le bloc. Si tous
ses paramètres sont déjà visibles, elle est indisponible avec une explication.
Légende, attribution et autres zones directes restent dans le widget. Les choix
typographiques et de présentation imposés par le modèle restent à sa charge.

## Bandeau contextuel unique

Les paramètres, la suppression, les liens et l’orthographe utilisent le même
bandeau interligne. Il prend toute la largeur de la colonne de gauche, au-dessus
du widget ou du passage concerné, et repousse le contenu. Il n’est ni une fenêtre
flottante ni un troisième panneau. Un seul bandeau est ouvert à la fois ; ouvrir
une autre interaction abandonne le réglage non appliqué de la précédente.

Le bandeau partage titre, fermeture, boutons, erreurs et gestion du focus. Son
contenu dépend de l’action : paramètres d’image ou de liste, destination d’un
lien, confirmation de suppression, suggestions orthographiques. Les options
effectivement affichées suivent le contrat public disponible ; une maquette de
champ ne crée pas implicitement un nouveau paramètre Typst.

- Ouvrir, fermer et modifier un réglage provisoire ne publie aucune édition.
- Appliquer vérifie la cible gardée, puis utilise la commande du moteur.
- Annuler/Échap abandonne les réglages et restitue le contexte précédent.
- Supprimer retire tout le widget ; l’historique permet de le rétablir.
- `Ctrl+Retour arrière` supprime directement le widget entier depuis son corps
  ou une annotation, en une étape d’annulation. Sur macOS, `Cmd+Retour arrière`
  est également disponible. Le raccourci appartient au registre de commandes et
  peut être remplacé par les mêmes préférences que les autres raccourcis.
- `Entrée` valide le bandeau. Si un bouton a reçu le focus par `Tab`, il active
  ce bouton ; sinon, il applique les paramètres, confirme la suppression ou
  choisit la première suggestion orthographique disponible. Un champ invalide
  ou une cible périmée reste ouvert avec son erreur.
- La saisie native du champ est traitée avant la validation d’`Entrée`, y compris
  si le dernier caractère et la touche arrivent dans la même frame. Cette touche
  n’insère pas de nouvelle ligne dans le corps du document pendant un bandeau.
- Une cible périmée est signalée, sans appliquer le réglage à un autre passage.
- Pendant la composition IME, ses événements gardent la priorité. Aucun bandeau
  ne force la validation ou l’abandon de la composition.
- L’interaction graphique reste distincte de la sélection et des styles publiés.
  Le focus de ses champs ne déclenche pas les raccourcis de rédaction du corps.

Les raccourcis textuels interactifs (`- `, `1. `, etc.), boutons et palette
rejoignent les mêmes commandes. Collage et appels MCP conservent leurs règles
explicites ; aucune reconnaissance automatique supplémentaire n’est ajoutée ici.

## Placement et portée du lot

Le tampon natif egui conserve ses caractères et curseurs. Les en-têtes et
bandeaux réservent une hauteur dans le placement graphique, sans caractères
factices : source Typst, compteurs, recherche, orthographe, navigation et rendu
continuent à observer le document publié. Le placement doit tenir compte des
lignes repliées, du redimensionnement, des annotations et des images.

La relecture MCP garde le parcours validé : indication discrète, ouverture
volontaire, deux colonnes, différences de contenu et de paramètres, acceptation
ou refus du lot entier, garde revalidée, une étape d’annulation après accord.
Ce lot d’harmonisation n’expose pas encore de mutations MCP rattachées.

## Vérification

Vérifier les gestes natifs et les commandes : suppression complète et annulation,
réglages provisoires/appliqués/périmés, changements de bandeau, retour de focus,
composition IME, liens, orthographe, recherche et sélection traversant des widgets.
Vérifier les thèmes clair/sombre, lignes repliées et colonnes étroites. Les captures
antérieures restent conservées ; les changements visuels intentionnels auront une
recette propre, sans écraser les références historiques.

## Livraison du lot 6.4a — 25 octobre 2026

L’enveloppe visuelle est centralisée dans `editor/widgets.rs`, les interactions
dans `app/bands.rs` et ses modules de peinture/actions. Les anciens dialogues
d’image et de lien et le menu flottant d’orthographe sont retirés. Les en-têtes
communs ont des boutons monochromes, libellés accessibles et info-bulles. Le
bandeau réutilise le tampon egui natif pour ses champs provisoires et réserve
sa place dans le placement des lignes, y compris après repli ou redimensionnement.
Son ouverture demande le défilement nécessaire ; les contrôles restent coupés
aux limites de la colonne.

La suppression d’un widget normalise sa première ligne puis efface son contenu
par un lot atomique du moteur, ou utilise la commande d’image existante. Une
annulation restitue l’ensemble. Les réglages d’image modifient le texte alternatif
ou remplacent le fichier ; la légende reste éditable dans le widget. Le réglage
de liste change son type pour l’ensemble du groupe. Un remplacement d’image
annulé dans le sélecteur conserve le bandeau. Un réglage périmé est refusé.
Le focus d’un champ bloque les commandes documentaires dans les menus natifs,
la palette, la barre et les raccourcis, tout en conservant la saisie native du
champ. L’IME garde notamment la priorité sur Échap.

Ce lot ne crée pas de nouvelles capacités : les listes du moteur sont encore
plates. Le principe d’une enveloppe unique pour une liste avec sous-listes reste
le contrat de leur future prise en charge ; aucun niveau ou numéro de départ
factice n’est proposé. La relecture des lots MCP reste à réaliser en 6.4b.

Vérifications locales sur macOS :

- Suite Rust complète : **493 tests**, dont **8 tests dédiés** aux bandeaux et
  widgets (suppression/annulation, brouillons, péremption, focus, IME, compteurs,
  recherche et redimensionnement de 1 240 à 600 pixels avec lignes repliées).
- **14 tests Python**, `cargo fmt`, Clippy avec avertissements refusés, frontière
  de dépendances du cœur et limite de taille des fichiers : réussis.
- **7 captures natives** de widgets : image, liste sombre, suppression,
  orthographe, lien, descriptions/citation sombre et P.-S./P.J. ; inspection
  visuelle en clair et sombre avec véritable aperçu Typst.
- **8 nouvelles références natives**, puis seconde passe identique octet pour
  octet. Les références précédentes restent intactes. Résultats et journaux dans
  `output/phase6-4a/`, références de ce lot dans `output/phase6-4a/baseline/`.

La recette utilise `--capture-widgets image.png image|list|remove|spelling|link|quote|notes`
(suffixe `-dark` possible), puis `scripts/capture-baseline.sh` avec un dossier
de sortie propre. Les essais IME de ce lot injectent les événements natifs egui ;
ils ne remplacent pas un essai manuel d’une méthode de saisie du système.
Les vérifications Windows/Linux restent celles de la CI.

## Suivi clavier et curseur — 25 octobre 2026

Le retrait d’une liste est maintenant appliqué au paragraphe entier, et non à
son premier caractère : le curseur d’un item vide apparaît au point où son texte
sera saisi. Les lignes repliées gardent ce retrait. Aucun caractère factice n’est
ajouté à la projection ou à la source.

La corbeille et le raccourci utilisent le même constructeur de commande gardée
(`widget_removal_command`) : commande d’image existante ou lot atomique
normalisation/suppression. La nouvelle commande d’interface `edit.remove_widget`
ne modifie pas le catalogue documentaire CLI/MCP. Hors d’un widget, le raccourci
reste disponible pour la suppression native d’un mot ; dans un champ du bandeau,
il édite ce champ sans supprimer son widget. Les événements de composition IME
restent prioritaires sur `Entrée` et `Échap`.

La recette de suivi couvre le curseur vide et le repli, les deux variantes de
raccourci par système, les types de widgets et leurs annotations, l’annulation,
les remplacements de raccourci, la saisie du dernier caractère, le choix d’un
bouton par Tab et la priorité IME. Les captures du suivi sont conservées dans
`output/phase6-4a-keyboard/` ; le scénario `list-empty` (avec suffixe `-dark`
possible) permet d’inspecter le curseur d’un item vide dans l’application native.
La suite complète atteint **501 tests Rust réussis**, avec **14 tests Python**,
formatage, Clippy, frontière de dépendances et contrôle de taille réussis. Cinq
captures de suivi ont été produites ; les huit captures de référence du lot
6.4a sont restées strictement identiques (`baseline/run-gv1udg4k`).
