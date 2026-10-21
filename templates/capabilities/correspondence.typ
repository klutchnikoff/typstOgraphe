// Capacités publiques du type Courrier : contenu riche, sans fichiers joints.
// Les mentions sont collectées sans être composées dans le corps principal.
#let postscript(body) = if body != [] {
  [#metadata(body) <typstographe-postscript>]
}
#let piece-jointe(body) = if body != [] {
  [#metadata(body) <typstographe-piece-jointe>]
}

// Le modèle fournit la présentation de chaque groupe après sa signature.
#let notes-courrier(postscript: body => body, pieces-jointes: items => items.join(parbreak())) = context {
  let ps = query(<typstographe-postscript>).map(it => it.value)
  let pj = query(<typstographe-piece-jointe>).map(it => it.value)
  if ps.len() > 0 { postscript(ps.join(parbreak())) }
  if pj.len() > 0 { pieces-jointes(pj) }
}
