# A.3 — Interface de reprise et décisions de fermeture

**Parcours et maquette validés le 31 octobre 2026 ; implémentation livrée.**
Le [bilan de recette](a3-recovery-results.md) précise les vérifications et limites.
Ce document complète le [contrat de conservation](a3-conservation.md) et son
[premier lot livré](a3-conservation-results.md).

## Accueil

L’application continue de démarrer sans ouvrir automatiquement un projet.
Une section **Travail à reprendre**, distincte des projets récents, affiche le nom,
le type, la date de conservation, les modifications non enregistrées et le nombre
de saisies en cours. Les copies provenant de fenêtres différentes sont identifiables
séparément sous le projet lorsque leur identité est établie ; elles ne sont pas
fusionnées automatiquement.

L’action principale est **Reprendre**. Une corbeille permet de supprimer une copie
après confirmation, avec une explication de la perte des seules données non
conservées ailleurs. Fermer la présentation ou ignorer cette section ne supprime
aucune copie. La suppression d’une copie ne consomme pas celles d’autres fenêtres.

## Document récupéré et saisies privées

Reprendre ouvre une **copie indépendante** du document confirmé conservé, avec
l’indication discrète **Document récupéré**. Le premier enregistrement demande un
emplacement ; aucune réécriture automatique du fichier d’origine. Une éventuelle
modification externe du fichier ne justifie donc jamais une fusion implicite.

Le panneau temporaire **Saisies à reprendre**, accessible depuis la barre latérale,
énumère les éléments encore privés : auteur/affiliation, note, lien, paramètres de
widget, champ d’en-tête, texte refusé… Le fermer ne les abandonne pas.

Sélectionner un élément réutilise son interface habituelle : fenêtre centrée pour
un auteur ou une affiliation ; bandeau contextualisé pour une note, un lien ou un
widget ; onglet et champ appropriés pour l’en-tête. Le focus rejoint la saisie et
revient au déclencheur en quittant son interface. Les flèches permettent de naviguer
dans les propositions ; les champs de texte conservent leurs touches d’édition.

Ouvrir une saisie récupérée ne l’applique pas. **Valider** utilise les contrôles
communs du moteur avant publication. Une validation échouée conserve les valeurs
et affiche son erreur. **Annuler** ou Échap quitte la reprise de ce formulaire
sans consommer la copie récupérée : son abandon durable est une action explicite.
La saisie n’est retirée des éléments à reprendre qu’après conservation durable de
son résultat validé ou abandon explicite. Aucun effacement à la seule ouverture.

Une cible disparue ou devenue incompatible laisse le brouillon consultable,
copiable et exportable avec une explication. Le texte refusé revient dans un
espace de récupération isolé, sans action globale d’application automatique. Les
brouillons exportés doivent pouvoir être rouverts par le parcours de reprise ;
leurs données restent non fiables et sont revalidées avant publication.

## Fermeture

| État de la fenêtre | Décision |
| --- | --- |
| Tout est enregistré, aucune saisie en attente | Fermeture directe. |
| Modifications documentaires ordinaires non enregistrées | **Fermer sans enregistrer** · **Annuler** · **Enregistrer et fermer**. |
| Formulaire ouvert, champ en attente ou brouillon refusé | **Abandonner et fermer** · **Annuler** · **Conserver et fermer**. |

La dernière action est celle par défaut, à l’extrême droite ; la destruction reste
à gauche. Pour une saisie en attente, le texte précise le nombre d’éléments et le
fait que leur conservation est locale : **le fichier `.typsto` ne sera pas modifié**.
Enregistrer n’entraîne jamais la validation implicite des formulaires ouverts.

Une erreur de conservation ou d’enregistrement laisse la fenêtre ouverte et le
travail intact, avec une erreur et une nouvelle tentative explicites. Annuler un
sélecteur de fichier annule également la fermeture qui en dépendait. Une composition
de clavier doit être terminée avant toute validation de la saisie correspondante.
Échap annule le dialogue ; Entrée déclenche son action par défaut lorsque les règles
des champs le permettent. Ctrl/Cmd + Entrée conserve le contrat commun des formulaires.

## Plusieurs fenêtres et transitions

Fermer une vue ne revient pas à fermer le projet dans les autres vues et ne révoque
pas leurs modifications documentaires confirmées. L’abandon depuis une fenêtre ne
supprime jamais les saisies privées d’une autre fenêtre.

Une transition remplaçant le document de plusieurs fenêtres attend leur conservation
avant de basculer. Une annulation ou un échec empêche cette transition. Ce mécanisme
complète le premier lot : chaque fenêtre conserve sa dernière saisie avant le
basculement, même si le délai de copie automatique n’est pas encore atteint. Les
interfaces sont temporairement bloquées après la fin d’une composition ; une
composition encore active, un échec ou une annulation empêche le basculement.

La fermeture de l’application avec plusieurs fenêtres doit expliciter la portée de
la décision ; elle ne transforme pas l’abandon local en suppression globale des
copies privées.

## Maquette validée

La maquette validée permet d’explorer l’accueil, une reprise Article, les formulaires
d’auteur et de note, leur validation ou leur fermeture, puis les trois choix de
fermeture. Les états de texte refusé, cible disparue et erreur de conservation font
également partie de la revue. Les opérations y sont simulées et ne touchent pas les
fichiers de l’utilisateur. L’implémentation reprend ces parcours avec les formulaires
et commandes ordinaires.
Elle ne clôt pas les recettes natives multisystèmes restant dans A.3.
