// Stable structured citation contract. Typst owns grouping and CSL formatting.
#let citation(body, form: "normal") = {
  // Keep native cite elements at their source location for preview navigation.
  set cite(form: form)
  body
}
#let reference-list(sources, style: "chicago-author-date") = {
  if sources.len() > 0 { bibliography(sources, style: style) }
}
