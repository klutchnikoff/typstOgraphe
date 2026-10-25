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

Une liste complète est un seul widget, y compris ses sous-listes. Les éléments
et niveaux ne reçoivent pas d’enveloppes imbriquées ni de corbeilles individuelles.
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
