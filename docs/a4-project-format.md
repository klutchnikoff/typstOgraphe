# A.4 — Format cible du projet `.typsto`

**Statut : organisation et budgets validés le 31 octobre 2026 ; intégration à faire.**
Le **CAS est retenu** après [évaluation](a4-snapshot-storage-evaluation.md).
L’organisation ci-dessous précise cette décision ; elle n’est pas encore le
format lu ou écrit par l’application. Le lecteur actuel attend la version 10.
La version 11 est retenue pour cette rupture, sans lecteur de compatibilité
préventif ni conversion silencieuse des anciens projets de développement.
Les exemples seront régénérés explicitement au moment de l’intégration.

## Conteneur et organisation

Un seul fichier ZIP, portant l’extension `.typsto`, reste autonome et utilisable
hors ligne. État courant et snapshots partagent **le même magasin CAS** :

```text
project.typsto
├── manifest.json
├── versions.json
└── objects/
    └── sha256/
        ├── <empreinte sur 64 caractères hexadécimaux>
        └── …
```

Les noms et champs du format sont anglais. Les objets contiennent les octets
originaux de JSON, mises à jour Yjs complètes, sources Typst, images ou fichiers
bibliographiques et dépendances. Leur empreinte SHA-256 est calculée **avant
compression ZIP** ; une référence prend la forme `sha256:<empreinte>`.
Chaque objet est stocké une seule fois et ne change jamais sous son empreinte.
Les JSON produits sont sérialisés de façon déterministe ; la lecture vérifie
les octets conservés sans les normaliser ni les réécrire avant contrôle.

Le conteneur n’est pas directement un répertoire Typst compilable après
décompression. L’**export Typst** matérialise les sources et chemins logiques de
l’état choisi dans un répertoire compilable, avec ses seules ressources utiles.
Cela évite de conserver une deuxième copie physique de l’état courant à la
racine du projet. Le PDF reste un export, pas un objet obligatoire de l’archive.

Préférences globales, profil personnel non incorporé au document, formulaires
ouverts et autres brouillons privés restent hors du `.typsto`. Une signature
incorporée devient au contraire une ressource autonome du document.

## Manifeste du projet

`manifest.json` identifie le format et pointe vers le descripteur de l’état courant :

| Champ | Rôle |
| --- | --- |
| `format` | Valeur fixe `typstOgraphe`. |
| `version` | Version du conteneur ; `11` retenue. |
| `project_id` | UUID stable indépendant du chemin du fichier. |
| `current` | Référence CAS du descripteur de l’état courant confirmé. |

`versions.json` est l’index connu de cette version de conteneur ; le manifeste
n’a pas à réénumérer les objets ou ressources. L’identité du projet ne remplace
ni le verrou du fichier ni l’identité/révision d’une session d’édition.
Déplacer ou renommer le fichier ne change pas son identité. La politique de
nouvelle identité pour une copie indépendante reste à préciser avec ce parcours.

## Descripteur immuable d’un état

L’état courant et chaque snapshot pointent vers un descripteur de même forme,
lui-même objet CAS. Deux versions dont l’état est identique peuvent donc partager
ce descripteur tout en conservant leurs identifiants, dates et noms distincts.

| Champ du descripteur | Contenu |
| --- | --- |
| `version` | Version du schéma du descripteur, initialement `1`. |
| `contract_version` | Version du contrat documentaire structuré. |
| `document_type` | Identifiant du type intégré, contrôlé contre les métadonnées. |
| `document` | Référence du corps structuré JSON. |
| `metadata` | Référence des métadonnées et options documentaires JSON. |
| `yjs` | Référence d’un checkpoint complet, avec encodage déclaré `yjs-v1`. |
| `model` | Identifiant de provenance et référence du source Typst du modèle. |
| `sources` | Références des sources dérivées `main.typ` et `body.typ`. |
| `resources` | Référence d’un manifeste de ressources partagé. |
| `producer` | Versions de l’application/générateur et du moteur Typst, à titre de diagnostic. |

Le **checkpoint Yjs confirmé et sa projection structurée doivent être cohérents**,
comme dans le contrat existant. Les métadonnées et le modèle doivent également
correspondre aux valeurs documentaires partagées. Un arbre ou un source Typst
isolé n’autorise pas à remplacer silencieusement un état partagé contradictoire.
Les sources Typst sont conservées mais restent dérivées, hors de la voie d’édition.

Le même descripteur complet sert aux versions conservées, aux références internes
de relecture et aux états privés base/candidat MCP. Ce partage de forme ne leur
donne ni la même visibilité, ni le même cycle de vie, ni les mêmes droits.

