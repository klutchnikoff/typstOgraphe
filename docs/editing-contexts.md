# Contextes d’édition, positions et propositions — lots 6.3 et 6.4b

> Référence historique antérieure au contrat structuré Tauri. Pour l’application
> courante, voir l’[index documentaire](README.md) et le [contrat structuré](structured-contract.md).

## Un document, plusieurs contextes

Le moteur conserve un seul document publié, sa garde complète et son historique.
Le contexte principal est celui de l’utilisateur graphique, ou celui du serveur
MCP autonome. Les API existantes `run`, `query`, `selection` et la saisie native
continuent à utiliser ce contexte.

Un `EditContext` est un handle Rust opaque, lié à l’identité du document. Il
contient une sélection logique gardée et un style de frappe indépendants. Le
moteur ne bascule jamais temporairement vers le contexte agent pour exécuter une
lecture ou préparer une proposition.

| API | Effet |
| --- | --- |
| `create_edit_context(guard, None)` | Curseur au début et style neutre ; aucune sélection utilisateur copiée implicitement. |
| `create_edit_context(guard, Some(selection))` | Capture explicite d’une sélection courante et de son style environnant. |
| `primary_context()` / `editing_context(handle)` | Lire un état de contexte. |
| `select_in_context(handle, selection)` | Déplacer uniquement ce participant, avec garde vérifiée ; recalculer son style au déplacement. |
| `navigate_context(handle, params)` | Résoudre `Current` et la destination relative dans ce contexte, puis y déplacer seulement ce participant. |
| `query_in_context(handle, query)` | Résoudre `Current` dans ce contexte ; adapter sélection/style d’`inspect` et disponibilité de `commands.list`. |
| `accept_proposal(handle, candidate)` | Publication atomique après accord humain recueilli par l’adaptateur ; garde revérifiée, contexte utilisateur conservé. Aucun outil MCP correspondant. |

Créer, consulter ou déplacer un contexte n’ajoute ni révision ni étape d’annulation,
et ne change pas le contenu ou le style utilisateur. Les déplacements ultérieurs
de l’utilisateur ne sont pas suivis par l’agent. Les sélections sont toutefois
reportées lors des éditions du document. Le style de frappe du participant reste
le sien jusqu’à un déplacement explicite ou une future édition approuvée.

La dernière copie du handle relâchée rend l’entrée récupérable ; elle est retirée
à la publication ou à la création du contexte suivant. Fermer ou remplacer le
document invalide les handles immédiatement. Une proposition ne conserve qu’une
référence faible à son contexte, aucun verrou de fichier.

Le rattachement local garde un handle pour toute l’invitation. Chaque requête
emprunte le moteur et ce même contexte, après publication des entrées natives.
Depuis 6.4b, les éditions passent par `proposals.submit` et une décision humaine
dans l’application. Le lot 6.6 complète le transfert : **Édition → Confier la
sélection à l’agent**, également dans la palette, capture la sélection non vide
courante sans la suivre ensuite. Cette action ne dispose pas de raccourci par
défaut et reste indisponible pendant une interruption ou une proposition en attente.
L’agent peut appeler directement `text.select` pour sa propre navigation ;
`Current` désigne toujours son contexte. Cette navigation ne prend ni focus ni
sélection utilisateur et n’ajoute aucun historique. Les interruptions et les
propositions en attente la bloquent. Les sélecteurs restent disponibles dans les lots.

## Lectures de rendu différées

`prepare_render(guard)` capture les entrées publiées pour une compilation hors UI ;
`RenderTask::compile()` ne connaît pas le moteur. `complete_render(compiled)`
revérifie la garde avant d’installer ce cache dérivé et conserve un éventuel rendu
natif déjà courant. `prepare_page_image(reference, page)` conserve un rendu
immuable partagé ; `PageImageTask::execute()` peut rasteriser sur un autre thread,
même si le document évolue ensuite. C’est à l’adaptateur de refuser alors la réponse
périmée. Aucun de ces calculs ne modifie contenu, historique ou contextes.

## Report exact des positions

Chaque transaction publiée enregistre ses insertions et suppressions réelles dans
la projection, en valeurs scalaires Unicode, puis vérifie qu’elles reproduisent
exactement le texte publié. Cela inclut les normalisations des blocs, les opérations
sur annotations et images, les remplacements multiples et les lots atomiques.
L’historique conserve les opérations inverses, y compris pour la frappe regroupée.
Il n’y a ni recherche de texte similaire ni comparaison approximative entre deux
versions. Un brouillon natif contenant une mutation de texte non tracée est refusé.

`PositionMap` contient les gardes de départ/arrivée et les index de positions
logiques. `map_retained` vérifie la garde de départ, reporte chaque extrémité puis
reconstruit une cible valide dans la structure d’arrivée :

- avant une modification, la position reste en place ; après, elle est décalée ;
- dans un passage supprimé/remplacé, elle rejoint la fin du remplacement ;
- à un point d’insertion, elle suit le texte inséré (affinité après insertion,
  appliquée aux deux extrémités) ;
