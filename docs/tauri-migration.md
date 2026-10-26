# Reconstruction de l'interface Tauri

Décision du 26 octobre 2026 : après essai du diagnostic, reconstruire l'interface
avec Tauri, Tiptap/ProseMirror et Yjs, en conservant le cœur Rust. Le portage démarré
sur `codex/tauri-interface` est intégré à `main`. Ce document conserve son bilan ;
les travaux restants sont désormais dans la [roadmap](../ROADMAP.md).
L'interface egui reste accessible comme référence jusqu'à son retrait décidé ensemble.

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
sur le modèle ne justifie pas son ouverture. La disponibilité est dérivée du contrat partagé des capacités et correspond aux
contrôles et actions effectivement proposés par le panneau. La règle est précisée
ci-dessous ; aucun drapeau actif/inactif n’est maintenu par widget.

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


## Réglages dérivés du contrat des capacités

La signature déclarée dans `core::content::catalogue::parameters` est la source
commune des paramètres pour les adaptateurs et la documentation MCP. Le panneau
web, la roue du widget et la disponibilité de la commande contextuelle dérivent
leurs réglages de ce même contrat, via `settingsForNode`. Le schéma MCP du lien
réutilise aussi le paramètre URL, sans redéfinition parallèle.

Par défaut, un paramètre optionnel produit un contrôle dans le panneau ; un
paramètre obligatoire appartient à la création. Les annotations `x-editor`
précisent une autre destination lorsqu’elle est nécessaire :

- `body` : édition directe dans le contenu, comme l’attribution d’une citation ;
- `structure` : commande structurelle, comme les lignes/colonnes du tableau ;
- `creation` : option de création, comme sa ligne d’en-tête initiale ;
- `panel` : réglage secondaire, y compris un paramètre obligatoire modifiable,
  comme la ressource d’une image ou l’URL d’un lien.

Ces indications décrivent l’interface d’un paramètre, sans autoriser une édition
ni dupliquer son implémentation Typst. Le contrat intégré en Rust reste la
référence ; il ne s’agit pas d’analyser la signature d’une fonction arbitraire
dans le code d’un modèle.

Le générateur de formulaire utilise le type, la valeur par défaut, les bornes,
les choix et le caractère obligatoire du schéma. Il fournit les champs texte,
multilignes, nombres, cases à cocher et listes de choix. Le contrôle de ressource
image appelle le sélecteur existant. `x-editor.attribute` permet une liaison
explicite lorsque le nom du paramètre diffère de celui de l’attribut structuré
(`url` vers `href`). Les libellés sont traduits dans l’adaptateur, sans Fluent
dans le cœur. Les types complexes non pris en charge échouent explicitement.

Les conversions de listes constituent une action réelle du panneau, distincte
d’un paramètre Typst. Le groupe de conversion provient lui aussi du catalogue.
La roue est active s’il existe au moins un champ de panneau **ou** plusieurs
choix de conversion ; sinon elle est automatiquement désactivée. Les listes
à puces et de descriptions conservent donc leur changement de type, tandis que
la citation ne présente pas un panneau vide pour son attribution déjà visible.

Le formulaire travaille sur un brouillon, contrôle les valeurs à la validation
et garde la protection contre les modifications concurrentes. Les attributs de
panneau manquants dans la définition du nœud sont déclarés depuis le même
contrat dans le schéma ProseMirror, partagé avec l’initialisation Yjs. Ajouter
un champ scalaire au contrat suffit à produire son contrôle ; la sémantique,
la validation documentaire et le rendu Rust/Typst de ce paramètre restent à
implémenter lors de l’évolution d’une capacité.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Un booléen par widget décidait si sa roue était active. | Champs et actions du contrat déterminent automatiquement la disponibilité. | Éviter une liste parallèle à maintenir. |
| Les champs du panneau étaient codés séparément par capacité. | Les contrôles ordinaires, leurs valeurs par défaut et contraintes viennent du schéma. | Faire évoluer la signature et son interface ensemble. |
| L’attribution était un paramètre optionnel mais ne justifiait pas de panneau. | Sa destination `body` indique qu’elle est éditée directement dans le bloc. | Exprimer la distinction dans le contrat, sans exception dans le calcul de disponibilité. |


## Placement commun des bandeaux interlignes — 26 octobre 2026

Les notes, liens et corrections orthographiques utilisent le même placement
`text` de `BandHost`. Leur appelant fournit uniquement la plage ciblée ; il ne
choisit ni un saut de ligne ni une règle de géométrie propre à sa capacité.

Le défaut observé avec « Test des notes » provenait de ce gestionnaire commun :
il assimilait une différence de hauteur du haut des glyphes à un changement de
ligne. Un exposant, ou une variation de police/taille, rompait cette hypothèse.
L’espace de présentation était alors inséré au milieu de la ligne, sans pour
autant modifier le document partagé.

La recherche du début de ligne compare désormais le chevauchement vertical des
rectangles adjacents, au lieu d’exiger des sommets identiques. Elle reste commune
à toutes les cibles textuelles, présentes ou futures, sans branche « note » ni
classe CSS particulière. Les limites des blocs, les retours simples et le repli
automatique restent respectés. Le placement `widget-header` des panneaux B garde
son ancrage sous l’en-tête ; il appartient également au gestionnaire commun.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Le haut d’un exposant différent du texte était interprété comme une autre ligne. | Des rectangles voisins qui se recouvrent verticalement appartiennent à la même ligne. | Accepter des éléments en ligne de tailles différentes. |
| L’espace du bandeau pouvait couper la phrase au niveau de sa cible. | L’espace est réservé avant le début de la ligne visuelle. | Garder le texte et son objet en ligne, avec le panneau au-dessus. |
| Les tests vérifiaient surtout le panneau et son ancrage. | Ils vérifient aussi la position horizontale de chaque caractère avant et après ouverture. | Détecter une coupure ou un changement de repli, quel que soit le type de cible. |


