# Contextes d’édition et historique — A.2

Ce lot ajoute `content::editing::EditingSession` au cœur Rust. Il complète les
[commandes communes](structured-commands.md) et respecte le
[contrat structuré](structured-contract.md). Il est exécuté dans les recettes
Rust et Rust ↔ Yjs/Tiptap ; il ne remplace pas encore les historiques et le relais
de frappe de l’application, de la CLI ou de MCP.

## Propriété des données

`EditingSession` possède un `SharedSession` confirmé et des contextes privés.
Chaque contexte conserve une réplique Yrs durable, son gestionnaire d’annulation,
une sélection et ses styles de frappe. Rien de cela n’ajoute de clés privées à
l’état partagé ou au `.typsto`. Les contextes sont bornés à 64 par session et leurs
identifiants ne sont pas réutilisés après fermeture.

Un identifiant de contexte n’est pas une autorisation. L’hôte associe les
participants authentifiés à leurs contextes, vérifie leurs permissions et
sérialise les appels. Le focus, les panneaux et la préédition IME restent à
l’adaptateur. Aucun contexte ne peut demander de prendre le focus humain.

L’API commune propose :

- `open_context`, `close_context`, `context` : cycle de vie et état privé ;
- `select`, `typing_style`, `insert` : sélection dans une zone et saisie avec les
  styles propres au contexte ;
- `execute` : lot de commandes documentaires explicites ;
- `execute_updates` : lot de mises à jour attribué au contexte par l’hôte ;
- `publish_updates` : publications sans attribution à un historique local ;
- `history` : annulation ou rétablissement du contexte ;
- `rejected_draft` : consultation d’une proposition privée refusée.

Tous ces appels vérifient la garde document/révision. L’ouverture d’un contexte,
la navigation et les styles de frappe ne modifient ni la révision documentaire,
ni l’historique, ni la possibilité de rétablir un lot. Les sélections sont pour
l’instant des plages ordonnées dans une seule zone, sans sélection discontinue.

## Sélection et styles

Le résolveur de zones, les frontières et les offsets UTF-16 sont ceux des
commandes communes. Les ancres internes utilisent les positions relatives de
Yrs, avec l’identité du bloc ou du champ ; aucun identifiant Yrs n’est exposé
comme cible publique.

Une insertion de bloc avant la zone ou une modification du texte peut déplacer
les offsets et chemins présentés par `context`. Cela ne réancre jamais une
commande préparée : sa garde stricte reste nécessaire. Une zone reconstruite ou
supprimée n’est pas remplacée par son nouveau voisin. La sélection devient
indisponible et `selection_lost` l’indique ; une insertion exige alors une
nouvelle sélection explicite. Un découpage qui reconstruit une partie du texte
ne transporte pas automatiquement la sélection vers une autre zone.

Pour un champ scalaire ou le texte d’une note actuellement conservé comme
attribut, on refuse de réutiliser une sélection si sa valeur a changé. Le contexte
ne devine pas des positions relatives que cette représentation ne fournit pas.
Les textes partagés et les champs riches utilisent leurs ancres Yrs.

Les styles suivent le contexte, jamais un autre participant. `insert` les utilise
uniquement dans les zones riches et replace le point d’insertion après le texte.
Dans un champ simple, l’insertion reste en texte brut ; demander un nouveau style
y est refusé. Les styles explicites des commandes `text.insert` restent explicites.

## Historique par participant

Le gestionnaire d’annulation est celui de **Yrs 0.28**, déjà utilisé par le projet.
Chaque réplique suit uniquement les publications attribuées à son contexte. Les
mises à jour des autres participants sont reçues sans capture ; elles ne vident
pas sa pile de rétablissement. Un nouveau lot local suit les règles de Yrs pour
la pile de rétablissement.

Un lot confirmé est importé dans la réplique en une transaction suivie : une
seule étape d’annulation couvre toutes ses zones. Les commandes sans changement
et les refus avant publication ne créent pas d’étape. Le groupement naturel de
la frappe reste à raccorder avec le relais ; ce lot ne remplace pas ce groupement
par un délai décidé dans le moteur.

La réplique est durable parce qu’un état Yjs transmis ne conserve pas les liens
internes `redone` de Yrs. Reconstruire le gestionnaire à chaque commande à partir
d’un checkpoint et de ses piles casserait certaines suites « insertion,
suppression, annuler deux fois, rétablir deux fois ». La recette couvre cette
suite avec une contribution distante intercalée.

La synchronisation privée est évitée si la révision n’a pas changé. Sinon elle
importe le différentiel nécessaire, sans reconstruire la réplique ni son
historique. Aucun coût supplémentaire n’est introduit dans la frappe de
l’application par ce lot, dont le raccordement de production reste à faire.

## Annulation refusée et brouillon privé

Une annulation est calculée dans la réplique du contexte. Son différentiel
repasse par la préparation et la validation finale de `SharedSession`, puis par
sa publication gardée. La révision augmente seulement pour une modification
confirmée.

Une protection supplémentaire refuse les suppressions d’objets introduits par
un autre participant. Par exemple : A insère un tableau, B écrit dans une cellule,
puis A annule l’insertion du tableau. Annuler le conteneur ferait aussi disparaître
le texte de B ; le moteur refuse au lieu de le masquer silencieusement. Cette
protection peut également concerner une première saisie dans une zone vide,
lorsqu’elle crée un conteneur de texte ensuite enrichi par quelqu’un d’autre.

La validation refuse aussi un résultat documentaire invalide : par exemple,
retirer une affiliation qu’un autre auteur utilise désormais.

En cas de refus :

- le document confirmé, sa révision et les autres contextes restent intacts ;
- le contexte conserve le différentiel proposé, sa garde de départ, la cause typée
  et, si sa projection est possible, son état documentaire privé ;
- il est marqué `pending` et bloque les nouvelles éditions jusqu’à traitement ;
- sa sélection privée n’est pas présentée comme une cible du document confirmé ;
- les autres participants peuvent poursuivre leur travail.

Aucun rétablissement forcé ou abandon de brouillon n’est automatique. Fermer un
contexte en abandonne explicitement les données ; l’hôte doit faire confirmer
cette décision si nécessaire. Le parcours d’examen, de remise en cohérence et de
reprise de ces brouillons doit être validé et construit avant le raccordement
à l’interface. Ce mécanisme ne constitue pas un contournement de l’acceptation
humaine des propositions MCP.

Les erreurs de contexte ont le code `editing_context`, avec les motifs `closed`,
`pending_draft`, `selection_required` et `history_conflict`. Leur présentation
utilise Fluent, avec repli anglais, sans traduction dans le cœur.

## Recette et suite

Les tests couvrent sélections et styles isolés, Unicode et surrogates, champs
simples et riches, notes, cellules et légendes, déplacement des chemins, perte de
cible, lots multizones, annulations successives, rétablissement après une édition
distante, mises à jour attribuées, refus structurels et dépassement de révision.

Le transport de diagnostic exécute un scénario de deux participants dans cette
API. Ses publications sont appliquées au véritable document Yjs/Tiptap ; chaque
état est relu par Rust et le curseur graphique reste indépendant.

Restent le raccordement aux adaptateurs, le groupement de frappe, le parcours
utilisateur des brouillons refusés et la vérification d’un seul propriétaire
d’historique par participant. L’interface conserve son gestionnaire Yjs existant
jusqu’à cette migration : on ne superpose pas les deux gestionnaires en production.
Les historiques de séance ne sont pas des snapshots durables et ne sont pas
rétablis par l’ouverture d’un `.typsto`.
