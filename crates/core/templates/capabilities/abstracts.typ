// Localized rich frontmatter; UI language never controls document labels.
#let abstract-labels(lang) = (
  fr: (abstract: "Résumé", keywords: "Mots-clés"),
  en: (abstract: "Abstract", keywords: "Keywords"),
  de: (abstract: "Zusammenfassung", keywords: "Schlüsselwörter"),
  es: (abstract: "Resumen", keywords: "Palabras clave"),
  it: (abstract: "Riassunto", keywords: "Parole chiave"),
  pt: (abstract: "Resumo", keywords: "Palavras-chave"),
  nl: (abstract: "Samenvatting", keywords: "Trefwoorden"),
).at(lang)
#let default-entry(entry, primary: false) = {
  set text(lang: entry.language, size: 10pt)
  let labels = abstract-labels(entry.language)
  if not primary {
    if entry.title != [] { block(above: 8pt, below: 3pt, strong(entry.title)) }
    if entry.subtitle != [] { block(below: 3pt, emph(entry.subtitle)) }
  }
  if entry.abstract != [] {
    block(above: 6pt, below: 3pt, strong(labels.abstract))
    block(width: 100%, entry.abstract)
  }
  if entry.keywords.len() > 0 {
    block(above: 3pt, below: 6pt)[#strong(labels.keywords + ": ")#entry.keywords.join(", ")]
  }
}
#let show-abstracts(localizations, primary-language: "fr", render-entry: auto) = {
  let renderer = if render-entry == auto { default-entry } else { render-entry }
  let ordered = localizations.filter(l => l.language == primary-language) + localizations.filter(l => l.language != primary-language)
  for entry in ordered { renderer(entry, primary: entry.language == primary-language) }
}
