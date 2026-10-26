# Reconstruction de l'interface Tauri

Décision du 26 octobre 2026 : après essai du diagnostic, reconstruire l'interface
avec Tauri, Tiptap/ProseMirror et Yjs, en conservant le cœur Rust. Branche de travail :
`codex/tauri-interface`, issue du diagnostic validé. L'interface egui reste accessible
pendant le portage ; elle sert de référence fonctionnelle.

## Invariants

- Un catalogue Rust pour les capacités, leur contrat et leur disponibilité par type.
  La vue web et MCP consomment ce catalogue. Les widgets ont leur implémentation web,
  les modèles gardent leur implémentation Typst ; cela ne constitue pas deux catalogues.
- Yjs porte le contenu partagé et l'historique des participants. Les métadonnées
  documentaires partagées suivent le même circuit. Les préférences et le profil
  restent globaux et locaux à l'utilisateur.
- Le cœur valide la structure et compose le Typst. Une capacité simplement documentée
  n'est jamais annoncée comme exécutable par un adaptateur qui ne la gère pas.
- Les propositions de l'agent restent des lots à accepter ou refuser explicitement.
  Ni le focus, ni la sélection de l'utilisateur ne sont pilotés par l'agent.
- Interface compacte, menus, palette, onglets de métadonnées, icônes monochromes,
  bandeaux cohérents, raccourcis centralisés, thèmes et internationalisation préservés.
- Plusieurs commits vérifiés et une seule poussée par lot.

## Contrat MCP validé

Les noms restent identiques lorsque leur sens reste identique. Les cibles évoluent
vers des chemins de blocs, cellules et annotations, avec des offsets UTF-16 à
l’intérieur d’une zone de texte. Aucune compatibilité artificielle avec les
positions de la projection plate n’est recherchée. Chaque lot porte la révision
lue par l’agent et attend l’acceptation explicite de l’utilisateur.

## Portage

1. Extraire le schéma structuré et le catalogue vers Rust ; le partager avec MCP.
   Reprendre citations, descriptions, P.-S./P.J., puis tableaux et notes.
2. Reconstruire le cadre d'application : accueil sans document, création guidée par
   les types/modèles disponibles, paramètres du document, préférences/profil,
   fichiers, recherche, palette et statistiques.
3. Raccorder le nouveau modèle aux opérations documentaires, au stockage durable,
   à la récupération et à MCP ; documenter les différences de cibles structurées.
4. Vérifier clavier/IME, sélection, annulation collaborative, synchronisation avec
   l'aperçu et régressions, puis décider du lancement par défaut.

Les capacités déjà présentes sont prioritaires. Les tableaux simples et notes sont
les premières extensions ; les mathématiques et diagrammes restent dans l'inventaire,
sans ajout automatique de dépendances ou d'interface complexe.

## État du portage

Le catalogue `typstographe_core::content::catalogue` définit les 16 capacités
visuelles et leur disponibilité par type. `content::operations::specifications`
ajoute les contrats d'édition et de ciblage pour MCP. Les identifiants et contrats
sont partagés ; le code des widgets ProseMirror et celui de leur mise en page Typst
restent propres à leurs technologies.

- Titres, gras, italique, liens, listes simples, descriptions, citations,
  images avec légende/texte alternatif, P.-S./P.J., tableaux simples et notes.
- Accueil, création depuis les registres de types/modèles, onglets des paramètres,
  préférences et profil/signature globaux, récents, palette et menus natifs macOS.
- Recherche exacte, remplacement, orthographe locale réutilisant le cœur,
  compteurs Unicode document/sélection, thèmes et catalogues Fluent français/anglais.
- Archives sous verrou et écriture atomique, import des anciens courriers,
  export PDF et sources autonomes, récupération du brouillon après inactivité.
- Sélection et navigation dans les deux sens avec le rendu Typst ; les requêtes
  correspondant à un ancien rendu sont ignorées.
- MCP rattaché : découverte, lecture et proposition atomique. Acceptation/refus
  du lot dans l'interface ; une annulation retire le lot accepté. Les fichiers,
  préférences et sélection humaine ne sont pas accessibles en écriture à l'agent.

### Représentations documentaires

