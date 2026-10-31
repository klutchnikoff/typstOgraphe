# A.4 — Comparaison et décisions locales communes

Statut : périmètre et maquette interactive validés le 31 octobre 2026.
Implémentation du comparateur commun et des décisions locales en cours.

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
Revenir au même contenu par Annuler ne réactive pas une ancienne garde. Le contrat
détaillé des conflits, du recalcul éventuel et des gardes sera arrêté avant codage.

## Raccordement MCP

La proposition conserve une base et un candidat privés, complets et figés.
Les décisions locales emploient le socle ci-dessus ; l’agent ne peut pas les
accepter à la place de l’utilisateur. Soumettre ou consulter ne publie rien.
Seules les opérations acceptées rejoignent le document courant, sans sauvegarde
ni export implicite. Les propositions ne deviennent pas automatiquement des
versions confirmées et ne remplacent pas les annotations de relecture du chantier J.

F.3 adapte le protocole, les reçus et le cycle de vie des propositions. Il ne
reconstruit pas un deuxième moteur d’application locale. L’acceptation par lot
entier reste le comportement actuel tant que ce raccordement n’est pas livré.

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

## Points à préciser au fil de l’implémentation

- Présentation des dépendances, progression des décisions, conflits et éventuel
  recalcul explicite après une intervention extérieure.
- Bibliothèque de diff : évaluer une brique existante avant de la retenir ;
  l’ancien usage de `similar` ne constitue pas une sélection actuelle.
