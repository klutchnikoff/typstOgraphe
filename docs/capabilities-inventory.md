# Inventaire des capacités documentaires

Établi le **26 octobre 2026**, à partir de la liste comparative de Docs (DINUM)
fournie par l'utilisateur, des besoins de typstOgraphe et d'un premier examen de
Typst et de Typst Universe.

Cet inventaire est une référence pour le [chantier C de la feuille de route](../ROADMAP.md#c--compléter-les-capacités-à-partir-des-besoins-des-types).
Il ne constitue ni un engagement à tout développer, ni le catalogue des commandes
déjà implémentées, décrit dans [l'inventaire des commandes](commands-inventory.md).
La liste de Docs est un point de comparaison fourni par l'utilisateur, pas un
audit exhaustif de ce produit.

**Les tableaux simples et les notes de bas de page ont une base fonctionnelle
dans l’interface Tauri.** Leur consolidation reste prioritaire sur de nouvelles
capacités complexes ; le périmètre des autres ajouts dépendra des types retenus.
Ce relevé ne lance aucune implémentation. L’assistance ACP et la collaboration
ont leurs propres chantiers dans la [roadmap](../ROADMAP.md).

## 1. Périmètre

Le catalogue exécutable de [contenu structuré](../crates/core/src/content/catalogue.rs)
décrit les capacités disponibles. Cet inventaire conserve les besoins à concevoir
ou à compléter ; il ne répète pas la liste des fonctionnalités terminées.
Les métadonnées, dates et signatures relèvent des paramètres du document et du modèle.
Une citation en bloc et une citation bibliographique sont deux capacités distinctes.

## 2. Consolidation prioritaire — P1

| Capacité | Compléments à cadrer après vérification de l’existant | Appui Typst |
| --- | --- | --- |
| Tableaux simples | Légende, alignements, contrat des contenus de cellules et recette complète clavier/fichier/MCP. Ne pas refaire la grille et les opérations déjà présentes ; différer les fusions et les calculs. | `table`, avec `figure` pour une légende et une éventuelle numérotation. |
| Notes de bas de page | Terminer la recette de navigation, des frontières, des modifications distantes et des exports ; corriger les manques réellement observés. | `footnote`. |

L’ajout d’une capacité nouvelle doit répondre au besoin d’un type retenu. Les
deux recettes ci-dessus accompagnent la sortie du prototype, sans remettre à zéro
leur implémentation.

## 3. Inventaire complémentaire

Les niveaux ci-dessous reprennent les priorités existantes ou donnent une
orientation indicative pour les nouveaux éléments. Leur périmètre, leur
activation par type et leur ordre précis restent à valider avant de coder.

| Capacité | Orientation | Appui ou piste à examiner |
| --- | --- | --- |
| Listes de tâches | P2 à instruire. État coché/non coché par élément et rendu imprimable. | Composition Typst à définir ; ne pas réduire l'état à un caractère décoratif. |
| Listes imbriquées | P2, déjà repérées au chantier B. Préserver une liste entière comme widget, sans multiplier les widgets à chaque profondeur. | Listes Typst ; règles d'édition et de navigation à concevoir. |
| Barré et souligné | P2 à instruire. Enrichissements accessibles sans nécessairement ajouter des boutons permanents. | `strike`, `underline`. |
| Exposants et indices | P2 à instruire, y compris hors des équations : m², CO₂… | `super`, `sub`. |
| Séparateur thématique | P2 à instruire. Exprimer une séparation de contenu ; le modèle choisit sa représentation. | `divider`, plutôt qu'un filet dessiné directement par l'utilisateur. |
| Alertes / encadrés sémantiques | P2 à instruire. Une capacité commune avec les variantes `note`, `tip`, `warning`, `caution`, `important`, un titre éventuel et un corps. | Une capacité telle que `callout`, à définir ; primitives de blocs Typst, éventuellement un paquet d'encadrés. |
| Citations bibliographiques et bibliographie | P2 déjà prévu. Choisir une référence et, si nécessaire, une page ou un complément ; conserver les données bibliographiques. | `cite`, `bibliography` ; fichiers BibLaTeX ou Hayagriva, styles CSL. |
| Références croisées | P2 déjà prévu. Choisir une cible : titre, figure, tableau, équation ou autre élément identifiable. | Labels et `ref` ; numérotation et libellés produits sans saisie manuelle. |
| Table des matières et listes des figures/tableaux | P2 déjà prévu. Contenu calculé à partir des éléments du document. | `outline`. Leur présence et leur emplacement peuvent relever du modèle. |
| Annexes | P2 à instruire avec les documents longs. Titres, numérotation et renvois cohérents. | Structure de titres et règles du type/modèle à définir. |
| Saut de page explicite | P2/P3 déjà repéré. Possibilité ponctuelle à exposer avec parcimonie. | `pagebreak`. Les débuts de chapitres ou d'annexes sur une nouvelle page relèvent d'abord du modèle. |
| Multicolonne | P2/P3 à instruire. Distinguer la composition du document entier d'un passage particulier en colonnes. | Réglage de page ou `columns`, selon le besoin ; interactions avec les notes et la pagination à vérifier. |
| Code en ligne et blocs de code | P3. Langage facultatif pour un bloc ; présentation littérale et coloration éventuelle. | `raw` natif ; Codly si des fonctions supplémentaires sont utiles. |
| Équations en ligne et en bloc | P3. Première piste : champ de saisie mathématique Typst avec aperçu. Évaluer séparément une saisie visuelle. | Mathématiques Typst ; MathLive avec MiTeX comme autre piste, sans choix d'intégration arrêté. |
| Diagrammes | P3. Définir d'abord les familles utiles : organigrammes, processus, arbres, schémas… | Fletcher et CeTZ à examiner ; l'interface de création reste à concevoir. |
| Graphiques de données | P3, distinct des diagrammes. Courbes, barres, nuages de points ; données saisies ou importées, par exemple en CSV. | Lilaq à examiner ; décider de la conservation des données et de leur édition. |
| Glossaire, acronymes et index | P3, selon les besoins des types de documents longs. | Fonctions natives et paquets Universe à inventorier avant tout choix. |

Les légendes d'images existent déjà. L'extension porte notamment sur les légendes
de tableaux et de graphiques, la numérotation et les renvois ; elle doit réutiliser
les interactions communes plutôt que recréer plusieurs mécanismes indépendants.

## 4. Typst et Universe : premier repérage

Il s'agit de candidats et de sources de référence, **pas de nouvelles dépendances
retenues**. Les versions, licences, dépendances transitives, entretien et limites
seront examinés lors du choix de chaque lot.

| Besoin | Référence | Intérêt pour l'étude |
| --- | --- | --- |
| Tableaux | [Typst — table](https://typst.app/docs/reference/model/table/) | Structure tabulaire native, cellules et en-têtes. |
| Notes | [Typst — footnote](https://typst.app/docs/reference/model/footnote/) | Appels de notes et composition native. |
| Bibliographie | [Typst — bibliography](https://typst.app/docs/reference/model/bibliography/) | Formats de données et styles bibliographiques natifs. |
| Renvois et sommaires | [Typst — ref](https://typst.app/docs/reference/model/ref/), [outline](https://typst.app/docs/reference/model/outline/) | Cibles, références et listes calculées. |
| Séparateur | [Typst — divider](https://typst.app/docs/reference/model/divider/) | Séparation thématique personnalisable par le modèle ; présente dans Typst 0.15.1 utilisé par le projet. |
| Composition | [Typst — columns](https://typst.app/docs/reference/layout/columns/), [pagebreak](https://typst.app/docs/reference/layout/pagebreak/) | Colonnes locales ou à l'échelle de la page, sauts explicites et contraintes de placement. |
| Code | [Typst — raw](https://typst.app/docs/reference/text/raw/), [Codly](https://typst.app/universe/package/codly/) | Le natif couvre le texte littéral et la coloration ; Codly ajoute notamment numéros de lignes, références et annotations. |
| Encadrés | [Showybox](https://typst.app/universe/package/showybox/) | Candidat pour la présentation de boîtes ; ne définit pas à lui seul notre contrat sémantique d'alerte. |
| Mathématiques | [Typst — math](https://typst.app/docs/reference/math/), [MathLive](https://mathlive.io/mathfield/), [MiTeX](https://typst.app/universe/package/mitex/) | Saisie Typst native, ou éditeur web produisant du LaTeX rendu via MiTeX. Comparer l’ergonomie et les dépendances sur l’interface Tauri avant de choisir. |
| Diagrammes | [Fletcher](https://typst.app/universe/package/fletcher/), [CeTZ](https://typst.app/universe/package/cetz/) | Diagrammes avec flèches pour Fletcher ; dessin plus général pour CeTZ, sur lequel Fletcher s'appuie. |
| Graphiques de données | [Lilaq](https://typst.app/universe/package/lilaq/) | Bibliothèque de tracés : courbes, barres, nuages de points, etc. |

Ce repérage n'est pas exhaustif. L'existence d'un paquet de composition ne résout
pas nécessairement la saisie visuelle, la navigation au clavier ou l'échange avec
les agents. Ces points font partie du coût réel de la capacité.

## 5. Principes et décisions avant chaque lot

1. **Définir le besoin et le type concerné.** Une capacité partagée est implémentée
   une fois et activée dans les types qui en ont besoin ; tous les types ne doivent
   pas proposer toutes les capacités.
2. **Préserver la séparation contenu/présentation.** L'utilisateur exprime un
   contenu, une intention et les paramètres utiles. Le modèle définit notamment
   couleurs, bordures, typographie, numérotation et valeurs par défaut. Les réglages
   généraux de composition appartiennent d'abord au modèle.
3. **Valider l'édition.** Choisir les champs directs, annotations facultatives,
   paramètres de bandeau et règles de clavier. Réutiliser les interactions de
   widgets et de bandeaux, en tenant compte des capacités en ligne. Pour les listes
   imbriquées, conserver un seul widget pour la liste entière.
4. **Examiner les solutions existantes.** Utiliser les fonctions natives quand
   elles suffisent. Un paquet externe ou une logique commune spécifique à
   l'application passe par l'adaptateur Typst stable prévu au chantier C ; ne pas
   envelopper systématiquement les primitives natives.
5. **Consigner le contrat.** Les identifiants publics restent anglais ou latins,
   les libellés sont traduits. Les noms proposés ici ne figent aucune signature.
   Le contrat documentaire doit servir l'interface graphique, la CLI et MCP.
6. **Garantir l'autonomie.** Contenu structuré et Typst généré cohérents ; données, ressources et
   dépendances nécessaires conservées dans le `.typsto` et les exports appropriés,
   avec compilation hors ligne. L'inventaire technique des paquets retenus reste
   `crates/core/templates/dependencies.json`.
7. **Vérifier le parcours complet.** Insertion, sélection, modification, navigation,
   annulation/rétablissement, recherche selon le contenu, sauvegarde/réouverture,
   synchronisation avec l'aperçu et export conforme au modèle. Définir explicitement
   les limites initiales avant d'étendre les structures complexes.