Le checkpoint est obligatoire aussi en mode individuel ; une simple frontière
causale, un vecteur d’état ou un différentiel dépendant d’un autre état ne suffit
pas. Il conserve les identités nécessaires à Yjs/Yrs. La pile d’annulation en
séance et les contextes privés d’édition ne sont pas persistés comme snapshots.

La déduplication porte sur les octets identiques : elle ne supprime pas les
représentations distinctes nécessaires aux contrôles de cohérence Yjs/JSON/Typst.
Le premier schéma conserve notamment la représentation métier actuelle des
références bibliographiques ; il n’exige pas une refonte du contrat d’édition.

## Ressources et dépendances

Le manifeste de ressources associe chaque **chemin logique** à une référence
CAS : images et signature incorporée, fichiers `.bib`/Hayagriva, wrappers Typst,
packages, inventaire `dependencies.json`, licences et provenance nécessaires.
Les modèles continuent à employer leurs chemins logiques habituels ; ils
n’utilisent jamais directement les empreintes physiques de l’archive.

Un manifeste identique est partagé entre les états. Remplacer une image crée un
nouvel objet ; les snapshots précédents continuent à référencer l’ancienne.
Modifier une dépendance ou le modèle suit la même règle, sans remplacement au
chargement par la version du catalogue de l’application.

**Point d’intégration identifié :** `packages::validate` et `valid_path`
s’appuient actuellement sur le catalogue embarqué de l’application. Le nouveau
lecteur devra contrôler les dépendances conservées à partir de leur inventaire
embarqué et de leurs objets, avec des chemins et limites vérifiés. Changer le
catalogue installé ne doit pas invalider une ancienne version autonome.
Ce point n’a pas été qualifié par l’essai de stockage, qui utilise les packages
actuels ; il demande sa propre recette avant livraison.

Conserver les sources et ressources ne revient pas à embarquer les binaires de
Typst, l’ensemble des polices du système ou le système d’exploitation. Les
polices dont un modèle exige l’embarquement devront être déclarées comme
ressources du projet. Une identité absolue du PDF entre versions du moteur ou
machines n’est pas promise par ce premier schéma.

## Index des versions et références internes de relecture

`versions.json` porte sa version de schéma et deux collections distinctes :

- `snapshots` : les versions affichées dans **Versions**, créées manuellement ou
  conservées automatiquement avant restauration ;
- `review_references` : les snapshots de référence internes à **Relectures**,
  gérés avec leur relecture et absents de la liste ordinaire des versions.

Ces collections partagent les mêmes descripteurs et objets CAS. Deux entrées
latérales restent distinctes : le partage du stockage ne définit pas les outils
de relecture, qui restent ceux du chantier J.

Chaque entrée de `snapshots` contient :

- un `id` UUID stable, indépendant de son nom et du descripteur CAS ;
- `created_at`, instant UTC automatique, affiché selon la locale de l’interface ;
- `name`, nom libre modifiable, daté par défaut ;
- `tags`, liste facultative, et `description`, texte facultatif ;
- `creator`, identité disponible du créateur, sans attribution de toutes les modifications ;
- `reason` : `manual` ou `before_restore` ;
- `state`, référence CAS immuable de son descripteur.

Le classement chronologique utilise la date, avec l’identifiant comme départage
stable. Ni l’ordre de l’index, ni le nom, ni les tags ne définissent le contenu
d’une version. Renommer ou étiqueter ne crée pas un nouveau snapshot et ne
change ni sa date, ni son créateur, ni sa référence d’état.

Chaque référence interne associe un identifiant de snapshot, un `review_id`, un
instant de création, une référence `state` immuable et une indication d’activité.
Elle est créée automatiquement au démarrage d’une nouvelle relecture, conservée
et protégée pendant toute sa durée. Reprendre une relecture existante ne recrée
pas sa référence. Son gestionnaire refuse la suppression tant qu’elle est active ;
une simple étiquette « relecture » dans une version manuelle ne crée pas ce lien.

La collection reste vide tant qu’aucune relecture n’existe. Une version visible
et une référence interne peuvent pointer vers le même état CAS : supprimer la
ligne de version ne libère pas l’état encore référencé par la relecture. Le
stockage des suggestions/commentaires et la rétention après clôture restent à
préciser dans J. Ce format n’impose ni nouvelle action « Comparer à l’état
initial », ni usage de la modale Versions dans les outils de relecture.

## Propositions MCP — paire d’états privés

Une proposition soumise est fondée sur **deux états figés explicitement liés** :

