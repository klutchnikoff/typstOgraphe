// A restrained working article. All presentation choices belong to this model.
#import "capabilities/authorship.typ": prepare-authorship, show-authorship
#import "capabilities/abstracts.typ": show-abstracts, abstract-labels
#import "capabilities/references.typ": reference-list
#let editor-options = (bullet-markers: (), custom-bullet-marker: false, outline: true, citation-styles: ("chicago-author-date", "apa", "ieee"))
#let article(localizations: (), authors: (), affiliations: (), references: (), citation-style: "chicago-author-date", show-outline: false, options: (
  paper: "a4", landscape: false, language: "fr", bullet-marker: auto,
), body) = {
  let primary = localizations.find(l => l.language == options.language)
  let full-title = if primary == none { [] } else { primary.title }
  let running-title = if primary == none or primary.short-title == [] { full-title } else { primary.short-title }
  let compact = options.paper == "a5" and not options.landscape
  let ink = luma(12%)
  let secondary = luma(32%)
  let hairline = 0.35pt + luma(72%)

  set document(title: full-title, author: authors.map(a => a.name), keywords: if primary == none { () } else { primary.keywords })
  set text(font: "Libertinus Serif", size: 11pt, lang: options.language, fill: ink)
  set page(
    paper: options.paper, flipped: options.landscape,
    margin: if compact { (x: 17mm, top: 21mm, bottom: 19mm) } else { (x: 28mm, top: 29mm, bottom: 25mm) },
    header-ascent: 50%,
    header: context if counter(page).get().first() > 1 {
      set text(size: 8.5pt, fill: secondary)
      set par(justify: false)
      block(width: 100%, inset: (bottom: 2mm), stroke: (bottom: hairline), running-title)
    },
    footer: context align(center, text(size: 9pt, fill: secondary, counter(page).display("1"))),
  )
  set par(justify: true, leading: 0.65em, spacing: 1.05em, first-line-indent: 0pt)
  set heading(numbering: none)
  show title: set text(size: if compact { 20pt } else { 23pt }, weight: "bold", hyphenate: false)
  show title: set block(above: 0pt, below: 0pt)
  show heading: set text(hyphenate: false)
  show heading.where(level: 1): set text(size: 14pt, weight: "bold")
  show heading.where(level: 1): set block(above: 1.5em, below: 0.65em)
  show heading.where(level: 2): set text(size: 11.5pt, weight: "bold")
  show heading.where(level: 2): set block(above: 1.2em, below: 0.55em)
  show heading.where(level: 3): set text(size: 11pt, weight: "regular", style: "italic")
  show heading.where(level: 3): set block(above: 1em, below: 0.5em)
  set list(marker: if options.bullet-marker != auto { options.bullet-marker } else if options.language == "fr" { [—] } else { [•] })
  set image(width: 100%, height: 65mm, fit: "contain")
  set table(align: left, inset: 7pt, stroke: (x: none, y: 0.4pt + luma(75%)))
  show table: set par(justify: false)
  set figure(placement: none, gap: 6pt)
  show figure.caption: set text(size: 9pt, fill: secondary)
  show quote.where(block: true): it => block(
    width: 100%, inset: (left: 11pt, top: 3pt, bottom: 3pt),
    stroke: (left: 0.7pt + luma(60%)), above: 0.85em, below: 0.85em,
  )[
    #emph(it.body)
    #if it.attribution != none { v(4pt); align(right, text(size: 9pt, fill: secondary, it.attribution)) }
  ]

  // Keep the title and subtitle together; let long author/affiliation lists flow.
  if full-title != [] or (primary != none and primary.subtitle != []) {
    block(width: 100%, breakable: false, below: 7mm)[
      #set align(center)
      #set par(justify: false, leading: 0.5em)
      #if full-title != [] { title(full-title) }
      #if primary != none and primary.subtitle != [] {
        v(3mm)
        text(size: 12pt, style: "italic", fill: secondary, hyphenate: false, primary.subtitle)
      }
    ]
  }
  {
    set align(center)
    set par(justify: false, leading: 0.45em)
    show-authorship(prepare-authorship(authors, affiliations))
  }

  // The shared helper still orders and dispatches the languages. Its renderer
  // belongs to the model, so other models remain free to choose another layout.
  let render-abstract(entry, primary: false) = {
    let labels = abstract-labels(entry.language)
    let translated-heading = not primary and (entry.title != [] or entry.subtitle != [])
    if translated-heading or entry.abstract != [] or entry.keywords.len() > 0 {
      block(width: 100%, inset: (x: if compact { 3mm } else { 7mm }), above: 6mm, below: 2mm)[
        #set text(lang: entry.language, size: 10pt)
        #set par(leading: 0.5em, spacing: 0.65em)
        #if not primary {
          if entry.title != [] {
            block(breakable: false, sticky: true, below: 2mm, text(size: 11pt, weight: "bold", entry.title))
          }
          if entry.subtitle != [] { block(breakable: false, sticky: true, below: 2mm, emph(entry.subtitle)) }
        }
        #if entry.abstract != [] {
          block(breakable: false, sticky: true, below: 2mm, text(weight: "bold", smallcaps(labels.abstract)))
          entry.abstract
        }
        #if entry.keywords.len() > 0 {
          let label = labels.keywords + if entry.language == "fr" { " : " } else { ": " }
          let separator = if entry.language == "fr" { " ; " } else { "; " }
          block(above: 2mm, text(size: 9pt, fill: secondary)[#strong(label)#entry.keywords.join(separator)])
        }
      ]
    }
  }
  show-abstracts(localizations, primary-language: options.language, render-entry: render-abstract)
  if show-outline { outline() }
  v(5mm)
  body
  reference-list(references, style: citation-style)
}
