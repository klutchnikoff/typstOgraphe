// typst compile --root . --ignore-system-fonts fixtures/article-bibliography.typ output/pdf/article-bibliography.pdf
// --input style=ieee or --input style=apa exercises the same source in another style.
#import "../crates/core/templates/article-working.typ": article
#import "../crates/core/templates/capabilities/references.typ": citation
#show: article.with(
  options: (paper: "a4", landscape: false, language: "fr", bullet-marker: auto),
  localizations: ((id: "fr", language: "fr", title: [Lire et discuter les travaux scientifiques],
    short-title: [Lire et discuter], subtitle: [], abstract: [
      Cette recette illustre une bibliothèque locale en deux formats, les citations
      groupées et narratives et la bibliographie automatique. Les personnes et
      travaux nommés « exemple » sont fictifs.
    ], keywords: ("écriture", "bibliographie")),),
  references: (path("article-references.bib"), path("article-references.yaml")),
  citation-style: sys.inputs.at("style", default: "chicago-author-date"),
  show-outline: true,
)

= Introduction

La discussion s’appuie sur des travaux que le lecteur doit pouvoir retrouver
#citation[#cite(<bourdieu1980>, supplement: [p. 23–25])#cite(<durand2020>)].

= Mettre les références en relation

#citation(form: "prose")[#cite(<bourdieu1980>)]
permet ici d’illustrer la forme narrative. La présentation finale relève du style
retenu, tandis que les références et leurs compléments restent identiques.

La notice non citée présente dans le fichier BibLaTeX ne doit pas apparaître
dans la bibliographie. Les deux fichiers sont lus localement, sans téléchargement.