- `base` : l’état confirmé depuis lequel l’agent a travaillé, issu d’un snapshot
  choisi ou capturé au début du lot, avec sa provenance et sa garde documentaire ;
- `candidate` : l’état candidat complet, construit et vérifié par le moteur
  lorsque l’agent soumet son travail ; il est explicitement fondé sur `base`.

Ces noms décrivent les rôles du contrat cible, sans prétendre que le protocole
MCP de production possède déjà ces nouveaux champs. L’enveloppe de proposition
conserve identité du projet/session, garde document/révision et opérations
documentaires nécessaires à sa publication. Une différence visuelle entre deux
snapshots n’est pas à elle seule une autorisation ni un plan d’édition.

Les états emploient les mêmes descripteurs et objets CAS que les versions, mais
dans l’espace privé de la proposition, **hors de `versions.json` et du `.typsto`
confirmé**. Le moteur peut réutiliser des objets confirmés en lecture seule ;
la session doit en garder une vue complète et disponible jusqu’à la décision,
y compris si un nettoyage du fichier conserve d’autres racines entre-temps.
Aucun manifeste confirmé ne pointe vers des objets seulement privés.

La soumission ne modifie pas le document courant et ne crée ni entrée Versions,
ni relecture J, ni enregistrement implicite. Le candidat reste privé jusqu’à
acceptation. Celle-ci passe par les commandes gardées du moteur ; sauvegarde et
export conservent leurs autorisations distinctes. La conservation durable
éventuelle d’une proposition après décision reste à préciser avec F : cette
décision n’ajoute pas une origine visible « agent » ou une sauvegarde automatique.
Le refus ne publie aucune modification. L’acceptation reste celle du lot entier
jusqu’à F.3 ; les groupes indépendants y seront traités sans réécrire les états
initiaux ou abandonner les vérifications de concurrence.

La modale de consultation/comparaison est commune avec Versions, avec les
onglets Contenu, Aperçu et Comparer. Son contexte MCP choisit par défaut
**« État de départ → Proposition »**, et expose Accepter/Refuser plutôt que les
actions de restauration/suppression d’une version conservée. Les aperçus et
ressources des deux états restent isolés du document courant. Les intitulés
ne prétendent pas que la base est encore le document actuel si celui-ci a évolué.

Si la révision courante change, la proposition devient périmée pour publication ;
la comparaison des deux états figés reste consultable, sans fusion ni réancrage
automatique. Revenir au même contenu ou à la même empreinte par Annuler ne
réactive pas la garde. Une proposition applicable doit être préparée à partir
d’une base et d’une garde actualisées, conformément à la concurrence stricte
existante. Ces règles ne relâchent pas les contrôles actuels de soumission.

## Parcours utilisateur distincts

La barre latérale distingue **Versions** et **Relectures** :

- Versions affiche les états conservés par origine visible, avec ordre
  chronologique dans chaque groupe. Le clic ouvre une modale centrée à 90 % de
  la fenêtre, sur fond atténué et légèrement flouté ; Contenu, Aperçu et Comparer
  occupent sa barre supérieure, et le sélecteur d’état apparaît en comparaison.
  La comparaison d’une version prend le document courant par défaut ou une autre
  version explicitement choisie. Restauration et suppression se confirment dans
  la même modale, sans empiler un second dialogue.
- Relectures conserve les outils d’annotations définis dans J. Le snapshot
  technique initial ne détermine pas son interface et n’ajoute pas une action
  de comparaison non validée.

Les propositions MCP rejoignent le parcours de consultation d’états de Versions,
avec la paire et les décisions propres au contexte MCP décrites ci-dessus ;
elles ne sont pas assimilées à une relecture J par ce mécanisme. Échap ferme
d’abord une confirmation, puis la consultation ; le focus revient à l’élément
d’ouverture du contexte concerné. Ce partage porte sur la présentation et le
calcul des différences, pas sur l’identité des opérations autorisées.

## Sauvegarde, création, restauration et suppression

L’index des versions reste extérieur à l’arbre documentaire et au checkpoint
Yjs : un état ne contient pas récursivement son propre historique. Sa gestion
appartient au propriétaire Rust, via des opérations gardées accessibles aux
adaptateurs autorisés ; aucun adaptateur ne modifie directement le ZIP.

- **Sauvegarder** prépare l’état confirmé, ses objets et le catalogue inchangé,
  puis remplace atomiquement le fichier. Aucun snapshot implicite.
- **Créer une version** ajoute son entrée et enregistre l’état courant dans la
  même publication du `.typsto`, après choix d’emplacement si nécessaire.
