# Article — contrat retenu le 27 octobre 2026

Le premier modèle est « Article de travail ». Le type reste intégré en Rust ;
sa présentation et la bibliothèque éditoriale commune sont en pur Typst.
Les citations bibliographiques et leur bibliothèque constituent le lot suivant.

## Fonction des modèles

```typ
#let article(localizations: (), authors: (), affiliations: (), options: (
  paper: "a4", landscape: false, language: "fr", bullet-marker: auto,
), body) = { /* présentation du modèle */ }
```

Tous les modèles du type acceptent les mêmes données. Leur changement ne modifie
ni le contenu, ni l’ordre des auteurs, ni les identifiants.

## Données

Toutes les clés sont présentes. Les absences sont représentées par `""`, `[]`,
`()`, ou `false` selon le genre. Les identifiants restent stables lors de l’édition
et du déplacement ; ils ne dépendent pas du libellé.

- `authors` : liste ordonnée de dictionnaires `id`, `name`, `affiliation-ids`
  (liste ordonnée d’identifiants), `email`, `orcid`, `corresponding` (booléen).
  Le nom de publication est libre. Plusieurs correspondants sont possibles ;
  un courriel ne suffit pas à attribuer ce statut. Aucune affiliation obligatoire.
- `affiliations` : liste de dictionnaires `id`, `text`. Le texte est une signature
  officielle libre, préservée exactement, sans décomposition en établissement,
  laboratoire ou adresse. Plusieurs auteurs peuvent référencer la même entrée.
- `localizations` : liste ordonnée de dictionnaires `id`, `language`, `title`,
  `short-title`, `subtitle`, `abstract`, `keywords` (liste de chaînes).
  Les titres et résumés sont du contenu riche sûr, pas du Typst arbitraire.
  Une entrée au maximum par langue. La langue principale est `options.language`.
  Le titre court est facultatif ; son absence utilise le titre complet pour les
  modèles qui composent un titre courant. Le titre principal et les métadonnées
  du document conservent le titre complet.

L’interface présente le titre principal d’abord, les auteurs et affiliations,
puis les ensembles linguistiques. Changer la langue principale conserve les
ensembles existants et réutilise ou crée celui de cette langue, sans traduire.
Un titre traduit peut exister sans résumé et inversement. Le résumé autorise
paragraphes, gras, italique et liens ; les titres restent du contenu en ligne.

Les brouillons incomplets sont prévisualisables et enregistrables. L’export
exige un titre principal, des noms pour les auteurs ajoutés et des textes pour
les affiliations ajoutées, ainsi que des identifiants uniques et associations
valides. La suppression d’une affiliation utilisée nécessite de traiter
explicitement ses associations, jamais de laisser des références pendantes.

## Bibliothèque commune

Signatures cibles :

```text
prepare-authorship(authors, affiliations) -> dictionary
show-authorship(prepared, render-author: auto, render-affiliation: auto) -> content
show-abstracts(localizations, primary-language: "fr", render-entry: auto) -> content
```

`prepare-authorship` résout les associations et numérote les affiliations utilisées
selon leur première apparition, sans modifier les identifiants ni les textes.
`show-authorship` fournit une composition sobre personnalisable ; les modèles
peuvent utiliser directement les données préparées pour une autre disposition.
`show-abstracts` compose les résumés, mots-clés et titres secondaires éventuels.
Le titre principal est placé par le modèle. Les fonctions de rendu reçoivent les
enregistrements préparés, avec l’indication de langue principale pour les résumés.

Les fichiers `capabilities/authorship.typ` et `capabilities/abstracts.typ` sont
communs aux types et modèles qui en ont besoin. Pubmatter reste une référence de
conception, sans dépendance imposée. Les emprunts éventuels gardent leur provenance
et leur licence. Les fonctions utilisées sont inventoriées et embarquées dans
les projets et exports, sans téléchargement à la compilation.

## Interfaces et recette

Même schéma documentaire pour l’interface, la CLI et MCP. L’état partagé conserve
les identifiants, les champs indépendants des enregistrements et les zones riches ;
une collection ne doit pas devenir une chaîne JSON opaque dans l’éditeur.

Vérifier : trois auteurs, affiliations multiples et partagées, auteur indépendant,
correspondant explicite, noms identiques, signature officielle exacte, résumés
français/anglais avec styles et mots-clés distincts, titre court, réorganisation,
annulation, éditions concurrentes, sauvegarde/réouverture et export hors ligne.

## Implémentation et utilisation

À la création d’un document, choisir **Article**, puis **Article de travail**.
L’onglet Général conserve modèle, papier, orientation et langue principale.
L’onglet En-tête présente les titres de cette langue, puis les auteurs,
affiliations et ensembles linguistiques. Ajouter une affiliation une seule fois,
puis la cocher pour chacun de ses auteurs. Les flèches changent l’ordre sans
changer les identifiants ; les corbeilles demandent confirmation. Une affiliation
utilisée ne peut être supprimée qu’avec le retrait explicite de ses associations.
Le résumé possède une zone riche, avec gras, italique et liens via le bandeau
commun. Les mots-clés sont saisis un par ligne, y compris les expressions longues.

Les collections sont décrites en Rust, traduites par Fluent et présentées par des
composants génériques. Les [profils de zones riches](rich-zones.md) centralisent
la création, le schéma et les comportements de saisie des titres et résumés.
Dans Yjs, chaque enregistrement possède ses champs propres :
`Y.Text` pour les textes libres, `Y.XmlFragment` pour les zones riches, valeurs
atomiques pour les booléens, langues, mots-clés et listes d’associations. L’ordre
est séparé des identités. Les éditions de texte et de champs distincts fusionnent ;
les changements concurrents d’une même valeur atomique suivent la résolution de
Yjs. Les incohérences structurelles éventuelles (par exemple une langue ajoutée
deux fois hors ligne) sont refusées par la validation avant enregistrement/export.
L’historique commun couvre aussi l’en-tête ; fermer une section ne le réinitialise pas.

La CLI et le MCP autonome découvrent ces mêmes schémas avec `document_types.describe`
(MCP) et la requête `document_types.list` (CLI). `metadata.set` reçoit la collection
complète sous forme de tableau JSON avec une garde de révision. Mettre à jour les
affiliations avant d’y associer les auteurs ; enlever les associations avant de
supprimer une affiliation. Les champs riches utilisent l’arbre documentaire
restreint décrit par le schéma, jamais une chaîne de code Typst. Le MCP rattaché
à l’application peut lire l’en-tête ; ses propositions d’édition restent limitées
au corps, comme pour les autres types, en attendant l’unification suivante.

Les fichiers communs sont des implémentations originales, sans code repris ni
paquet Pubmatter embarqué. Ils sont inventoriés dans
`crates/core/templates/dependencies.json`. La recette teste le rendu Typst réel,
les identités et associations, les deux langues, l’annulation, les éditions Yjs,
les parcours clavier Chromium/WebKit, la réouverture et un export recompilé avec
ses seules dépendances. Aucune capture d’écran n’est produite.

La bibliographie et les badges de citations restent à préciser ensemble. Ce lot
ne fournit pas encore de correcteur orthographique propre aux zones riches de
l’en-tête ni de synchronisation vers ces zones depuis l’aperçu. Le correcteur du
corps et sa synchronisation restent disponibles.
