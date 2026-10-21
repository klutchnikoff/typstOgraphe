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

Le modèle importe `packages/datify-1.3.0/src/main.typ`. Le rendu utilise uniquement
ces fichiers locaux. Le même ensemble complet est embarqué dans le binaire,
copié dans les documents `.typsto`, puis dans leurs exports Typst. Il reste
possible de changer la langue d’un document hors ligne. Le fichier documentaire
conserve ses propres exemplaires ; il ne dépend pas du cache de paquets Typst.

Pour une mise à jour : récupérer les archives officielles, vérifier les empreintes
et la dépendance réellement importée, appliquer le seul changement d’import,
puis exécuter les tests d’archivage, d’export et de compilation hors ligne.
