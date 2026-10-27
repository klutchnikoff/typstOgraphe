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
