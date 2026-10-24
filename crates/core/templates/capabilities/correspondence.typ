// Capacités publiques du type Courrier : contenu riche, sans fichiers joints.
// Les mentions sont collectées sans être composées dans le corps principal.
#let postscript(body) = if body != [] {
  [#metadata(body) <typstographe-postscript>]
}
#let attachment(body) = if body != [] {
  [#metadata(body) <typstographe-attachment>]
}

// Le modèle fournit la présentation de chaque groupe après sa signature.
#let letter-notes(postscript: body => body, attachments: items => items.join(parbreak())) = context {
  let postscripts = query(<typstographe-postscript>).map(it => it.value)
  let attachment-items = query(<typstographe-attachment>).map(it => it.value)
  if postscripts.len() > 0 { postscript(postscripts.join(parbreak())) }
  if attachment-items.len() > 0 { attachments(attachment-items) }
}
