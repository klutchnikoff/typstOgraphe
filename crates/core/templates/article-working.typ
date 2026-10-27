// Neutral working article. Canonical data and helpers can be reused by other models.
#import "capabilities/authorship.typ": prepare-authorship, show-authorship
#import "capabilities/abstracts.typ": show-abstracts
#let editor-options = (bullet-markers: (), custom-bullet-marker: false)
#let article(localizations: (), authors: (), affiliations: (), options: (
  paper: "a4", landscape: false, language: "fr", bullet-marker: auto,
), body) = {
  let primary = localizations.find(l => l.language == options.language)
  let full-title = if primary == none { [] } else { primary.title }
  let running-title = if primary == none or primary.short-title == [] { full-title } else { primary.short-title }
  set document(title: full-title, author: authors.map(a => a.name), keywords: if primary == none { () } else { primary.keywords })
  set text(font: "Libertinus Serif", size: 11pt, lang: options.language)
  set page(paper: options.paper, flipped: options.landscape, margin: (x: 22mm, y: 23mm),
    header: context if counter(page).get().first() > 1 { text(size: 9pt, running-title) },
    footer: context align(center, text(size: 9pt, counter(page).display("1"))),
  )
  set par(justify: true, leading: 0.6em, spacing: 0.9em)
  set heading(numbering: none)
  set list(marker: if options.bullet-marker != auto { options.bullet-marker } else if options.language == "fr" { [—] } else { [•] })
  set image(width: 100%, height: 65mm, fit: "contain")
  set figure(placement: none)
  show figure.caption: set text(size: 9pt)
  show quote.where(block: true): it => block(inset: (left: 12pt), stroke: (left: 0.7pt), above: 8pt, below: 8pt)[
    #emph(it.body)
    #if it.attribution != none { block(align(right, text(size: 9pt, it.attribution))) }
  ]
  if full-title != [] { title(full-title) }
  if primary != none and primary.subtitle != [] { block(below: 6pt, emph(primary.subtitle)) }
  show-authorship(prepare-authorship(authors, affiliations))
  show-abstracts(localizations, primary-language: options.language)
  v(8pt)
  body
}