Pendant la session, Yjs porte le contenu et les paramètres partagés. Le cœur
valide l'arbre et génère le Typst ; celui-ci n'est plus la projection plate
utilisée pour adresser les éditions. L'archive version 10 contient `document.json`,
`state.yjs`, `contenu.typ`, `main.typ`, métadonnées, modèle et dépendances. Le lecteur
vérifie que le source correspond à l'arbre ; la vue vérifie l'instantané restauré
contre Yjs avant de permettre l'édition. L'export Typst reste utilisable hors ligne.
Les images retirées restent en mémoire pour l'annulation, mais leurs octets ne
figurent pas dans une nouvelle archive ou un export s'ils ne sont plus utilisés.

Le remplacement du document exige une révision à jour et aucune vue déconnectée.
Les autres vues rechargent le même état ; elles ne fusionnent pas deux documents
différents. Un agent perd son invitation lors d'un remplacement.

### Lecture de l'interface

| Before | After | Why |
| --- | --- | --- |
| Barre de diagnostic du prototype | Menus, palette, barre compacte et accueil sans document | Retrouver les parcours de l'application |
| Blocs de présentations différentes | Commandes de paramètres/suppression communes, contenus éditables spécifiques | Réduire les conventions à apprendre |
| Paramètres séparés du contenu partagé | Onglets alimentés par le registre et paramètres dans Yjs | Conserver des vues cohérentes et l'annulation |
| Positions de texte plat pour l'agent | Chemins de blocs/cellules et offsets dans une zone | Protéger les frontières structurées |

### Correctifs après essai de l'interface

| Before | After | Why |
| --- | --- | --- |
| Contour de focus autour d'une sous-zone de texte | Surface éditable sur toute la colonne, marges intérieures et aucun cadre de focus | Pouvoir cliquer et rédiger dans toute la surface disponible |
| Menus ouverts après un clic extérieur | Fermeture sans commande au clic extérieur ou avec Échap ; un seul menu ouvert | Annuler un choix sans modifier le document |
| Nœuds de listes privés des commandes Tiptap | Extensions de listes Tiptap, conservant nos widgets et la limitation sans imbrication | Restaurer les boutons, les raccourcis textuels et Entrée sans réimplémenter les commandes |
| Actions dispersées ou empilées dans certains panneaux | Groupe à droite ; action destructive à gauche du groupe, action principale en dernier | Retrouver une disposition prévisible dans les formulaires, bandeaux et dialogues |

Les barres d'outils, onglets et listes de résultats restent des contrôles de
navigation ou de sélection, distincts des pieds de formulaire. Les icônes des
widgets suivent l'ordre suppression puis paramètres. Le contour de focus reste
visible sur les boutons et les champs ; seule la surface de rédaction en est
dispensée. Les menus partagent leur gestion de fermeture, les panneaux leurs
règles de disposition CSS. La suggestion orthographique déclenchée par Entrée
est mise en évidence à l'extrémité droite.

### Bandeaux B et règles de rédaction — reprise des références validées

