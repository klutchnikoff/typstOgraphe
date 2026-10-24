# Contextes d’édition, positions et propositions — lot 6.3

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
| `query_in_context(handle, query)` | Résoudre `Current` dans ce contexte ; adapter sélection/style d’`inspect` et disponibilité de `commands.list`. |

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
Toutes les commandes MCP rattachées restent refusées en 6.3, y compris la sélection.
Le transfert volontaire d’une sélection depuis l’interface viendra avec le parcours
de proposition ; aucun suivi implicite du curseur n’est introduit.

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
défilement lors d’une future acceptation restent dans l’adaptateur, en 6.4.

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
la future acceptation. `map_for_review` donne une position logique dans le candidat,
sans faire sortir une référence gardée à une révision spéculative.

La préparation, même réussie, laisse document publié, contextes utilisateur/agent,
historique, révision et fichiers inchangés. Images, signature, métadonnées et modèle
restent privés au candidat. Abandonner l’objet ne fait rien au document.
`check_proposal` contrôle contexte, identité et révision de base : un déplacement
du curseur n’invalide pas la proposition ; toute édition publiée l’invalide, même
si ses positions sont reportables. Une autre invitation ne peut pas la reprendre.

## Limites avant 6.4

Ce lot ne définit aucun outil MCP de proposition ou d’approbation, ni fonction de
publication d’un candidat. Les 49 commandes et 19 requêtes du cœur, les 67 outils
et les cinq ressources MCP conservent leurs schémas. Le serveur autonome continue
à appliquer ses commandes habituelles.

En 6.4, il faudra valider ensemble la présentation des différences, puis intégrer
le stockage des propositions en attente, l’acceptation/refus explicites du lot
entier, la revalidation à l’acceptation et une unique étape d’annulation. Le report
de la sélection et du passage visible utilisateur utilisera alors la carte cumulée
du candidat ; son style et son focus devront être conservés. Une vérification de
fraîcheur technique ne vaut jamais accord humain.

Les tests se trouvent dans `engine/tests/positions.rs`, `contexts.rs`, `proposals.rs`,
`view_sync/tests/positions.rs` et `mcp/src/local/tests.rs`. La recette globale est
consignée dans [le bilan MCP local](mcp-local.md).
