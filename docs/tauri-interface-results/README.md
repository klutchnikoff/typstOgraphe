# Vérifications locales — interface Tauri

## Accueil et préparation de la fusion — 26 octobre 2026

Revue de l’accueil et de l’identité visuelle avant intégration dans main.

| Before | After | Why |
| --- | --- | --- |
| Accueil textuel et récents empilés | Signe tO, trois actions centrées ; récents à retrouver dans l’organisation de l’interface | Accès direct à créer, ouvrir ou reprendre |
| Identité native provisoire | Icônes adaptées aux trois plateformes, ressources embarquées | Cohérence de l’application installée |
| Pas de recette dédiée à ce nouvel accueil | Deux parcours sur Chromium et WebKit : actions pendant l’animation, retour sans rejeu, mouvement réduit, fenêtre minimale | Vérifier le comportement au-delà d’une capture |

Compilation TypeScript/Vite, formatage et **66 tests unitaires** réussis.
La suite complète de **72 parcours Chromium/WebKit** est verte. Captures de
l’accueil [clair](welcome-light.png) et [sombre](welcome-dark.png) inspectées.
L’adaptateur Rust passe ses dix tests (un diagnostic ignoré) et Clippy.
Ces essais ne remplacent pas les recettes natives Windows/Linux.

La CI Windows précédente échouait sur les imports SVG externes : Vitest
autorisait un chemin issu de URL.pathname, incorrect pour un lecteur Windows.
La configuration utilise désormais fileURLToPath, comme Vite ; la validation
distante de ce correctif reste à contrôler sur la prochaine CI.

## Résultats du portage

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

## En-tête stable et bandeau compact

La citation sert de référence pour l'affinage du 26 octobre : l'en-tête est
conservé au-dessus du bandeau, l'icône est celle de la barre d'outils, le bandeau
ne répète plus de titre et le fond de rédaction est neutre. Les paramètres ont
Annuler/Valider ; la corbeille a seulement Annuler/Supprimer. Ces règles sont
appliquées aux composants communs, sans modifier le document ni son rendu Typst.

Les captures des trois états (replié, paramètres, suppression), en clair et sombre,
ont été inspectées. Le parcours vérifie le contenu et les dimensions inchangés de
l'en-tête, le positionnement du bandeau juste dessous, l'absence de filet entre les
deux, l'identité de l'icône avec celle de la barre, les couleurs de fond, les actions
visibles et la fermeture au clavier. La vérification commune des huit widgets et
le parcours Tab de l'image suivent également cette nouvelle disposition.

Validation du lot : **50 parcours Chromium/WebKit**, **43 tests web**, **8 tests
i18n**, construction TypeScript/Vite, reconstruction du pilote Rust avec les
catalogues mis à jour et formatage Prettier réussis.

Les nouvelles références locales sont dans `output/tauri-widget-presentation/`.
Les maquettes et recettes précédentes restent conservées. Ces contrôles ne
constituent pas un essai manuel d'IME ou une recette native Windows/Linux.

## Paramètres disponibles et cadre de citation

La recette commune vérifie désormais deux groupes : paramètres actifs pour les
listes et images ; paramètres désactivés pour citation, tableau, P.-S. et P.J.,
avec une info-bulle explicative. La suppression reste vérifiée pour les huit
widgets. Le parcours de tableau ouvre la confirmation de sa corbeille au lieu
d'un panneau sans contrôles.

Le parcours visuel de citation vérifie son cadre extérieur de 1 px, la bordure
intérieure de 5 px sur les paragraphes, l'absence de bordure gauche sur la source,
le filet pleine largeur et l'alignement à gauche. Il teste également les deux
variantes de Ctrl/Cmd+Entrée sans édition parasite, l'indisponibilité dans la
palette, l'accès direct à l'attribution par Tab et l'annulation de la suppression.
Le parcours orthographique couvre maintenant aussi une citation : la correction
reste prioritaire même lorsque ses paramètres sont désactivés.

Vérifications : 43 tests web et 8 tests i18n réussis ; construction TypeScript/Vite,
pilote Rust et formatage réussis. La suite graphique a passé 49 parcours sur 50 ;
le parcours citation Chromium échouait car le test envoyait Tab sans attendre le
retour du focus depuis la palette. Après ajout de cette attente, le parcours a été
répété trois fois sur chaque navigateur, avec six réussites. Aucun changement du
comportement applicatif n'a été nécessaire pour cette synchronisation du test.

