# Dépendances Typst et conservation documentaire

Décision confirmée le 31 octobre 2026. Ce contrat précise les règles de A.4 et
remplace le traitement des paquets comme éléments restaurables du document.

## Catalogue de l’application

Chaque version de typstOgraphe fournit un catalogue figé de bibliothèques de
capacités et de paquets Typst, avec leurs versions, licences et provenance.
Le catalogue et les wrappers sont maintenus par le projet, pas par l’utilisateur.
Le modèle appelle les signatures publiques de nos capacités ; la mise en œuvre
et ses dépendances peuvent évoluer derrière ces signatures.

Une mise à jour doit conserver le fonctionnement des capacités existantes.
Cette garantie repose sur nos adaptations et nos tests de contrats et de modèles,
pas sur l’hypothèse qu’une version plus récente d’un paquet serait nécessairement
compatible. Des fonctionnalités supplémentaires peuvent demander une version
plus récente de typstOgraphe.

Le rendu, les aperçus de versions, la validation de modèle, la récupération et les
exports réalisés par l’application utilisent les bibliothèques de sa version
installée. La résolution est la même pour Tauri, CLI et MCP. Une dépendance absente
de son catalogue n’est pas ajoutée par une décision documentaire ou téléchargée
implicitement. Une capacité inconnue ne doit pas être interprétée silencieusement.

## État documentaire et ressources transportées

Les snapshots garantissent l’état documentaire : contenu structuré, métadonnées,
réglages, source du modèle, images, signature et références bibliographiques.
Ils ne figent pas la version de l’application, du moteur Typst, des polices système
ou de ses paquets. **L’identité du PDF entre versions de typstOgraphe n’est pas
garantie.** Le modèle reste une donnée conservée ; ses bibliothèques de capacités
appartiennent à l’application.

Le `.typsto` et l’export Typst conservent les copies nécessaires à leur autonomie
hors ligne. Le stockage CAS contrôle l’intégrité de ces fichiers et de leur
inventaire ; il peut conserver les copies liées aux états historiques. Ce rôle de
transport ne leur donne aucune autorité sur les bibliothèques exécutées dans
typstOgraphe. Les nouvelles sauvegardes et exports de l’état courant incorporent
les dépendances sélectionnées dans le catalogue de l’application.

Comparer, accepter une proposition MCP, reprendre une version ou Annuler/Rétablir
ne change jamais les versions techniques des paquets. Une différence de copies
transportées ne crée ni groupe applicable ni modification documentaire et ne lie
pas des changements de texte autrement indépendants.

## Formats incompatibles — parcours futur

Une version future incompatible avec un ancien format `.typsto` proposera
clairement une importation/conversion, en distinguant ce parcours d’une ouverture
ordinaire et sans écraser silencieusement le fichier d’origine. Ce chantier
n’est pas implémenté à ce stade ; aucune couche de rétrocompatibilité n’est
requise pour les fichiers du prototype. Le contrôle actuel des formats et schémas
inconnus reste en place.
