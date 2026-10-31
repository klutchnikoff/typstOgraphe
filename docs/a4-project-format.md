# A.4 — Format cible du projet `.typsto`

**Statut : stockage CAS, adaptateurs, Versions et comparateur commun avec reprise complète implémentés le 31 octobre 2026.**
Le **CAS est retenu** après [évaluation](a4-snapshot-storage-evaluation.md).
L’organisation ci-dessous est implémentée par `content::project::Project`.
Tauri, CLI et MCP lisent et écrivent ce même format 11 ; le lecteur/écrivain
de format 10 a été retiré.
La version 11 est retenue pour cette rupture, sans lecteur de compatibilité
préventif ni conversion silencieuse des anciens projets de développement.
L’exemple Article a été régénéré explicitement depuis son contenu versionné.

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
Déplacer ou renommer le fichier ne change pas son identité. **Règle validée :**
une sauvegarde ordinaire conserve `project_id` ; Enregistrer sous ou créer une
copie indépendante depuis l’application produit un nouveau `project_id`, en
conservant les versions, leurs identifiants et leurs métadonnées. Les identifiants
de versions sont interprétés dans le projet concerné. Une copie externe du
fichier ne constitue pas une autorisation de partager une session ou un verrou.

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
| `sources` | Références `main` et `body` des sources dérivées ; chemins logiques actuels `main.typ` et `contenu.typ` à l’export. |
| `resources` | Référence d’un manifeste de ressources partagé. |
| `producer` | Numéro du générateur contrôlé lors de la lecture ; versions de l’application et de Typst à titre de diagnostic. |

Le **checkpoint Yjs confirmé et sa projection structurée doivent être cohérents**,
comme dans le contrat existant. Les métadonnées et le modèle doivent également
correspondre aux valeurs documentaires partagées. Un arbre ou un source Typst
isolé n’autorise pas à remplacer silencieusement un état partagé contradictoire.
Les sources Typst sont conservées mais restent dérivées, hors de la voie d’édition.
Le générateur courant est **2** : les apostrophes et guillemets du
texte rédigé gardent la syntaxe `smartquote` de Typst, au lieu d’être échappés.
Le modèle et la langue du document déterminent les caractères rendus. Les
chaînes de code (chemins, URL et paramètres) restent encodées séparément.
Les générations antérieures ou inconnues sont refusées ; les exemples de cette
phase de développement sont régénérés explicitement, sans lecteur de compatibilité.

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
Le modèle suit la même règle. Les copies de paquets historiques peuvent être
conservées comme ressources de transport ; elles ne sélectionnent pas les
bibliothèques exécutées par l’application.

Le lecteur CAS utilise `packages::validate_embedded`, indépendant du catalogue
installé : inventaire embarqué de schéma 1, identifiants et racines uniques,
chemins relatifs bornés, dépendances déclarées présentes, points d’entrée,
licences et provenance conservés. Les références explicites du modèle doivent
être présentes. Le manifeste de ressources fixe les empreintes de tous les
fichiers fournis, y compris les locales chargées dynamiquement.
Une recette compile hors ligne un projet avec un package fictif absent du
catalogue installé. `Archive` ne réalise plus d’entrées/sorties ; il valide une
charge documentaire et son inventaire transporté. À l’exécution, y compris après
réouverture ou récupération, le catalogue de la version installée fournit les
bibliothèques des capacités. Les empreintes contrôlent l’intégrité des octets conservés ;
elles ne certifient pas l’origine ou l’authenticité d’un package.

Conserver les sources et ressources ne revient pas à embarquer les binaires de
Typst, l’ensemble des polices du système ou le système d’exploitation. Les
polices dont un modèle exige l’embarquement devront être déclarées comme
ressources du projet. On garantit l’état documentaire, pas l’identité du PDF
entre versions de typstOgraphe. Une future incompatibilité de format demandera
une importation/conversion explicite, sans construire ce parcours maintenant.
Le [contrat des dépendances Typst](typst-dependencies.md) précise la séparation
entre catalogue de l’application, état documentaire et autonomie des exports.

## Index des versions et références internes de relecture

`versions.json` porte sa version de schéma et deux collections distinctes :

- `snapshots` : les versions créées volontairement et affichées dans **Versions** ;
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
- `reason` : `manual` ;
- `state`, référence CAS immuable de son descripteur.