Les captures claires et sombres ont été inspectées. Les références de cette
correction sont conservées dans `output/tauri-quote-settings/`, sans remplacer
les captures précédentes. Aucun nouvel essai matériel d'IME ni de recette native
Windows/Linux n'est revendiqué pour ce lot.


## Réglages générés depuis le contrat

La disponibilité n’est plus une table de booléens dans les vues de widgets. Le
contrat Rust déclare les paramètres et leurs destinations ; le même calcul
construit les champs, active la roue et autorise l’ouverture contextuelle. Les
paramètres directement édités dans le bloc ne sont pas répétés dans le panneau.
Le groupe de conversion des listes provient lui aussi du catalogue partagé.

Les tests ajoutent notamment un paramètre numérique inédit au contrat d’une
citation : il active sa roue, reçoit sa valeur par défaut, refuse une valeur
hors bornes sans publier d’édition, puis rejoint l’attribut et la sérialisation
ProseMirror. Aucun cas dédié à ce paramètre n’est ajouté dans le widget ou le
formulaire. Les tests couvrent également l’absence de paramètres, les champs
édités ailleurs, les conversions, les liaisons de noms et les types de valeurs.
La génération du contrôle ne remplace pas l’implémentation de sa sémantique dans
le cœur et le modèle Typst.

Validation : **49 tests web**, **14 tests du contenu structuré du cœur**, **9 tests
Tauri**, **8 tests i18n**, construction TypeScript/Vite et Rust, Clippy Tauri avec
avertissements refusés et formatage réussis. Les **50 parcours Chromium/WebKit**
ont réussi ; après l’unification finale de la disponibilité des liens et notes
avec celle des widgets, les six parcours ciblant la commande contextuelle, les
liens et la citation ont également réussi sur les deux navigateurs.

Les captures des listes en clair, des images en sombre et de la citation repliée
ont été inspectées. La présentation B est conservée. Les références locales
sont dans `output/tauri-capability-settings/`. Aucun nouvel essai matériel d’IME
ni aucune recette native Windows/Linux n’est revendiqué dans ce lot.


## Ligne préservée sous les bandeaux contextuels

Le scénario minimal « Test des notes », avec insertion d’une note après « Test »,
a reproduit le défaut avant correction : les fragments de texte se retrouvaient
sur deux lignes séparées par le panneau. Le document ne contenait pourtant pas
de retour ajouté. La comparaison des sommets des rectangles de glyphes dans
`BandHost` confondait l’exposant avec une autre ligne.

La correction se limite au calcul commun du début de ligne : les rectangles
adjacents peuvent avoir des sommets différents lorsqu’ils se chevauchent
verticalement. Aucun changement spécifique aux notes, aux liens ou au correcteur
n’a été ajouté. Les futurs bandeaux ancrés sur du texte empruntent ce calcul.

La recette ajoute des contrôles du même invariant pour une note, un lien
volontairement surélevé et une correction orthographique, dans la prose, une
liste et une citation. Les positions horizontales de tous les caractères sont
comparées avant et après ouverture, le panneau doit rester juste au-dessus de
sa cible, et le JSON documentaire doit rester identique. Un autre parcours
vérifie les retours simples, les replis automatiques et le redimensionnement
pendant l’ouverture. La confirmation, l’annulation et les autres panneaux B
restent couverts par les recettes existantes.

Validation : **60 parcours Chromium/WebKit**, **49 tests web**, construction
TypeScript/Vite et contrôle Prettier réussis. Les captures du scénario minimal,
des panneaux dans les widgets et du cas multiligne ont été inspectées. Les
références avant/après sont conservées dans `output/tauri-inline-bands/`.
Le cœur Rust n’a pas été modifié ; aucun nouvel essai matériel d’IME ni recette
native Windows/Linux n’est revendiqué dans ce lot.


## Profil du rendu : compilation et préparation de l’aperçu

Mesure locale du 26 octobre 2026, macOS, profil de test non optimisé pour le code
applicatif (`opt-level=0`, informations de débogage désactivées). Chaque scénario
comporte une première itération exclue puis huit mesures à chaud, avec une petite
modification du texte à chaque mesure. Les deux chemins alternent leur ordre et
gardent leur monde Typst en mémoire. Ce sont des durées côté Rust, hors frappe,
temporisation, IPC et décodage dans la vue web ; aucun gain release n’est déduit
de ces mesures de développement.