Le contrat est celui de [l'audit du chantier L](interface-design-audit.md#1-ce-qui-est-déjà-décidé),
de la [maquette B](mockups/interface-polish.html) et de ses
[compléments livrés](architecture-chantier-l.md). Le premier portage plaçait à tort
les interactions en tête de colonne. Les vérifications fonctionnelles précédentes
ne démontraient pas sa conformité à ces références.

| Before | After | Why |
| --- | --- | --- |
| Paramètres éloignés du widget | En-tête déplié dans son cadre commun, bordure d'accent, sans pointe | Variante B validée |
| Correction en tête de colonne | Bandeau inséré avant la ligne du mot, pointe de 12 px centrée sur celui-ci | Rendre la cible précise, y compris après redimensionnement |
| Correction d'un item sans lien avec son cadre | Bandeau interligne de la largeur du widget, dans celui-ci | Conserver la délimitation du bloc |
| Logiques de focus et de fermeture séparées | Placement, cible mappée, brouillon unique, Entrée/Échap et restitution du contexte partagés | Éviter les écarts entre capacités |
| `- ` reconnu seulement au début du paragraphe Tiptap | Reconnu au début d'une ligne saisie, y compris après un retour simple ; même règle pour `1. ` | Respecter nos raccourcis, indépendamment de Markdown |
| Ajout d'une liste absorbant le paragraphe au curseur | Widget vide au curseur ; seul un texte explicitement sélectionné devient son contenu | Préserver le texte non sélectionné |
| Entrées successives produisant des paragraphes vides | Retour simple interne ; deuxième retour = nouveau paragraphe ; retours supplémentaires sans ajout | Une seule ligne de séparation, avec `¶` atténué |
| Citation ajoutant des paragraphes par défaut | Insertion au curseur sans paragraphes vides supplémentaires ; mêmes règles de retours dans son corps | L'utilisateur décide du découpage |
| Retour arrière héritant de la sortie de liste Markdown | Retour au texte précédent ; fusion entre items ou retrait d'un item vide, sans désassembler le widget | Préserver les frontières et la rédaction |

`¶`, les en-têtes et les espaces des bandeaux sont des éléments de présentation :
ils ne sont pas des caractères de Yjs, du Typst, des compteurs ou du contenu MCP.
Les listes restent non imbriquées. Le collage et l'import de documents structurés
restent des parcours distincts de la reconnaissance interactive des raccourcis.

Les paramètres de listes disposent des trois choix segmentés ; le numéro initial
apparaît pour une liste numérotée. Le texte alternatif et le remplacement d'une
image sont provisoires jusqu'à Appliquer. Entrée valide l'action par défaut ou le
bouton explicitement atteint par Tab ; Échap abandonne le brouillon. Réutiliser la
commande contextuelle redonne le focus au brouillon existant. Une cible modifiée
est refusée ; une modification située avant elle reporte sa position.

La réservation interligne utilise les décorations ProseMirror et les emplacements
non éditables des NodeViews. Le panneau est positionné dans cette place réservée ;
il ne recouvre pas le texte. Ce mécanisme commun évite de déplacer ou de publier le
contenu pour afficher un contrôle, et garde les champs de formulaire montés lors
d'un redimensionnement ou d'une modification distante.

### Insertion des widgets : découpage et reprise

Toutes les insertions graphiques de widgets de bloc utilisent la même opération :
tableaux (construits par Tiptap), listes, descriptions, citations, images, P.-S. et
P.J. La commande standard de tableau n'est plus utilisée pour placer le widget.
La grille du tableau suit directement sa barre de commandes, sans marge interne
simulant une ligne vide au-dessus ou au-dessous.

- En début de ligne, insérer directement, sans conserver un paragraphe vide avant.
- Au milieu ou en fin de ligne, couper au curseur et placer le widget après la
  partie gauche, sans ajouter de ligne vide entre les deux.
- Un retour simple déjà présent à cette frontière n'ajoute pas un second saut.
- Toujours réserver un paragraphe vide immédiatement après le nouveau widget.
  Réutiliser celui qui existe déjà ; le texte situé à droite du curseur reste
  conservé après cette ligne de reprise.
- Le focus initial reste dans le widget. Le paragraphe suivant permet de reprendre
  la rédaction ; sortir d'une liste ou d'une annotation réutilise cette ligne.
- L'insertion, son découpage et sa ligne de reprise forment une seule opération
  d'annulation. Une insertion depuis une zone qui ne peut pas contenir de widget
  (cellule, annotation, etc.) est indisponible, sans déplacement implicite hors
  de cette zone.

Cette règle concerne l'insertion de blocs. Les notes de bas de page restent des
éléments en ligne ; changer le type d'une liste existante reste une conversion.

### Périmètre restant distinct

Le lancement Rust par défaut et l'ancienne CLI restent inchangés pendant l'essai.
La CLI historique ne lit pas les archives version 10 ; le MCP rattaché de la
nouvelle application emploie son nouveau contrat. Il ne faut pas confondre cet
adaptateur avec l'ancien serveur autonome, qui documente sa disponibilité propre.
Le catalogue propose actuellement le seul modèle personnel intégré : bibliothèque
de modèles importables, sélection de polices système détaillée, personnalisation
des raccourcis, ACP et collaboration réseau restent des travaux séparés.

Les contrôles automatiques ne remplacent pas un essai matériel des méthodes de
saisie macOS/Windows/Linux. La composition est gérée par le navigateur ; il faut
encore refaire l'essai manuel japonais dans cette interface avant de le valider.

## Vérifications et prochain palier

Le [compte rendu des contrôles](tauri-interface-results/README.md) conserve les
mesures et captures clair/sombre. Les recettes utilisent le vrai moteur Typst ;
le client MCP est un client protocolaire, sans LLM ni skill.

Le prochain palier avant de retirer la référence egui est le raccordement du
lancement et des adaptateurs documentaires restants, puis la stabilisation des
erreurs typées du nouveau contrat structuré. La conception des types déclaratifs
et des manifestes de modèles ne fait pas partie de ce lot d'interface.

## Deux composants obligatoires : widgets et panneaux contextuels

Le cadre d'un widget et le panneau d'une interaction sont deux composants distincts.
Les capacités ne reconstruisent ni leur en-tête ni leur bandeau.

- `src/widget-frame.ts` construit l'enveloppe des huit widgets : marqueur de
  contexte, emplacement du panneau B, barre, titre, corbeille et paramètres.
  Les vues de listes, descriptions, citations, images, tableaux et P.-S./P.J.
  l'utilisent toutes. Le tableau conserve la vue maintenue par Tiptap pour sa
  grille ; le composant commun lui ajoute son enveloppe et protège son interface
  des mutations du contenu éditable.
- `src/context-panel.ts` construit le titre, la fermeture accessible, le corps,
  les actions, l'action par défaut et l'erreur d'un panneau. Les paramètres,
  confirmations de suppression, liens, notes et corrections orthographiques
  fournissent uniquement leur contenu et leurs actions. Ce composant reste
  indépendant d'un correcteur : une future correction grammaticale le réutilisera.
- `src/band-layout.ts` est le gestionnaire unique de ces panneaux. Il reçoit une
  cible typée : `widget-header` pour l'en-tête déplié B, `text` pour un passage.
  Le second cas détecte l'enveloppe commune, sans énumérer des balises HTML :
  largeur éditable dans la prose, largeur intérieure du cadre dans un widget,
  avec une pointe vers le passage. Les bordures asymétriques d'une citation sont
  prises en compte. Un seul panneau reste ouvert, et son brouillon, son focus,
  sa cible suivie, le défilement et le clavier suivent les règles communes.

Le gestionnaire est fourni directement aux propriétés et au correcteur ; il n'y
a plus de relais global `band:open-inline` / `band:close`. La commande contextuelle
et son raccourci partagent le même aiguillage. Corbeille, confirmation et raccourci
de suppression rejoignent `removeWidget` ; le bouton auxiliaire de suppression du
tableau ouvre lui aussi la confirmation commune. Les actions propres aux cellules
restent des contrôles du tableau.

La présentation commune se règle dans les sélecteurs `.content-widget`,
`.widget-bar` et `.context-band`. Des variables de variante permettent à une image
ou à une citation de garder leurs particularités, sans recopier les règles du cadre.
Le catalogue documentaire reste en Rust : la liste des vues de widgets n'est pas
un second catalogue de capacités. Le schéma du document, Yjs et le contrat MCP
ne changent pas pour construire ces composants de présentation.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Image, tableau et notes de lettre assemblaient leur propre cadre. | Un constructeur d'enveloppe pour les huit widgets. | Modifier la présentation commune à un seul endroit. |
| Propriétés et orthographe reconstruisaient titre, fermeture et actions. | Un composant de panneau, hébergé par un gestionnaire unique. | Garder les interactions cohérentes quand de nouveaux panneaux sont ajoutés. |
| Le placement cherchait plusieurs balises pour reconnaître un widget. | Un marqueur d'enveloppe et deux modes de placement explicites. | Ajouter une capacité sans modifier les règles de positionnement. |

## Affinage du cadre B — en-tête conservé

Choix du 26 octobre 2026, précisés sur l'exemple de la citation puis appliqués aux
composants communs. Cette évolution affine la variante B ; les anciennes maquettes
et captures restent des références historiques.

- L'en-tête du widget reste au-dessus du bandeau, avec la même géométrie et les
  mêmes commandes dans les deux états. Son symbole reprend l'icône du catalogue
  Rust utilisée par la barre d'outils ; son nom reste accessible et en info-bulle,
  ou remplace l'icône si aucune n'est disponible.
- Replié : en-tête, filet, contenu. Déplié : en-tête, bandeau, filet, contenu.
  Le filet sous l'en-tête devient transparent pendant l'ouverture pour garder
  sa hauteur sans séparer les deux surfaces bleues.
- Le bandeau intégré ne répète ni le nom du widget ni un titre de paramètres,
  et n'affiche pas de fermeture supplémentaire. Annuler et Échap le ferment.
- La corbeille montre uniquement Annuler et Supprimer, sauf erreur à signaler.
  Les paramètres montrent leurs contrôles puis Annuler et Valider ; Supprimer
  reste réservé à la corbeille. Les autres contrôles propres à une capacité,
  comme Remplacer pour une image, restent disponibles dans les paramètres.
- Le fond coloré appartient à l'en-tête et au bandeau déplié. Le contenu éditable
  garde le fond de la zone de rédaction, dans les deux thèmes.

Les panneaux liés à un passage (liens, notes, orthographe) conservent leur titre
et leur placement interligne avec pointe. Un lien en ligne garde son action
Retirer le lien ; il n'est pas un widget doté d'une corbeille. Les règles de
brouillon, péremption, focus, clavier et annulation restent celles du gestionnaire
commun. Le nouveau libellé Valider est traduit par Fluent.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Le panneau B apparaissait avant l'en-tête et répétait le nom. | L'en-tête reste en premier ; le panneau sans titre se déploie dessous. | Conserver le repère et gagner de la place. |
| Les paramètres proposaient aussi Supprimer ; la corbeille avait une explication. | Paramètres : Annuler/Valider ; corbeille : Annuler/Supprimer. | Séparer les deux intentions sans actions redondantes. |
| Le fond doux recouvrait le contenu du widget. | Le fond doux se limite à l'en-tête et au bandeau. | Garder une surface de rédaction uniforme. |
| Les en-têtes affichaient un nom alors que la barre utilisait une icône. | La même icône est fournie par le catalogue, avec un nom accessible et un repli textuel. | Reconnaître la capacité avec le même symbole partout. |

## Paramètres réels et citation — correction du 26 octobre

Un panneau de paramètres doit contenir des contrôles utilisables. Une explication
sur le modèle ne justifie pas son ouverture. La disponibilité appartient à la
définition commune de la vue du widget et correspond aux contrôles implémentés,
indépendamment du nombre de paramètres de sa signature Typst.

- Les listes ont le choix du type et, pour une liste numérotée, le numéro initial.
  L'image a son texte alternatif et son remplacement : leurs paramètres restent actifs.
- Citation, P.-S., P.J. et tableau n'ont actuellement aucun contrôle secondaire
  dans ce panneau. Leur roue dentée est grisée, désactivée et accompagnée d'une
  explication en info-bulle. Les paramètres éditables directement dans le contenu
  et les commandes propres au tableau restent des interactions distinctes.
- La même disponibilité est vérifiée avant l'ouverture et par la commande
  contextuelle/palette. Ctrl/Cmd+Entrée n'ouvre aucun panneau vide et n'insère pas
  de retour à la ligne à la place : ce raccourci est réservé à l'action contextuelle.
  Une erreur orthographique reste prioritaire, y compris dans une citation dont
  la roue dentée est désactivée. Un panneau déjà ouvert reste accessible au raccourci.
- La corbeille et la suppression directe restent disponibles pour ces widgets.

Le cadre extérieur de la citation reprend la bordure ordinaire de 1 px. La barre
verticale d'accent de 5 px est à l'intérieur du contenu et accompagne les paragraphes
cités, y compris leur marque de séparation. Elle ne borde ni l'en-tête, ni le bandeau
de suppression, ni l'attribution. Le filet de l'attribution occupe 100 % de la
largeur intérieure disponible, et le texte est aligné à gauche. Aucun changement
du schéma ou du modèle Typst n'est nécessaire pour ces réglages visuels.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Une roue active ouvrait une simple explication sans contrôle. | Roue désactivée, commande indisponible et ouverture refusée lorsqu'il n'y a aucun réglage. | Éviter une interaction inutile, quel que soit le déclencheur. |
| La bordure gauche épaisse entourait tout le widget citation. | Cadre normal ; barre d'accent uniquement sur le texte cité. | Distinguer la citation de son attribution et de ses commandes. |
| L'attribution était centrée sous un filet de 80 %. | Attribution à gauche sous un filet de 100 %. | Garder une délimitation et un alignement clairs. |