Le schéma sérialisé regroupe les champs éditables dans l’objet `details` :
`name`, `tags`, `description`, `creator`. Les autres champs restent au premier
niveau. `reason` vaut `manual`. Les références de relecture
portent directement `id`, `review_id`, `created_at`, `state`, `active`.
Les schémas de stockage rejettent les champs inconnus ; les manifestes de
ressources rejettent aussi les chemins logiques répétés.

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
ni relecture J, ni enregistrement implicite. Le candidat reste privé ; seules
les opérations explicitement acceptées sont publiées. Le contenu des deux états
de comparaison reste immuable. La publication passe par les commandes gardées
du moteur ; sauvegarde et export conservent leurs autorisations distinctes.
La conservation durable éventuelle d’une proposition après décision reste à préciser avec F : cette
décision n’ajoute pas une origine visible « agent » ou une sauvegarde automatique.
Le refus ne publie aucune modification. Le comparateur commun d’A.4 fournit
les décisions locales et leur application gardée, pour Versions comme pour MCP.
Les groupes indépendants font l’objet de choix rouge/vert provisoires ; les
opérations liées restent indivisibles. Les états initiaux ne sont jamais réécrits.
La validation humaine « Appliquer N changements » annonce le nombre de groupes
acceptés, avec un résumé des groupes écartés ; « Refuser la proposition » clôture
un lot entièrement refusé sans édition. L’application publie la composition comme une édition
ordinaire, sans écrire le fichier ni ajouter de snapshot. La garde capturée est
revérifiée avant publication ; aucun réancrage implicite n’est autorisé après une
intervention extérieure. La récupération conserve cette édition, puis
l’utilisateur décide de l’enregistrement et des versions.

La modale de consultation/comparaison est commune avec Versions, avec les
onglets Contenu, Aperçu et Comparer. MCP utilise **« État de départ → Proposition »** ;
les verts sont sélectionnés au départ. Les aperçus des deux états et leurs
ressources restent isolés du document courant. Les intitulés ne prétendent pas
que la base est encore le document actuel si celui-ci a évolué. Aucun droit
supplémentaire d’enregistrement n’est accordé aux outils de l’agent.

Si une intervention extérieure change la révision courante, la proposition devient
périmée pour publication ;
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
  L’ordre est « avant à gauche, après à droite » : version historique à gauche,
  document courant figé à droite. Une reprise locale utilise explicitement le
  passage historique de gauche. Pour MCP, base à gauche et proposition à droite,
  l’action applique au contraire la modification proposée à droite. Les rôles
  de source et de cible sont indépendants de l’ordre visuel des colonnes.
- Relectures conserve les outils d’annotations définis dans J. Le snapshot
  technique initial ne détermine pas son interface et n’ajoute pas une action
  de comparaison non validée.

Les propositions MCP rejoignent le parcours de consultation d’états de Versions,
avec la paire et les décisions propres au contexte MCP décrites ci-dessus ;
elles ne sont pas assimilées à une relecture J par ce mécanisme. Échap ferme
d’abord une confirmation, puis la consultation ; le focus revient à l’élément
d’ouverture du contexte concerné. Ce partage porte sur la présentation et le
calcul des différences, pas sur l’identité des opérations autorisées.

## Comparaisons privées et validation selon le contexte

Versions / courant et MCP partagent les décisions locales, avec un vocabulaire
et une validation explicites (Reprendre / Garder, Accepter / Refuser). Pour deux
versions historiques A / B, la composition Bnew reste temporaire et n’est jamais
publiée directement. Une seconde comparaison obligatoire C → Bnew expose toutes
les différences avec le courant figé C. Elle seule peut publier une édition
annulable du même projet ; aucune sauvegarde ni version n’est implicite.
Ces comparaisons ne sont pas des entrées CAS persistées. Les gardes de révision,
les autorisations et la validation du rendu restent applicables à chaque étape.
Voir le [contrat du comparateur](a4-comparison-contract.md).

## Sauvegarde, création, reprise et suppression

L’index des versions reste extérieur à l’arbre documentaire et au checkpoint
Yjs : un état ne contient pas récursivement son propre historique. Sa gestion
appartient au propriétaire Rust, via des opérations gardées accessibles aux
adaptateurs autorisés ; aucun adaptateur ne modifie directement le ZIP.

- **Sauvegarder** prépare l’état confirmé, ses objets et le catalogue inchangé,
  puis remplace atomiquement le fichier. Aucun snapshot implicite.
- **Créer une version** ajoute son entrée et enregistre l’état courant dans la
  même publication du `.typsto`, après choix d’emplacement si nécessaire.
- **Reprendre une version** utilise le comparateur : tout sélectionner à gauche,
  puis appliquer. Le résultat est une édition ordinaire en mémoire, avec une
  étape d’annulation, sans enregistrement ni snapshot automatique. Le moteur
  publie un nouvel état courant sous garde ; il n’injecte pas un ancien binaire
  Yjs et ne ramène pas la révision en arrière. Identité, chemin et versions
  restent inchangés. L’origine inutilisée `before_restore` est retirée du schéma.
