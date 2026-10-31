# A.4 — Comparaison et décisions locales communes

Statut : périmètre et maquette interactive validés le 31 octobre 2026.
Comparateur commun et premier raccordement des décisions locales implémentés.
Voir le [bilan de validation](a4-comparison-results.md) et la
[maquette conservée](mockups/a4-comparison.html).

## Socle commun retenu

Comparer deux états complets et figés, puis permettre à l’utilisateur de choisir
les modifications à appliquer au document courant. Ce moteur est développé une
seule fois pour Versions et les propositions MCP. « Tout appliquer » peut rester
une commodité ; aucune validation globale n’est imposée par l’architecture.

Les états de comparaison comprennent contenu structuré, métadonnées, options,
modèle, dépendances et ressources. L’état courant choisi comme référence est
capturé temporairement, sans créer une entrée dans Versions. Il ne change pas
silencieusement pendant la lecture de la comparaison.

Le calcul porte sur les changements documentaires, sans exposer les octets Yjs.
Il couvre texte, mise en forme, liens, structure, tableaux, annotations, paramètres,
en-tête et ressources, avec un aperçu isolé de chaque état. Les correspondances
incertaines sont présentées prudemment ; le moteur ne déduit pas une identité ou
un déplacement de la seule proximité de deux blocs.

## Comparer et appliquer sont deux opérations distinctes

La comparaison décrit des différences entre deux états immuables. Le plan
applicable précise une source choisie, le document courant destinataire, ses
préconditions et les commandes gardées nécessaires. Une différence visuelle
n’est ni une commande documentaire ni une autorisation de publication.

La présentation conserve deux colonnes, les retraits et les ajouts explicitement
marqués, des groupes accompagnés de leur contexte et une navigation au clavier.
La convention confirmée est **avant à gauche, après à droite** :

| Parcours | Gauche — avant | Droite — après | Action sur le document courant |
| --- | --- | --- | --- |
| Versions | Version historique sélectionnée | Document courant figé à l’ouverture | Reprendre un passage de la gauche |
| Proposition MCP | État de départ figé | Proposition figée | Appliquer une modification de la droite |

L’ordre de lecture ne détermine donc pas la source de l’application. Le plan
nomme explicitement cette source et sa cible. Les actions « Reprendre ce passage
de la version » et « Appliquer cette modification » rendent cette distinction
visible ; elles ne modifient jamais les deux états affichés.

## Groupes applicables et dépendances

Un groupe visuel n’est pas automatiquement une unité de décision indépendante.
Une citation et la référence bibliographique qu’elle nécessite, ou une image et
sa ressource, doivent être traitées ensemble. Le plan regroupe les opérations
liées et vérifie la cohérence de l’état résultant avant publication. Une absence
de preuve d’indépendance impose un regroupement sûr, pas une application partielle
hasardeuse. Cela peut exceptionnellement produire une seule unité si tout est lié,
sans imposer ce traitement à tous les lots.

Une application locale modifie le document courant, pas les snapshots. Elle
publie atomiquement ses opérations et fournit une étape d’annulation cohérente.
Un refus laisse le document inchangé. Les actions disponibles et leurs libellés
dépendent du contexte Versions ou MCP ; le moteur et les composants restent communs.

## Progression et concurrence

Les états de départ et proposé restent inchangés après chaque décision.
Le contexte de travail suit séparément groupes appliqués, refusés et restants,
garde attendue et provenance des publications connues. Appliquer un premier groupe
ne doit pas rendre tous les suivants inutilisables au seul motif que la révision
vient de changer : le moteur connaît sa propre publication et doit revérifier
le reliquat sous la nouvelle garde, sans le rejouer aveuglément.

Une intervention extérieure ne bénéficie pas de cette progression connue.
Elle impose un contrôle et un réexamen explicites avant toute nouvelle application.
Revenir au même contenu par Annuler ne réactive pas une ancienne garde. Le premier raccordement bloque les publications restantes après une intervention
extérieure ; il ne recalcule ni ne réancre implicitement la paire. Versions demande
une nouvelle ouverture ; MCP demande une nouvelle proposition depuis une base relue.

## Raccordement MCP

La proposition conserve une base et un candidat privés, complets et figés.
Les décisions locales emploient le socle ci-dessus ; l’agent ne peut pas les
accepter à la place de l’utilisateur. Soumettre ou consulter ne publie rien.
Seules les opérations acceptées rejoignent le document courant, sans sauvegarde
ni export implicite. Les propositions ne deviennent pas automatiquement des
versions confirmées et ne remplacent pas les annotations de relecture du chantier J.

