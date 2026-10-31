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

## Index des versions et protection des références

`versions.json` porte sa version de schéma, une collection de snapshots et les
références typées nécessaires pour savoir lesquels sont protégés.
Chaque entrée de snapshot contient :

- un `id` UUID stable, indépendant de son nom et du descripteur CAS ;
- `created_at`, instant UTC automatique, affiché selon la locale de l’interface ;
- `name`, nom libre modifiable, daté par défaut ;
- `tags`, liste facultative, et `description`, texte facultatif ;
- `creator`, identité disponible du créateur, sans attribution de toutes les modifications ;
- `reason` : `manual`, `review_start` ou `before_restore` ;
- `state`, référence CAS immuable de son descripteur.

Le classement chronologique utilise la date, avec l’identifiant comme départage
stable. Ni l’ordre de l’index, ni le nom, ni les tags ne définissent le contenu
d’une version. Renommer ou étiqueter ne crée pas un nouveau snapshot et ne
change ni sa date, ni son créateur, ni sa référence d’état.

Les références de relecture lient un identifiant de relecture à un identifiant
de snapshot, avec une indication d’activité. Elles constituent la source de la
protection contre la suppression ; une simple étiquette « relecture » ne suffit
pas. Supprimer une version référencée par une relecture active est refusé.
La collection reste vide tant qu’aucune relecture n’existe. Le stockage
des suggestions/commentaires et la politique après clôture restent à préciser
dans J ; ce format ne définit pas déjà le contenu de ces annotations.

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
- **Supprimer** retire seulement une entrée autorisée, puis ne nettoie que les
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
et **100 snapshots maximum**, toutes origines confondues (`manual`, `review_start`,
`before_restore`). L’état courant ne compte pas comme un snapshot supplémentaire.
Un plafond est un refus explicite sans perte, jamais une autorisation de supprimer
des versions. Ces budgets doivent être vérifiés lors de
l’intégration ; ils ne sont pas une garantie de latence sur ces volumes.

À 100 snapshots, l’édition et la sauvegarde ordinaire restent possibles dans les
limites de volume. Pour créer une nouvelle version, l’utilisateur est invité à
supprimer explicitement une version non protégée. Aucun roulement automatique.
Si une restauration exige une sauvegarde `before_restore` et qu’aucune place n’est
disponible, la restauration est refusée avant toute modification ; même règle
pour le démarrage d’une nouvelle relecture exigeant sa référence initiale.

La maquette de Versions doit maintenant concrétiser :

1. Le conteneur ZIP entièrement CAS, partagé avec l’état courant ; le répertoire
   directement compilable est produit par l’export, sans copie parallèle à la racine.
2. Le descripteur commun complet et vérifié, l’index séparé, le contenu des versions
   immuable et leurs noms/tags modifiables.
3. La lecture sélective, le traitement explicite d’une ancienne version abîmée
   et les limites retenues, avant les recettes de limites et de conservation.

La sélection du moteur de diff et la validation partielle des propositions MCP
restent séparées de ce format ; F.3 ne bloque pas A.4.