- **Supprimer une version** retire seulement une entrée autorisée de `snapshots`,
  sans pouvoir détacher une référence active de `review_references`, puis ne nettoie que les
  objets devenus inaccessibles depuis l’état courant, tous les snapshots retenus
  et les références protégées. Aucun effacement automatique de versions.

La création ou suppression ne doit pas laisser dans le fichier un index
partiellement publié. La reprise ne modifie le fichier qu’à l’enregistrement
explicite suivant. Gardes de révision, contrôle d’une modification
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
et **100 versions manuelles affichées maximum**. Le quota
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
Une reprise par comparaison ne crée pas de version et reste possible au quota
maximal, sous les mêmes gardes documentaires et budgets de volume.
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

La sélection du moteur de diff et le plan d’application locale restent distincts
du format de stockage. Leur moteur commun pour Versions et MCP appartient à A.4 ;
F.3 en adapte le protocole. Voir le [contrat de comparaison](a4-comparison-contract.md).

Bornes techniques du premier schéma : 64 Kio pour `manifest.json` ; 4 Mio pour
`versions.json`, chaque descripteur et chaque manifeste de ressources ; 160
caractères pour un nom de version ; 32 tags de 64 caractères ; 16 Kio pour sa
description et 512 octets pour l’identité libre de son créateur. Le budget de
64 Mio compte le descripteur et les fichiers logiques, même lorsque plusieurs
chemins partagent un objet. Les limites sont contrôlées avant inflation des
objets lorsque les en-têtes suffisent, puis sur les octets effectivement lus.

## Frontière livrée dans A.4.1

`Project` possède l’identité, l’état courant, l’index et une vue des objets.
`new` initialise un checkpoint complet si nécessaire pour un nouveau projet.
`with_current` exige ensuite un état confirmé muni de son checkpoint cohérent :
aucune régénération implicite à partir de caches JSON ou Typst.
`prepare_version`, `prepare_version_details`, `prepare_remove_version` et
`prepare_review_reference` retournent un candidat sans modifier le propriétaire
initial. L’appelant doit enregistrer le candidat avec `save` avant de le publier.
La référence d’une relecture existante n’est pas recapturée.

`save` conserve toutes les racines retenues et ne recopie que leurs objets
accessibles, avec contrôle des empreintes pendant la réécriture. Le fichier
d’origine et l’empreinte de sa garde restent inchangés si une étape échoue.
Les objets sont copiés successivement ; la lecture ne décompresse pas l’ensemble
des anciennes versions. Les vues encore vivantes gardent leur fichier source
ouvert même après son remplacement atomique.

Tauri/CLI/MCP conservent ce propriétaire durant la session. Enregistrer garde
l’identité ; Enregistrer sous ou une copie indépendante d’un projet déjà enregistré
crée un nouvel UUID et conserve les identifiants des versions. Le premier
Enregistrer d’un projet neuf garde son identité initiale. Un brouillon récupéré
n’acquiert pas de verrou sur le chemin d’origine ; une copie issue d’un projet
précédemment enregistré reçoit donc une nouvelle identité lors de sa sauvegarde.

La récupération privée utilise le schéma `typstographe-web-editor-v2` : un champ
`project` obligatoire contient une archive CAS 11 compressée et encodée en base64,
ainsi que `saved_origin`. Ce contexte est mis en cache par propriétaire et modèle,
sans recompression de l’historique à chaque frappe. Le JSON/Yjs confirmé et les
images courantes du checkpoint remplacent son ancien état courant à la reprise ;
le modèle et son identifiant doivent correspondre au contexte épinglé. Le fichier
d’origine n’est pas nécessaire. L’enveloppe JSON est bornée à 1 Gio pour inclure
l’archive encodée et la saisie ; les limites documentaires, Yjs, images et saisies
privées restent appliquées séparément. Le lecteur contrôle aussi la taille sur
disque avant lecture. Aucun formulaire privé ne devient un état documentaire.

Les commandes `versions.list`, `versions.read` et `versions.change` et la
consultation graphique Contenu/Aperçu sont raccordées. Le paramètre `history` de
`versions.change` est une empreinte de l’index fournie par `versions.list` : une
modification concurrente de nom, de tags ou de liste est refusée, même si la
révision documentaire n’a pas changé. La garde du document reste distincte.
La comparaison commune publie les compositions locales ou complètes comme une
édition ordinaire, pour Versions et les propositions MCP privées. Le cycle de
vie des relectures reste dans J. Le [bilan A.4.3](a4-versions-interface.md) décrit
la consultation ; le [bilan du comparateur](a4-comparison-results.md) précise
les choix provisoires, la publication et les garanties de conservation.
