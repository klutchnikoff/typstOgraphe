# Navigation clavier de l’interface Tauri

Le contrat commun est dans `app/src/keyboard-navigation.ts`. Les composants
annoncent leur famille de navigation ; ils ne réimplémentent pas les touches.
Les actions continuent à passer par leurs commandes et validations habituelles.

## Conventions

| Famille | Touches | Focus et exécution |
| --- | --- | --- |
| Recherche de commandes ou de texte | Haut / Bas, Entrée | Le champ conserve le focus ; le choix actif est annoncé par `aria-activedescendant`. Entrée exécute la commande ou passe à l’occurrence suivante ; Maj+Entrée revient à la précédente dans la recherche de texte. |
| Menu | Haut / Bas, Gauche / Droite, Début / Fin, Entrée, Échap | Gauche/Droite passe au menu voisin ; navigation parmi les actions disponibles ; Entrée exécute, Échap ferme et rend le focus au déclencheur. |
| Onglets et choix exclusifs | Gauche / Droite, Début / Fin | Activation du choix parcouru. Les onglets reconstruits conservent le focus. |
| Suggestions orthographiques | Flèches, Entrée | Le bandeau garde d’abord le focus, même si les suggestions arrivent plus tard. Les flèches choisissent ; Entrée applique. |
| Actions, barre d’outils, en-têtes de widgets | Gauche / Droite, Début / Fin | Déplacement du focus seulement ; Entrée ou Espace déclenche le bouton. |
| Listes de récupération et dictionnaire personnel | Haut / Bas, Début / Fin | Parcours des entrées. Les actions de récupération restent explicites. |
| Formulaire | Tab / Maj+Tab, Entrée, Échap | Parcours des champs ; Entrée valide ou actionne le bouton focalisé. Échap ferme la portée locale sans valider. |

Les champs conservent leurs touches natives : déplacement du curseur, sélection,
flèches des nombres et listes déroulantes. Les sélecteurs natifs de fichiers,
dates et couleurs gèrent eux-mêmes Entrée. Aucune action n’est interceptée
pendant une préédition IME. Dans les bandeaux, Entrée valide et Maj+Entrée
insère un retour ; dans les adresses multilignes des préférences, Entrée reste
un retour à la ligne.

Un déplacement vers une corbeille, « Refuser » ou « Tout remplacer » ne déclenche
jamais cette action. Les choix masqués et indisponibles sont ignorés. Les parcours
bouclent ; Début et Fin visent les extrémités, sauf dans les champs de saisie.

## Brancher un composant

- `navigationGroup(root, options)` installe une navigation déléguée, avec découverte
  des éléments au moment de l’interaction et observation de leurs ajouts/retraits.
  L’axe peut être vertical, horizontal ou double. `input` active une sélection
  virtuelle dans une liste, sans quitter le champ ; `roving` limite le groupe à
  un arrêt Tab ; `activateOnMove` est réservé aux onglets et choix exclusifs.
- `actionRow()` crée le groupe horizontal commun aux boutons d’action. Les boutons
  ajoutés ensuite héritent du comportement, y compris les actions d’assistance.
- `keyboardScope(root, options)` déclare une fermeture, une validation et, si
  nécessaire, le parcours Tab d’un formulaire. `initialGroup` permet d’entrer
  dans une liste de suggestions reçue de manière asynchrone. `tabExit` restitue
  la saisie à l’éditeur au bord d’un bandeau.
- `installMenus()` applique le contrat des menus à tous les `.tool-menu`, y compris
  ceux créés plus tard. Les menus natifs macOS conservent leur comportement système.

Les générateurs `contextPanel`, `widgetFrame` et les barres d’action installent
ces contrats. Un nouveau widget ou bandeau qui utilise ces générateurs en hérite.
Un nouveau type de contrôle doit déclarer sa famille ; les règles ne sont pas
déduites du texte du bouton ou du nom de la capacité.

Les groupes imbriqués possèdent chacun leur axe : Gauche/Droite parcourt les
boutons d’une entrée ; Haut/Bas peut remonter à la liste des entrées. Les champs
éditables restent exclus. Un élément retiré pendant qu’il porte le focus cède
celui-ci au prochain élément disponible.

## Présentation commune des bandeaux

Tous les bandeaux utilisent `contextPanel` et le même cadre CSS : en-tête visible
avec titre et fermeture, contenu, actions et erreur éventuelle. Cela comprend les
paramètres et la suppression d’un widget, ainsi que l’orthographe, les liens et
les notes dans son texte. Il n’existe plus de variante visuelle « dans un widget » :
seule la largeur s’adapte à son conteneur. Les champs conservent les mêmes touches
et les boutons passent par les mêmes groupes de navigation.

L’ancrage reste adapté au déclencheur : les paramètres et la suppression se
placent sous l’en-tête du widget ; un outil de texte se place au-dessus du passage
concerné avec sa flèche. Ce placement ne change ni l’anatomie ni le comportement
du formulaire. Le bandeau reste hors du DOM éditable pour protéger la sélection.

## Fermeture et focus

Un seul résolveur Échap ferme la portée visible la plus proche du focus.
Lorsque le focus est dans l’éditeur, il choisit la portée ouverte prioritaire :
menu (40), dialogue (30), bandeau (20), panneau latéral (10), informations du document (5).
Chaque propriétaire rend le focus à son déclencheur ou à son repère dans
l’éditeur. Les menus utilisent le même résolveur. Celui-ci intervient en capture, avant
que ProseMirror n’absorbe Échap : un panneau reste fermable après le retour du
focus dans l’éditeur.

