// typstOgraphe authorship contract. Original implementation; Pubmatter is a design reference.
// Inputs use canonical IDs and official free-text signatures, not inferred institution names.
#let prepare-authorship(authors, affiliations) = {
  let used = ()
  for author in authors {
    for id in author.affiliation-ids {
      assert(affiliations.any(a => a.id == id), message: "Unknown affiliation ID")
      if not used.contains(id) { used.push(id) }
    }
  }
  (
    authors: authors.map(a => (..a, affiliation-indices: a.affiliation-ids.map(id => used.position(it => it == id) + 1))),
    affiliations: used.enumerate().map(((index, id)) => (..affiliations.find(a => a.id == id), index: index + 1)),
  )
}
#let default-author(author) = {
  author.name
  if author.affiliation-indices.len() > 0 { super(author.affiliation-indices.map(str).join(",")) }
  if author.corresponding { super("*") }
  if author.orcid != "" { [ #link("https://orcid.org/" + author.orcid)[ORCID]] }
}
#let default-affiliation(affiliation) = [#super(str(affiliation.index)) #affiliation.text]
#let show-authorship(prepared, render-author: auto, render-affiliation: auto) = {
  let author-fn = if render-author == auto { default-author } else { render-author }
  let affiliation-fn = if render-affiliation == auto { default-affiliation } else { render-affiliation }
  if prepared.authors.len() > 0 {
    block(width: 100%, above: 5pt, below: 4pt, prepared.authors.map(author-fn).join([, ]))
  }
  if prepared.affiliations.len() > 0 {
    block(width: 100%, below: 6pt, text(size: 9pt, prepared.affiliations.map(affiliation-fn).join(linebreak())))
  }
  let contacts = prepared.authors.filter(a => a.corresponding and a.email != "")
  if contacts.len() > 0 {
    block(below: 6pt, text(size: 9pt, contacts.map(a => [#text("*") #a.name: #link("mailto:" + a.email)[#a.email]]).join(linebreak())))
  }
}
