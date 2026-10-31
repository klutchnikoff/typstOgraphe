# A.4.1 — Propriétaire CAS dans le cœur

État au 31 octobre 2026, sur `feat/a4-snapshot-storage-evaluation`.
Le [contrat du format](a4-project-format.md) fait référence pour les données ;
ce bilan décrit le périmètre implémenté et sa vérification.

## Ce qui est disponible

`crates/core/src/content/project/` fournit le propriétaire `Project` du format 11 :

- identité stable, copie indépendante avec nouvel UUID et historique conservé ;
- état courant complet, checkpoint Yjs obligatoire après l’initialisation,
  projections JSON, métadonnées, modèle épinglé et sources Typst contrôlés ;
- index distinct des versions visibles et des références internes de relecture ;
- préparation de création, changement de nom/tags/description/créateur et
  suppression d’une version, sans altérer son contenu ni le propriétaire initial ;
- capture unique de la base d’une relecture, reprise sans recapture ;
- objets SHA-256 partagés, manifeste de ressources dédupliqué et lecture sélective ;
- remplacement ZIP atomique sous verrou et contrôle de modification externe,
  vérification des objets copiés et nettoyage des seuls objets inaccessibles.

L’API retourne un **candidat préparé**. L’adaptateur doit appeler `save`, puis
publier ce candidat seulement si l’écriture réussit. Cette API de stockage ne
remplace pas les gardes de révision ni l’autorité documentaire du moteur.
Les objets ne sont jamais extraits vers les chemins indiqués par l’archive.

`packages::validate_embedded` vérifie les dépendances conservées contre leur
propre inventaire et les chemins autorisés, indépendamment du catalogue installé.
Les octets des packages, wrappers, licences et provenances restent épinglés.
Il s’agit de l’intégrité des copies transportées, pas de la sélection des
bibliothèques à exécuter dans l’application : cette dernière suit désormais le
[catalogue de la version installée](typst-dependencies.md). Le test de dépendance
fictive ci-dessous vérifie l’autonomie des octets archivés pour un usage Typst
extérieur, pas leur activation dans typstOgraphe.
Les empreintes garantissent l’intégrité des contenus attendus ; aucune signature
cryptographique d’éditeur de package n’est revendiquée.

## Garanties vérifiées

Les tests de ce lot couvrent notamment :

- sauvegarde/réouverture et compilation Typst hors ligne des types pourvus d’un
  modèle ; conservation de l’identité, des métadonnées, du modèle et du checkpoint ;
- deux versions distinctes du même état partageant exactement les mêmes objets ;
- remplacement puis retrait d’images et changement de modèle, avec anciennes
  versions encore compilables ; une base de relecture protège ses ressources
  même après suppression de la version visible correspondante ;
- accès d’une vue déjà ouverte à son fichier source après remplacement atomique ;
- création de 100 versions, refus explicite de la suivante, sauvegarde ordinaire
  et référence interne de relecture toujours possibles dans les budgets globaux ;
- dépendance fictive d’une ancienne version, absente du catalogue installé,
  sauvegardée, relue et compilée avec ses propres fichiers ;
- article avec citation et fichier bibliographique original, et refus des caches
  JSON/Yjs/Typst contradictoires même lorsque leurs empreintes sont valides ;
- ancienne version corrompue ou descripteur absent : document courant consultable,
  réécriture refusée sans modifier le fichier ni l’empreinte de sa garde ;
- échec en cours de réécriture d’une nouvelle version : aucun index partiel publié ;
- refus de doublons ZIP, chemins inattendus, liens symboliques, archives tronquées,
  références invalides, index incohérents, schémas et encodages inconnus ;
- contrôle des budgets annoncés avant inflation lorsque possible : archive,
  nombre d’objets, état logique, checkpoint, image, manifeste et index ;
- garde de fichier compatible avec une archive dépassant l’ancien budget d’un
  état courant, et refus rapide d’un fichier dépassant 512 Mio.

Les recettes de volume utilisent des fichiers creux et des en-têtes falsifiés
pour vérifier les refus sans écrire ou gonfler des centaines de Mio à chaque CI.
Le contrôle logique compte chaque chemin, même si plusieurs chemins désignent le
même objet CAS. Il ne faut pas confondre ces refus avec un benchmark aux plafonds.

Commandes principales :

```sh
cargo test --locked --offline -p typstographe-core
cargo test --locked --offline -p typstographe-i18n
cargo clippy --locked --offline -p typstographe-core -p typstographe-i18n --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/check_file_length.py
```

Résultats : **188 tests du cœur et son doctest passent**, ainsi que les **10 tests
i18n**, Clippy sans avertissement, le formatage et le contrôle des longueurs des
modules. Les **3 tests ciblés de reconnexion MCP de la CLI** passent hors du bac
à sable, qui interdit leurs connexions locales.
Les nouvelles erreurs de stockage ont des messages Fluent français et anglais.
L’évaluation de performance Git/CAS reste dans son
[rapport distinct](a4-snapshot-storage-evaluation.md) ; ses mesures ne sont pas
présentées comme celles de ce nouveau code de production.

La tentative de suite CLI/MCP complète a été interrompue à l’édition des liens
par saturation du disque ; elle n’est pas déclarée validée pour ce lot. Les caches
incrémentaux reconstruits pendant ces contrôles ont été nettoyés, puis la recette
de reconnexion a été relancée seule, en séquentiel et sans nouveau cache
incrémental. Aucun fichier source, rapport ou résultat d’évaluation n’a été supprimé.

## Raccordement A.4.2

Le propriétaire CAS est désormais conservé par Tauri/CLI/MCP et la récupération.
Le format 11 remplace le lecteur/écrivain de format 10, retiré du projet.
L’exemple Article a été régénéré explicitement depuis son contenu Git.
Le [bilan A.4.2](a4-storage-adapters.md) décrit identité, copies, dépendances
épinglées et récupération autonome.

L’interface Versions, les commandes publiques, la restauration dans une session
active et la comparaison commune suivent ce raccordement. Le cycle de vie des
relectures et les paires privées base/candidat MCP ne sont pas anticipés par une
fausse fonctionnalité utilisateur. A.4 reste en cours.