- la direction d’une sélection est conservée, avec Unicode et les textes répétés.

La dernière carte n’est pas une chaîne de versions à rejouer : vérifier ses gardes
et la consommer pour la publication correspondante. Une opération sans changement
peut laisser disponible la carte précédente. Les anciens paramètres gardés restent
refusés par les commandes, même si un contexte interne a été reporté depuis.

`ViewSync` conserve sélection et ancre du passage visible. Son report ne touche
pas aux demandes de révélation ni au focus. egui capture cette ancre dans le texte
publié, hors préédition IME, puis appelle le report avant d’adapter une publication.
Les pixels, le décalage dans la ligne visible et la restauration effective du
défilement à l’acceptation sont gérés par l’adaptateur egui de 6.4b.

## Candidats sans publication

`prepare_proposal` et `prepare_resolved_proposal` utilisent le même parcours de
préparation que les lots atomiques : commandes documentaires admises, frontières
protégées, garde complète, limite de 256 étapes, JSON de 4 Mio et budget de
ressources. Le résolveur portable travaille sur le candidat qui évolue ; comme
pour les lots existants, il doit se limiter à préparer les paramètres et les
imports autorisés, sans écriture externe.

Le brouillon démarre dans le contexte du participant. Il ne possède ni chemin de
sauvegarde ni verrou du fichier publié. Une sauvegarde finale est refusée avant
toute résolution ; les opérations de fichiers, d’historique et de récupération
restent interdites dans les étapes. Un échec tardif abandonne tout le candidat.

`PreparedProposal` expose le document candidat pour relecture, sa sélection logique
et son style final ; il garde aussi son instantané structuré et ses identités pour
l’acceptation. `map_for_review` donne une position logique dans le candidat,
sans faire sortir une référence gardée à une révision spéculative.

La préparation, même réussie, laisse document publié, contextes utilisateur/agent,
historique, révision et fichiers inchangés. Images, signature, métadonnées et modèle
restent privés au candidat. Abandonner l’objet ne fait rien au document.
`check_proposal` contrôle contexte, identité et révision de base : un déplacement
du curseur n’invalide pas la proposition ; toute édition publiée l’invalide, même
si ses positions sont reportables. Une autre invitation ne peut pas la reprendre.

## Publication après accord — socle de 6.4b

`Engine::accept_proposal` est une API Rust destinée à l’adaptateur qui a recueilli
l’accord humain. Elle ne constitue pas une collecte de consentement et ne doit pas
être exposée à l’agent. Elle revérifie le droit d’écriture, le contexte, l’identité
du document et la révision exacte avant de publier quoi que ce soit.

La validation du candidat complet, de ses positions et, si nécessaire, de la
politique du modèle est faite sur une session de préparation. Aucune opération
faillible ne suit son adoption. Le contenu, les métadonnées, le modèle et les
ressources sont publiés ensemble, avec une seule révision et une seule étape
d’annulation. Aucun fichier n’est enregistré ni exporté.

La sélection utilisateur **au moment de l’accord**, même déplacée pendant la
relecture, est reportée avec la carte cumulée du lot. Son style de frappe reste
inchangé. Le participant proposant reçoit sa sélection et son style finaux ; les
autres participants sont reportés normalement. L’issue demande seulement un rendu,
sans déplacement implicite vers le passage modifié. L’interface egui conserve
le focus et restaure le défilement lors de l’intégration 6.4b.

Un lot sans changement documentaire peut faire évoluer le seul contexte agent,
sans révision, document sale, annulation ni perte de la pile de rétablissement.
L’adaptateur doit retirer le lot en attente après succès ; abandonner un candidat
reste sans effet. Un échec de garde, un autre contexte ou une session en lecture
seule ne publie aucun changement, même après retour au même texte par annulation.

## Intégration 6.4b

Le cœur garde ses 49 commandes et 19 requêtes, sans transport. L’adaptateur MCP
ajoute `proposals.submit/get/cancel` uniquement en mode rattaché (70 outils contre
67 en autonome). Aucune approbation n’est exposée à l’agent. Le [contrat des
propositions](mcp-proposals.md) décrit la relecture native et les états bornés,
validés en D39 et livrés le 25 octobre 2026. Le serveur autonome conserve ses règles.

`ViewSync::reveal_at` valide une cible de navigation distincte de la sélection
utilisateur. Le repérage temporaire cesse lors d’un mouvement de sélection ou d’une
publication qui périme sa cible. L’interface restaure l’ancre visible et le focus,
sans changer la sélection quand l’utilisateur choisit « Voir ». Une vérification
de fraîcheur technique ne vaut jamais accord humain. Les fichiers sont livrés en 6.5 ; la
[recette agent réelle](mcp-attached-acceptance.md) est terminée en 6.6.

Les tests se trouvent dans `engine/tests/positions.rs`, `contexts.rs`, `proposals.rs`,
`engine/tests/proposals/acceptance.rs`,
`view_sync/tests/positions.rs` et `mcp/src/local/tests.rs`. La recette globale est
consignée dans [le bilan MCP local](mcp-local.md).