- **Restaurer** prépare une version `before_restore` de l’état courant et la
  restauration dans une même opération protégée. En cas d’échec, les deux sont
  refusées. Restaurer ne consiste pas à injecter l’ancien binaire Yjs ou à
  ramener une révision en arrière : le moteur publie un nouvel état courant en
  préservant les garanties de concurrence et les autres versions conservées.
- **Supprimer une version** retire seulement une entrée autorisée de `snapshots`,
  sans pouvoir détacher une référence active de `review_references`, puis ne nettoie que les
  objets devenus inaccessibles depuis l’état courant, tous les snapshots retenus
  et les références protégées. Aucun effacement automatique de versions.

La création, suppression ou restauration ne doit pas laisser dans le fichier un
index partiellement publié. Gardes de révision, contrôle d’une modification
externe et verrou coopératif restent applicables. Sauvegarde et export gardent
leurs autorisations propres ; le MCP rattaché ne gagne aucun droit implicite.

## Lecture sélective, intégrité et erreurs

Ouvrir le projet charge manifeste, index et état courant validé ; consulter une
ancienne version charge ensuite ses seuls descripteurs et objets nécessaires.
Il n’est pas nécessaire de décompresser ou compiler tous les snapshots pour
afficher la liste. Les vérifications d’intégrité des objets sont faites lors
de leur lecture, et la cohérence documentaire lors de l’utilisation d’un état.

Une erreur dans l’état courant interdit sa publication dans l’éditeur. Une
ancienne version défectueuse doit pouvoir être signalée comme indisponible
sans bloquer la consultation d’un état courant valide, lorsque manifeste et
index restent interprétables. Aucune réparation ou suppression silencieuse.
Une sauvegarde conserve l’historique ; si une réécriture sûre échoue, le fichier
original reste intact et une copie indépendante de l’état valide est une issue
explicite, pas un nettoyage implicite des anciennes versions.

Le nettoyage est refusé si des descripteurs conservés ne permettent pas
d’établir leurs références avec certitude. Le lecteur contrôle aussi doublons,
chemins, schémas, tailles annoncées/réelles et références manquantes. Les erreurs
restent typées jusqu’aux adaptateurs Fluent, avec leur repli anglais.

## Budgets retenus et traitement des limites

Des limites distinctes évitent de confondre taille du document et poids de son
historique. **Budgets retenus :** conserver le plafond
de 64 Mio par état documentaire, 32 Mio par checkpoint Yjs et 16 Mio par image ;
autoriser jusqu’à 512 Mio sur disque et décompressés pour l’archive, 16 384 objets
et **100 versions affichées maximum** (`manual`, `before_restore`). Le quota
porte sur les entrées de `snapshots`, pas sur le nombre d’objets CAS. L’état
courant et les références internes de `review_references` n’y sont pas comptés.
Les paires privées MCP sont également hors quota ; leurs limites et leur
nettoyage relèvent de l’espace de proposition. Le plafond du fichier s’applique
aux données qui y sont effectivement conservées, sans faire passer ce budget
pour un budget global des espaces privés. Leurs bornes propres restent à
préciser avec F avant intégration, en conservant les limites de session existantes.
Un plafond est un refus explicite sans perte, jamais une autorisation de supprimer
des versions. Ces budgets doivent être vérifiés lors de
l’intégration ; ils ne sont pas une garantie de latence sur ces volumes.

À 100 versions affichées, l’édition et la sauvegarde ordinaire restent possibles dans les
limites de volume. Pour créer une nouvelle version, l’utilisateur est invité à
supprimer explicitement une version non protégée. Aucun roulement automatique.
Si une restauration exige une sauvegarde `before_restore` et qu’aucune place n’est
disponible, la restauration est refusée avant toute modification.
Une référence de relecture ne consomme pas silencieusement une place visible :
sa rétention et ses limites propres restent à préciser dans J, sous les plafonds
de volume et d’objets du projet. Atteindre ceux-ci refuse une nouvelle référence
sans supprimer un état conservé ni commencer une relecture sans sa base.

La maquette de Versions doit maintenant concrétiser :

1. Le conteneur ZIP entièrement CAS, partagé avec l’état courant ; le répertoire
   directement compilable est produit par l’export, sans copie parallèle à la racine.
2. Le descripteur commun complet et vérifié, l’index séparé, le contenu des versions
   immuable et leurs noms/tags modifiables.
3. La lecture sélective, le traitement explicite d’une ancienne version abîmée
   et les limites retenues, avant les recettes de limites et de conservation.

La sélection du moteur de diff et la validation partielle des propositions MCP
restent séparées de ce format ; F.3 ne bloque pas A.4.