## Latence du rendu web — diagnostic du 26 octobre 2026

Les deux interfaces réutilisent le même compilateur Typst. Leurs compteurs
mesurent toutefois leur chaîne de rendu respective : egui conserve le tampon
RGBA, tandis que le prototype web convertit chaque page en PNG puis en URL
base64. `elapsedMs` inclut cette conversion et la préparation de la synchronisation,
mais pas les 180 ms de temporisation après saisie, la projection structurée vers
Typst, l’IPC ni le décodage par la vue web. La temporisation egui est de 220 ms et
n’entre pas non plus dans son compteur de rendu.

Le profil local a isolé l’encodage PNG comme principal surcoût en développement.
Le filtrage, la compression et les opérations de pixels étaient exécutés sans
optimisation Rust. La projection structurée reste inférieure à 1 ms sur les deux
documents d’essai ; la compilation Typst est autour de 12 ms à chaud.

Le correctif garde le transport PNG et la résolution existants :

- Optimisation ciblée de `png`, `fdeflate` et `tiny-skia` dans le profil Cargo de
  développement ; le code applicatif reste en debug. Le profil de test hérite
  également de ces réglages. Le profil release ne change pas.
- Utilisation de `png::Compression::Fast` pour l’aperçu local, sans perte de
  pixels. Le choix des filtres et de la compression reste celui de la bibliothèque.
- Consommation du tampon raster pour convertir l’alpha prémultiplié, au lieu
  de le copier au préalable. Les pages transparentes restent prises en charge.
- Les exports du cœur gardent leur propre politique ; le modèle Typst et la
  résolution de rendu ne sont pas modifiés.

Les durées par étape sont disponibles dans `Preview.timings` pour les prochains
diagnostics. Le compteur visible conserve son sens de durée totale côté Rust.
Un test ignoré par défaut compare le chemin RGBA du cœur et celui de l’aperçu
sur des sources, ressources, modèles et options de rasterisation identiques.
Il alterne l’ordre des deux chemins et modifie le texte à chaque itération.
Les résultats locaux avant/après sont conservés avec la recette détaillée.


## Publication de l’aperçu sans page vide — 26 octobre 2026

Avant le correctif de performance, la version web encodait déjà les pages en
PNG puis en base64, avec les réglages de compression par défaut et les
bibliothèques en debug non optimisées. L’optimisation garde ce transport et la
même résolution. Elle ne changeait pas le remplacement des images côté client.

Ce remplacement supprimait les pages visibles puis insérait immédiatement les
nouveaux éléments `img`, avant leur décodage par le navigateur. Le client pouvait
donc afficher brièvement des images vides, même si le PNG était déjà entièrement
produit côté Rust.

`previewController.show` prépare désormais les pages hors de l’arbre affiché et
attend `HTMLImageElement.decode()` pour chacune d’elles. Toutes les pages, les
calques de sélection et leurs références de synchronisation sont ensuite publiés
ensemble, en conservant le défilement courant. Le dernier aperçu reste affiché
pendant cette préparation ; aucune transition animée ni délai artificiel n’est
ajouté. Le statut « Mise à jour… » reste présent jusqu’à la publication effective.

Une édition, une nouvelle révision, un remplacement de document ou une
invalidation pendant le décodage interdit la publication de l’ancien résultat.
Une erreur de décodage garde le dernier aperçu et suit la voie de signalement
existante ; une erreur tardive sur un résultat périmé est ignorée. Le retour
asynchrone d’une navigation depuis une ancienne image est également rejeté.

Le compteur visible reste la durée côté Rust : il n’inclut pas cette attente de
décodage, qui était déjà nécessaire au navigateur avant de dessiner l’image.
Le fonctionnement de l’API est décrit dans la
[documentation de `decode()`](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/decode).

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Les anciennes pages disparaissaient dès réception du PNG. | Elles restent visibles jusqu’au décodage de toutes les nouvelles pages. | Éviter un aperçu temporairement vide. |
| Le statut de rendu pouvait annoncer la fin avant que les images soient prêtes. | Il est actualisé après publication des images décodées. | Faire correspondre le statut à ce qui est visible. |
| Un résultat pouvait devenir périmé pendant la préparation de l’affichage. | Version, révision, identité du document et ticket de publication sont vérifiés à la fin. | Conserver un aperçu et une navigation cohérents avec le document courant. |


## Réactivité native prioritaire — 26 octobre 2026

La vitesse de l’aperçu est un critère d’architecture : le prototype doit employer
les vrais mécanismes destinés à l’application, y compris pendant le développement.
Le délai de frappe de 180 ms est supprimé au profit d’une file immédiate avec une
seule compilation active et une demande pour le dernier état disponible.

L’aperçu courant transmet les pixels RGBA par IPC binaire Tauri à un canvas,
sans PNG ni base64. Le remplacement reste atomique et soumis aux contrôles de
révision et d’identité du document. La résolution, les polices et le rendu Typst
restent identiques. Les transports PNG et SVG servent au banc de comparaison.

La décision repose sur une recette dans la fenêtre native, avec frontend embarqué,
compilation Rust optimisée et mesures incluant la préparation de l’affichage et
une occasion de peindre. Le profil de développement est également mesuré avec le
même chemin de rendu. Méthode, résultats bruts, limites et procédure reproductible :
[latence de l’aperçu Typst](tauri-preview-performance.md).
