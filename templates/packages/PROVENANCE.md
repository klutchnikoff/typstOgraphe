# Dépendances Typst embarquées

Copies des archives publiées dans le registre officiel Typst, versions figées :

| Paquet | Archive | SHA-256 |
| --- | --- | --- |
| Datify 1.3.0 | https://packages.typst.org/preview/datify-1.3.0.tar.gz | `43f313ea6b463469080cfa61e404d51a2d70d7aead6e5365f9b4dc7163519ac6` |
| Datify-core 2.1.0 | https://packages.typst.org/preview/datify-core-2.1.0.tar.gz | `5a3621f82e88098e8924f811f86b2733e8eeed3a6f08ac8e42507888ef0c29e6` |

Datify 1.3.0 importe Datify-core **2.1.0** dans son archive publiée.
La seule modification aux sources amont est l’import de `src/formats.typ` :
`@preview/datify-core:2.1.0` est remplacé par
`../../datify-core-2.1.0/src/main.typ`.

Les sources, toutes les locales et les licences MIT des deux archives sont
conservées. Les données CLDR de Datify-core sont également accompagnées de
la licence Unicode : https://www.unicode.org/license.txt (copie du 21 octobre 2026).

L’inventaire de référence est `templates/dependencies.json` dans le dépôt.
Il liste les capacités, leurs interfaces et les paquets avec leurs versions,
licences, archives, empreintes et dépendances. Il est copié sous le nom
`dependencies.json` à la racine des documents et des exports, avec les seules
entrées correspondant aux ressources embarquées.

Le modèle importe `capabilities/dates.typ` et appelle `format-date`.
Seul cet adaptateur importe `packages/datify-1.3.0/src/main.typ` ; l’API de Datify
et ses versions ne font pas partie du contrat des modèles. Le rendu utilise
uniquement ces fichiers locaux. Le même ensemble complet est embarqué dans le binaire,
copié dans les documents `.typsto`, puis dans leurs exports Typst. Il reste
possible de changer la langue d’un document hors ligne. Le fichier documentaire
conserve ses propres exemplaires ; il ne dépend pas du cache de paquets Typst.

Pour une mise à jour : récupérer les archives officielles, vérifier les empreintes
et la dépendance réellement importée, appliquer le changement d’import local,
actualiser l’inventaire et adapter `capabilities/dates.typ` en conservant sa
signature et son comportement public. Les modèles gardent le même appel.
Exécuter les tests du contrat, d’archivage, d’export et de compilation hors ligne.
Un remplacement par un autre paquet suit exactement le même principe.