À l’ouverture et lors d’un retour par Ctrl/Cmd+Entrée, `BandHost` confie le focus
à `contextPanel.focus()` : cible explicite `data-initial-focus`, choix exclusif
actif, premier champ visible et disponible, puis formulaire si aucun champ
n’existe. Les suggestions différées ne volent pas ce focus. Depuis le formulaire, les
flèches rejoignent le groupe de suggestions s’il contient des choix disponibles ;
sinon elles rejoignent le premier groupe disponible, notamment les actions.
Un groupe vide, masqué ou désactivé ne bloque donc pas la navigation. La fermeture et la
validation restituent le repère de sélection enregistré, reporté à travers les
éditions du document, puis le défilement vers ce repère.

Une commande qui ouvre un bandeau ne doit pas programmer `editor.chain().focus()`
avant son ouverture : Tiptap peut exécuter ce focus à la frame suivante et reprendre
la main au champ. `insertInlineWithSettings` est le chemin commun des insertions
inline accompagnées d’un bandeau. Le générateur `ParameterFields` accepte le hint
`x-editor.label_visibility: "accessible"` : le nom accessible du champ est conservé
sans afficher un libellé qui ferait doublon avec le titre du bandeau.

## Objets intégrés au texte

`inline-objects.ts` reconnaît les nœuds inline atomiques et sélectionnables à partir
du schéma ProseMirror (les retours simples, non sélectionnables, sont exclus).
`inlineObjectView` leur donne un badge non éditable, une sélection d’objet et le
même accès au bandeau par clic, Entrée ou Espace. Le nom accessible et l’info-bulle
donnent la fonction de l’appel de note et son contenu. Cette
présentation ne change ni le contenu documentaire ni le rendu Typst.

L’enveloppe sélectionnable et le cadre visible du badge sont deux éléments
distincts : le navigateur positionne son curseur à la frontière de l’objet,
à l’extérieur du cadre et de son padding.
L’espace de 0,5 ex appartient à l’enveloppe, avant le cadre : le point d’insertion
précède cet espace, et un clic dans celui-ci positionne le curseur avant l’objet.
Le rôle de bouton et l’arrêt Tab appartiennent au cadre visible, pas à cet espace.
Les flèches Gauche/Droite sans modificateur
traversent l’objet en gardant un curseur de texte ; elles ne produisent pas une
sélection d’objet que la frappe suivante remplacerait. Les combinaisons avec Maj,
Alt ou Ctrl/Cmd gardent leur comportement de sélection/navigation habituel.

Le badge affiche « Note 1 », « Note 2 », etc. `InlineObjectNumbering` dérive les
numéros de l’ordre des objets dans le document, y compris dans les citations et
cellules. Ses décorations sont recalculées une fois par modification documentaire ;
les mouvements de curseur réutilisent le résultat. Les NodeViews existantes
actualisent leur texte et leur nom accessible sans changer leur sélection ni
reprendre le focus. Aucun numéro n’est inscrit dans les attributs, les sauvegardes
ou Yjs : insertion, suppression, déplacement, annulation et édition distante
recalculent le même ordre dans chaque vue.

`InlineObjectFocus` met en évidence le badge uniquement quand le point d’insertion
est immédiatement à sa droite : fond accentué et texte en vidéo inversée, sans
modifier la géométrie du cadre. `activeInlineObject` est le résolveur commun de
cette décoration et de la commande contextuelle, indépendant de la numérotation.
Une sélection de texte ou d’objet ne constitue pas un point d’insertion actif.

Pour Ctrl/Cmd+Entrée, l’ordre est : bandeau déjà ouvert, badge actif à gauche du
point d’insertion, correction orthographique, puis paramètres du lien ou du widget.
Avant une note, le raccourci reste donc disponible pour corriger le mot précédent.
Entre deux badges, seul celui qui précède le curseur est actif et visé. Le clic
sur le cadre et Entrée/Espace sur son bouton permettent toujours d’ouvrir la note.

Retour arrière juste après un objet et Suppr juste avant ouvrent sa confirmation
de suppression, sans changer le document. La commande centrale Ctrl/Cmd+Retour
arrière applique la même priorité à l’objet plutôt qu’au widget qui l’entoure.
Une sélection explicite d’objet suit ce même chemin. Échap conserve l’objet et
restitue le curseur ; Entrée confirme, avec possibilité d’annuler ensuite. Le
bouton « Supprimer » des paramètres d’une note ouvre cette même confirmation.

## Raccordements

| Zone | Composant commun |
| --- | --- |
| Palette et recherche/remplacement | Liste avec focus dans le champ |
| Orthographe et futurs diagnostics | `contextPanel`, groupe de suggestions |
| Titres, listes, fin de courrier, menus de l’application | `installMenus` |
| Métadonnées | Groupe horizontal à activation automatique |
| Paramètres/suppression des widgets, liens et notes | `contextPanel` et `actionRow` |
| Préférences et dictionnaire personnel | Portée de formulaire et groupe vertical |
| Création et document non enregistré | Portée de dialogue et `actionRow` |
| Récupération | Entrées verticales et actions horizontales |
| Assistance, relecture du lot entier | `actionRow` dans la portée du panneau |
| Barre latérale, outils et en-têtes des widgets | Groupes selon leur axe visuel |

## Vérifications

`app/tests/keyboard-navigation.test.ts` couvre les ajouts dynamiques, les groupes
imbriqués, les champs natifs et l’IME, les suggestions asynchrones, le retrait
d’un élément focalisé et les fermetures imbriquées. Les parcours de navigateur
vérifient le focus et les documents dans Chromium et WebKit, sans capture.
Le parcours natif conserve les contrôles d’assistance, de sauvegarde et d’export.