| Scénario | Référence RGBA avant | Aperçu web avant | Référence RGBA après | Aperçu web après |
| --- | ---: | ---: | ---: | ---: |
| Courrier simple, 1 page | 32,8 ms | 463,2 ms | 28,0 ms | 54,9 ms |
| Document d’essai, 2 pages | 92,8 ms | 1 015,0 ms | 34,3 ms | 93,2 ms |

La référence RGBA appelle exactement `core::rendering::render`, utilisé par egui,
dans le même banc Rust ; ce n’est pas une mesure d’une fenêtre egui complète.
Son amélioration après correction vient de `tiny-skia`, également utilisé par
ce chemin dans le banc de mesure. Les médianes indépendantes par étape ne
s’additionnent pas exactement à la médiane de la durée totale.

Pour le courrier simple, avant correction : préparation 7,4 ms, compilation
11,5 ms, rasterisation 7,6 ms, préparation/encodage PNG 434,1 ms, base64 0,4 ms et
synchronisation 1,2 ms. Après correction : compilation 11,7 ms, rasterisation
1,9 ms, PNG 31,7 ms et base64 0,8 ms. La génération du source avec sa carte de
positions est mesurée séparément : 0,13–0,14 ms pour le courrier simple et
0,37–0,38 ms pour le document d’essai.

Les pages restent en 1 191 × 1 684 pixels. Le compromis est un flux d’aperçu un
peu plus gros : les URL base64 passent d’environ 117 à 219 ko pour une page et de
210 à 400 ko pour les deux pages. Il s’agit d’octets d’URL de l’aperçu, pas de la
taille des archives ni des PDF exportés.

Données brutes versionnées :
[avant](../../experiments/web-editor/results/preview-profile-before.json) et
[après](../../experiments/web-editor/results/preview-profile-after.json).

Pour reproduire depuis la racine du dépôt :

```sh
CARGO_INCREMENTAL=0 cargo test -p typstographe --locked --offline profile_preview_pipeline -- --ignored --nocapture --test-threads=1
```

Le diagnostic est ignoré dans la CI courante et n’impose aucun seuil temporel
lié à une machine. Le test fonctionnel compare les PNG décodés à l’ancien
encodage, pixel par pixel, pour des pages opaques et transparentes.


Validation du lot : **10 tests Rust**, **6 parcours Chromium/WebKit** ciblant le
rendu réel, les annotations, les images, la sauvegarde collaborative et la
synchronisation, construction du binaire, formatage Rust et Clippy avec
avertissements refusés réussis. Le diagnostic de performance a été lancé
explicitement ; aucune limite temporelle n’est ajoutée à la CI.


## Remplacement des pages après décodage

La recette force une attente distincte pour chacune des deux pages du nouvel
aperçu. Elle vérifie que les anciennes pages restent en place pendant toute
l’attente, y compris après libération de la première page, puis que les deux
images ajoutées sont déjà chargées et possèdent leurs dimensions intrinsèques.
Le défilement reste identique et le statut demeure « Mise à jour… » tant que
l’affichage n’a pas été publié. Avant correction, cette recette échouait : le
client n’attendait aucun décodage.

Six tests unitaires supplémentaires couvrent la publication groupée, la
conservation de l’ancien aperçu sur erreur, les résultats concurrents et
périmés, les changements de document/version/révision, les erreurs tardives et
la navigation issue d’une ancienne image. Les références visuelles de l’attente
et du nouvel aperçu sont conservées dans `output/tauri-preview-swap/`.


Validation : **55 tests unitaires web**, **62 parcours Chromium/WebKit**,
construction TypeScript/Vite et formatage Prettier réussis. Les captures de
l’ancien aperçu maintenu pendant le décodage et de l’aperçu actualisé ont été
inspectées. Les optimisations de l’encodage Rust sont conservées ; aucune
nouvelle recette native macOS/Windows/Linux n’est revendiquée dans ce lot.


## Aperçu RGBA natif — 26 octobre 2026

Le rendu interactif utilise désormais les pixels RGBA par IPC binaire vers un
canvas. Les captures [claire](preview-rgba-light.png) et
[sombre](preview-rgba-dark.png) montrent le parcours réel du navigateur, vérifié
visuellement après édition d’une note, d’une légende et d’un tableau. Les
compteurs de ces captures utilisent le relais fonctionnel STDIO/SSE et ne servent
pas à chiffrer la latence native.

La [recette native et ses résultats](../tauri-preview-performance.md) comparent
quatre transports dans la vraie fenêtre Tauri, avec les mesures optimisées et de
développement, la frappe continue, les limites et les rapports bruts.
