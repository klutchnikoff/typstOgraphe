// Visual recipe: compile from the repository root, using only the bundled fonts.
// typst compile --root . --ignore-system-fonts fixtures/article-layout.typ output/pdf/article-working.pdf
#import "../crates/core/templates/article-working.typ": article

#let empty = (title: [], short-title: [], subtitle: [], abstract: [], keywords: ())
#show: article.with(
  options: (paper: sys.inputs.at("paper", default: "a4"),
    landscape: sys.inputs.at("landscape", default: "false") == "true",
    language: "fr", bullet-marker: auto),
  localizations: (
    (..empty, id: "fr", language: "fr",
      title: [Écrire et transmettre les savoirs],
      short-title: [Écrire et transmettre],
      subtitle: [Pratiques éditoriales et travail collectif dans la recherche],
      abstract: [
        L’écriture scientifique ne se réduit pas à la restitution de résultats.
        Elle organise un travail de sélection, de mise en relation et de discussion
        des connaissances. Cet article de démonstration propose une réflexion sur
        les conditions matérielles de cette activité et sur la place des outils
        de composition dans le travail collectif. Il distingue les choix qui
        relèvent des auteurs de ceux qui appartiennent à la présentation éditoriale,
        afin de faciliter la lecture et la circulation des textes.
      ],
      keywords: ("écriture scientifique", "travail collectif", "édition", "sciences humaines et sociales")),
    (..empty, id: "en", language: "en",
      title: [Writing and sharing knowledge],
      subtitle: [Editorial practices and collective work in research],
      abstract: [
        Scholarly writing involves more than reporting results. It brings together
        the selection, organisation and discussion of knowledge. This demonstration
        article considers the material conditions of this activity and the role of
        document tools in collaborative work. It distinguishes authors’ decisions
        from editorial presentation choices, with a view to making texts easier
        to read, review and share.
      ],
      keywords: ("scholarly writing", "collaboration", "publishing")),
  ),
  authors: (
    (id: "camille", name: "Camille Martin", affiliation-ids: ("irmar",),
      email: "camille.martin@example.org", orcid: "", corresponding: true),
    (id: "alex", name: "Alex Dupont", affiliation-ids: ("irmar", "research"),
      email: "", orcid: "", corresponding: false),
  ),
  affiliations: (
    (id: "irmar", text: "Univ Rennes, CNRS, IRMAR (Institut de Recherche Mathématique de Rennes) - UMR 6625, F-35000 Rennes, France"),
    (id: "research", text: "Centre d’étude des pratiques de recherche, France"),
  ),
)

= Introduction

Écrire un article suppose de donner une forme lisible à une démarche qui ne
s’est pas nécessairement déroulée dans le même ordre. Les notes de terrain,
les échanges entre collègues et les premières analyses constituent un ensemble
hétérogène. Le texte final établit entre ces matériaux des relations que le
lecteur doit pouvoir comprendre sans avoir participé à leur élaboration.

Ce document sert d’exemple de composition : les personnes et la seconde
affiliation sont fictives. Il permet d’examiner les titres, les résumés en deux
langues et la continuité du texte sur plusieurs pages, sans prétendre présenter
les résultats d’une enquête. La signature institutionnelle illustre, quant à
elle, la nécessité de conserver une affiliation officielle en texte libre.

== Distinguer contenu et présentation

Le contenu porte le raisonnement ; la présentation en facilite le parcours.
Un changement de modèle peut modifier la taille des titres ou la disposition
des coordonnées sans transformer les auteurs, leurs affiliations ou les
paragraphes. Cette séparation devient particulièrement utile lorsqu’un même
texte circule entre plusieurs espaces de publication.#footnote[
  La présente note est un exemple de note de bas de page. Elle appartient au
  contenu de l’article et suit la numérotation du document.
]

Il ne s’agit pas de rendre la forme indifférente. Au contraire, une hiérarchie
régulière, des marges suffisantes et des espacements mesurés soutiennent la
compréhension. La mise en page reste discrète lorsqu’elle rend les articulations
du texte évidentes sans attirer l’attention sur ses propres procédés.

= Des pratiques individuelles au travail collectif

La rédaction engage plusieurs temporalités. Certaines décisions interviennent
dans l’instant, au moment de choisir un mot ou de reformuler une phrase. D’autres
s’inscrivent dans une durée plus longue : déplacer une section, rapprocher deux
observations ou reprendre une hypothèse après discussion. L’outil doit permettre
ces opérations sans imposer à chaque étape une décision de mise en page.

#quote(block: true, attribution: [Principe de composition de cet exemple])[
  Une présentation réussie laisse au premier plan le texte et les relations
  que son auteur souhaite rendre visibles.
]

== Relire, discuter et transmettre

La relecture ne consiste pas seulement à repérer des erreurs. Elle éprouve la
solidité des transitions et la précision des termes. Un lecteur extérieur au
projet peut ainsi signaler qu’un passage suppose une connaissance implicite ou
qu’une conclusion apparaît avant les éléments qui la justifient.

Les versions successives doivent conserver la possibilité de comprendre les
choix effectués. Une proposition de modification gagne à être examinée dans son
ensemble, en rapport avec la consigne qui l’a motivée. Cette continuité importe
pour le travail partagé, mais aussi pour une personne qui reprend un texte après
plusieurs semaines d’interruption.

=== Trois repères pour la rédaction

- Une structure explicite permet de retrouver les étapes du raisonnement.
- Des informations éditoriales organisées évitent les doubles saisies.
- Un rendu stable facilite la relecture et la transmission du document.

Le tableau suivant rassemble les éléments utilisés dans cette démonstration.
Il s’agit d’un inventaire de présentation, sans données expérimentales.

#figure(
  table(columns: (1fr, 2fr),
    table.header([*Élément*], [*Rôle dans le document*]),
    [Titre court], [Identifier l’article dans l’en-tête courant.],
    [Affiliation], [Conserver la signature officielle de chaque auteur.],
    [Résumé], [Présenter la démarche dans chacune des langues retenues.],
  ),
  caption: [Éléments éditoriaux présents dans l’exemple.],
)

= Conclusion

Un document de travail doit rester agréable à lire alors même que son contenu
évolue. Une composition sobre n’exige pas de figer prématurément le texte : elle
fournit des repères assez stables pour accompagner ses transformations. Les
choix du modèle peuvent ensuite s’adapter aux exigences d’une revue ou aux
préférences d’un collectif, tandis que les informations demeurent structurées.

L’enjeu est de rendre cette organisation accessible sans demander à chacun de
maîtriser un langage de composition. Le titre, les auteurs et les résumés
constituent des informations à part entière ; leur présentation doit aider à
les reconnaître, puis laisser la place au développement de l’argumentation.
