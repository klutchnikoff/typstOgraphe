# Vérifications locales — interface Tauri

26 octobre 2026, macOS Apple Silicon. Ces résultats décrivent un build de
développement ; ils ne remplacent pas la CI Windows/Linux ni un test utilisateur.

- Suite Rust du workspace : 600 tests réussis pendant le portage. Les ajouts ont
  ensuite été contrôlés par les suites ciblées : contenu structuré (13 tests),
  localisation (8), adaptateur Tauri/MCP (7), collaboration web (8).
- `cargo clippy --workspace --all-targets -- -D warnings` et clippy du workspace
  Tauri séparé ; contrôle de la frontière de dépendances du cœur.
- TypeScript, Vite, Prettier et recettes de scripts d'architecture vérifiés.
- Douze recettes graphiques en série : six parcours sur Chromium et WebKit.
  Elles couvrent rendu Typst, notes/images/tableaux au clavier, concurrence et
  reconnexion, annulation locale, sélection entre blocs, recherche/remplacement
  littéral, anciens widgets, raccourcis Typst et accueil/création.
- Contrôles complémentaires après séparation des panneaux : recherche et
  préférences persistantes, sur les deux navigateurs.
- Fenêtres Tauri natives : deux Y.Doc indépendants, fusion après reconnexion,
  annulation locale, annotations, lot MCP soumis via STDIO, relecture et
  acceptation par l'interface, sélection préservée, annulation du lot, écriture
  d'une archive sous verrou, export et reprise après fermeture du processus.
- Sources exportées recompilées avec le CLI Typst, sans polices système et avec
  le dossier de polices exporté (`--ignore-system-fonts --font-path fonts`).

## Mesures

Le document de saisie compte 100 paragraphes, 26 615 unités UTF-16 et sept pages.
La mesure va de `keydown` à la frame suivant la transaction, sans latence de
pilotage Playwright. Les fichiers JSON conservent les mesures brutes.

| Mesure | Chromium | WebKit |
| --- | --- | --- |
| Saisie, médiane | 8,5 ms | 9 ms |
| Saisie, p95 | 15 ms | 28 ms |
| Rendu complet des sept pages PNG | 6,7 s | 6,9 s |

Le RSS de la recette native ne couvre que le processus Rust, **pas WebKit**, ni
le client MCP de test. Voir `native-metrics.json`. Il ne s'agit pas de mesures
stabilisées de consommation globale de l'application.

Les captures [en mode clair](editor-light.png) et [sombre](editor-dark.png)
utilisent le courrier fictif de diagnostic. Ce courrier
n'est pas ouvert au lancement normal. `Essai Rust` et les identités fictives
restent réservés aux recettes.

## Points à vérifier avant la bascule par défaut

- Essai matériel des touches mortes et IME dans le nouvel éditeur sur les systèmes
  visés ; un test de saisie Unicode automatisé ne démontre pas le trajet IME.
- Raccordement de la CLI et du MCP autonome à l'archive structurée version 10.
- Unification des diagnostics encore textuels des nouveaux modules structurés
  avec les erreurs typées existantes, avant de stabiliser cette nouvelle API.
- Bibliothèque de modèles et manifeste de leurs options : chantier distinct.

Le client de recette MCP vérifie le protocole et les permissions. Il ne constitue
pas une nouvelle évaluation de la qualité rédactionnelle d'un LLM.

## Correctifs de l'interface après essai

- Construction TypeScript/Vite, formatage Prettier et huit tests de collaboration
  réussis après les corrections.
- Suite de vingt parcours graphiques réussie sur Chromium et WebKit. Les nouveaux
  contrôles couvrent les listes via la barre et la saisie `- ` / `1. `, Entrée,
  conversion et retrait de la liste sélectionnée, fermeture des menus sans édition,
  absence de cadre et surface éditable pleine colonne, disposition des actions et
  mise en évidence de l'enregistrement.
- Le serveur de recette utilise désormais un port distinct (1421 par défaut),
  sans réutiliser le serveur de développement et son éventuel document ouvert.
- L'échec Windows de la CI précédente se situait dans Prettier : le checkout
  convertissait les sources web en CRLF. Les attributs Git imposent désormais LF
  pour les fichiers texte de ce dossier, sans convertir les fichiers binaires.

Ces contrôles locaux ne constituent pas un essai natif sous Windows ou Linux.

## Reprise de la variante B et de la saisie

Les références sont la maquette `docs/mockups/interface-polish.html` en variante B
et les décisions du chantier L. Les captures de référence et celles de l'interface
ont été ouvertes et comparées : paramètres de liste, paramètres d'image en sombre,
correction dans la prose et dans un item. Les derniers contrôles ont corrigé le
titre de l'image, l'affichage de son bandeau entier après défilement, les coins du
cadre commun et le double contour de focus autour de la correction.

- Suite complète de **38 parcours** réussie sur Chromium et WebKit, suivie de
  contrôles ciblés des finitions et de la frontière terme/définition, tous réussis.
  La suite disponible comprend désormais 40 parcours sur les deux navigateurs.
- Contrôles clavier : `- ` au début du document ou après un retour simple, listes
  par boutons, insertion au curseur sans absorption du texte, deux retours puis
  retours supplémentaires, `¶` absent du document partagé, citation sans
  paragraphes vides ajoutés, Retour arrière dans les items et les descriptions.
- Contrôles des bandeaux : brouillon sans publication, un seul bandeau, choix de
  liste au clavier, Entrée/Échap/Tab, retour du focus, reprise de brouillon par
  Ctrl/Cmd+Entrée, application annulable en une étape, déplacement de la cible après
  une édition précédente et refus d'une cible modifiée.