F.3 adapte le protocole, les reçus et le cycle de vie des propositions. Il ne
reconstruit pas un deuxième moteur d’application locale. La modale MCP réutilise maintenant les décisions locales. Les changements liés
restent atomiques ; une commodité permet d’appliquer les groupes restants.

## Maquette validée et règles du premier raccordement

La modale à 90 %, ses onglets Contenu / Aperçu / Comparer, ses deux colonnes et
les actions locales reprennent la maquette interactive validée. Les actions de
chaque groupe sont alignées à droite ; l’action principale est à l’extrémité
droite. Les dépendances sont indiquées dans la ligne de décision. Les états
restent figés et les décisions sont affichées séparément. Une intervention
extérieure conserve la consultation mais bloque les publications restantes.

Le premier raccordement autorise la reprise depuis une version vers le document
courant capturé à l’ouverture. Deux versions historiques se comparent en lecture
seule. Reprendre localement passe par l’historique d’annulation et la récupération
ordinaires, sans sauvegarde, export ou snapshot automatique. La restauration
complète et son snapshot `before_restore` restent un lot distinct.

Une différence de structure non identifiable sans ambiguïté reste groupée.
Les changements de paramètres, de ressources et de dépendances sont regroupés
avec le contenu associé lorsque leur indépendance n’est pas prouvée.

## Implémentation et limites explicites

`content::comparison::Comparison` possède la paire privée, les groupes et la garde
de travail. `prepare` produit un candidat opaque, lié à l’identité de cette
comparaison et à l’époque de ses décisions. `publish` recontrôle ces références et
passe par `EditingSession::publish_prepared`. Un candidat préparé pour une autre
paire, une décision concurrente ou une ancienne révision est refusé sans publication.

`similar` **3.2.1** a été retenu après évaluation de son API de séquences et de
mots Unicode : crate Rust sans backend natif supplémentaire, version déjà disponible
dans le cache local, maintien en amont et délai de calcul configurable. Les
[sources de la bibliothèque](https://github.com/mitsuhiko/similar) et son
[API de comparaison](https://docs.rs/similar/latest/similar/algorithms/index.html)
sont les références. L’alignement exact des blocs et le détail des mots utilisent
Myers avec échéance de 100 ms par alignement. Une limite atteinte produit un
regroupement conservateur ; elle n’autorise aucune correspondance approximative.

Le premier découpage applique les différences entre ancrages de blocs identiques.
Plusieurs modifications dans un même paragraphe ou widget restent une décision.
Si les réglages, la bibliothèque bibliographique, les ressources ou les paquets
changent, le contenu associé est regroupé en une seule décision documentaire :
le premier moteur ne prétend pas encore prouver des indépendances plus fines.
Les caractères, espaces significatives, liens et mises en forme sont conservés ;
les deux projections riches restent disponibles même lorsque le texte est identique.
Les fichiers modifiés sont inventoriés et les deux rendus Typst sont isolés.

L’adaptateur capture un checkpoint immuable sous verrou, puis prépare et compile
le candidat hors du verrou de la session active. La publication reprend le verrou
et revérifie garde, identité et décisions ; écrire entre la capture et la fin de
la compilation ne permet jamais d’écraser la nouvelle saisie. La fenêtre qui accepte
reçoit aussi le delta confirmé dans la réponse, indépendamment des notifications.

La référence aux dépendances épinglées participe au même historique causal que
les réglages, y compris si seul le contenu d’un paquet change. Cette référence
opaque est renouvelée pour chaque décision publiée : réutiliser une empreinte de
contenu pourrait confondre deux jeux après réouverture d’un ancien checkpoint.
Le propriétaire résout la référence avec la source du modèle et
retient les jeux immuables nécessaires à Annuler/Rétablir. La récupération invalide
son cache lors d’un changement de ce jeu. Les octets Yjs et ces références internes
ne constituent pas la présentation des différences.

La soumission MCP fige les archives complètes de base et de candidat, pas seulement
leurs textes. Une proposition devenue périmée avant sa première consultation
reste lisible. Les reçus exposent `decisions` et `workingRevision` ; `revision`
reste celle de la base initiale. `partially_accepted` est terminal quand des groupes
ont été appliqués et les autres refusés. Aucun outil MCP ne peut décider ; aucun
accord ne sauvegarde ni n’exporte implicitement.

Les comparaisons restent temporaires en mémoire, hors `.typsto` et hors récupération
des formulaires. Fermer une consultation Versions libère son contexte ; fermer la
modale d’une proposition en attente conserve ses décisions pendant l’invitation.
Après intervention extérieure, un retour au même texte par Annuler ne réactive pas
la garde. Aucun rebase automatique ni reprise vers un tiers état n’est livré.
Une collision entre deux ressources d’image immuables de même chemin et de contenu
différent est refusée ; l’import normal utilise des chemins fondés sur le contenu.