- Navigation Tab des bandeaux normalisée : les boutons restent accessibles dans
  WebKit/macOS même lorsque la navigation native les saute. Le parcours du champ
  alternatif à Remplacer, Supprimer, Annuler et Appliquer est testé explicitement.
- Géométrie orthographique : pointe de 12 px vers le mot, largeur du bloc à 2 px
  près et centre de la pointe à 4 px près, à 1280 puis 900 px de largeur de fenêtre.
- Construction TypeScript/Vite, Prettier, huit tests web, huit tests i18n, sept
  tests de l'adaptateur Rust et clippy réussis. La frontière des dépendances du cœur
  reste vérifiée.
- Recette native Tauri complète puis reprise réussies : deux fenêtres, convergence,
  annulation locale, annotations, proposition MCP sous validation, archive, export
  PDF/Typst et réouverture.

Les traces et captures locales sont conservées dans `output/tauri-bands-b/` ;
la recette native est dans `output/web-editor/native-1793015839171/`. Les sorties
restent ignorées par Git, comme les autres captures de référence du projet.
La recette native vérifie les parcours documentaires ; le clavier et la géométrie
des nouveaux bandeaux sont contrôlés par les parcours Chromium/WebKit. Ces essais
ne remplacent pas un essai manuel d'IME ni une recette native Windows/Linux.

## Insertion commune des widgets

Le placement des tableaux, images, trois listes, citations, P.-S. et P.J. suit
maintenant un contrat commun : découper au curseur, ne pas ajouter de paragraphe
vide avant et réserver un paragraphe vide immédiatement après. Une ligne de
reprise déjà présente est réutilisée. Le texte non sélectionné reste conservé,
y compris ses marques de mise en forme.

| Avant | Après | Pourquoi |
| --- | --- | --- |
| Le tableau utilisait son placement standard et pouvait laisser une ligne vide avant. | Les huit widgets passent par la même opération de placement. | Garder une insertion prévisible, au curseur. |
| Une marge séparait la barre du tableau de sa grille. | La grille suit directement sa barre. | Éviter une bande vide ressemblant à une ligne supplémentaire. |
| La sortie d'une liste pouvait recréer un paragraphe de reprise. | La sortie réutilise le paragraphe vide suivant. | Continuer la saisie sans doubler les lignes. |

- **43 tests web**, dont 35 contrôles du contrat d'insertion : début, milieu, fin,
  retour simple existant, ligne vide réutilisée, sélection et frontières de cellule.
- Les nouveaux parcours graphiques exercent onze insertions par navigateur, via
  les commandes de la barre et le sélecteur d'image : structure exacte, focus
  initial, reprise de saisie et deux annulations restaurant le document initial.
  Le rendu Typst est réel ; l'espacement entre la barre du tableau et sa grille
  est contrôlé à 2 px près.
- La suite complète de **42 parcours Chromium/WebKit** passe après correction du
  pilote. Construction TypeScript/Vite, Prettier, huit tests Rust et clippy passent
  également.
- Les captures des tableaux en début, milieu et fin de ligne ont été inspectées.
  Elles sont conservées localement dans `output/tauri-widget-insertion/`.
- Le pilote de recette place désormais chaque scénario dans un dossier voisin.
  L'ancienne imbrication finissait par provoquer une erreur de chemin trop long
  lors de l'enregistrement des préférences dans la suite complète. Un contrôle
  Rust vérifie cinquante réinitialisations suivies d'un enregistrement.

Ces essais navigateur ne constituent pas une nouvelle recette native des trois
systèmes ni un essai manuel d'IME.

## Composants communs des widgets et panneaux

La construction des huit cadres passe par `widgetFrame`, y compris pour les
images, les tableaux et les P.-S./P.J. Les contenus éditables restent propres à
chaque capacité. Les paramètres, confirmations, liens, notes et corrections
orthographiques utilisent `contextPanel` et le même `BandHost`, avec deux cibles
explicites : en-tête de widget B ou passage de texte. Le gestionnaire est injecté
directement dans les consommateurs.

La vérification des cadres a relevé deux écarts : le panneau de citation ne tenait
pas compte de sa bordure gauche de 5 px, et le remplissage des P.-S./P.J. repoussait
leur bandeau. Le gestionnaire mesure maintenant les bordures réelles ; le
remplissage des notes de lettre appartient à leur contenu, avec l'en-tête commun
au bord intérieur du cadre. La présentation B reste la référence, sans nouveau
choix graphique pour ce lot.

Les nouveaux contrôles couvrent les huit widgets : un cadre, paramètres intégrés,
document inchangé à l'ouverture, confirmation par Entrée et suppression directe,
annulation restaurant le contenu exact. Un essai garde une requête de suggestions
en attente puis ouvre les paramètres d'une liste : la réponse tardive ne remplace
ni le nouveau panneau ni son focus. Une suppression directe immédiatement après
la saisie s'annule seule et restitue également le texte qui vient d'être écrit.

Validation du lot : **48 parcours Chromium/WebKit**, **43 tests web**, construction
TypeScript/Vite et formatage Prettier réussis. Le cœur Rust et son contrat ne sont
pas modifiés par ce lot.

Les captures de liste claire, image sombre, citation et correction dans la prose
ou un widget ont été inspectées. Elles sont conservées localement dans
`output/tauri-context-components/`. Aucun essai manuel d'IME ni nouvelle recette
native Windows/Linux n'est revendiqué dans ce lot.
