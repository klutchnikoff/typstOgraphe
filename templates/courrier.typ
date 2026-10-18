// Contrat du type « courrier » : métadonnées nommées et corps en dernier argument.
#let courrier(
  expediteur: "",
  fonction: "",
  service: "",
  email: "",
  destinataire: "",
  adresse: "",
  objet: "",
  reference: "",
  lieu: "",
  date: "",
  compact: false,
  body,
) = {
  let accent = rgb("285c77")
  set document(title: objet, author: expediteur)
  set page(
    paper: "a4",
    margin: (top: 24mm, bottom: 23mm, left: 24mm, right: 24mm),
    footer: context {
      set text(size: 8pt, fill: rgb("697587"))
      line(length: 100%, stroke: 0.4pt + rgb("dfe4eb"))
      v(3pt)
      align(center)[Ville de Valmont · 1, place de la Mairie · 35000 Valmont]
      align(center)[#counter(page).display("1")]
    },
  )
  set text(font: "Libertinus Serif", size: 11pt, lang: "fr")
  set par(justify: true, leading: 0.6em, spacing: 1.1em)
  set heading(numbering: none)
  show heading: set text(fill: accent)
  show link: set text(fill: accent)

  text(size: 16pt, weight: "bold", fill: accent)[VILLE DE VALMONT]
  linebreak()
  text(size: 9pt, fill: accent)[#if compact { service } else { [DIRECTION GÉNÉRALE] }]
  v(8pt)
  line(length: 100%, stroke: (if compact { 0.5pt } else { 1.5pt }) + accent)
  v(16pt)

  let contenu = {
    align(right, block(width: 62%)[#destinataire #linebreak() #adresse])
    v(10pt)
    align(right)[#lieu, le #date]
    v(10pt)
    strong[Objet : #objet]
    v(9pt)
    body
    v(15pt)
    align(right)[#expediteur #linebreak() #text(size: 9pt)[#fonction]]
  }
  if compact {
    text(size: 9pt)[#expediteur · #service #linebreak() #email #linebreak() Réf. #reference]
    v(12pt)
    contenu
  } else {
    grid(
      columns: (30mm, 1fr),
      gutter: 8mm,
      {
        set par(justify: false)
        text(size: 8pt, fill: rgb("586578"))[
          *Votre interlocutrice* #linebreak()
          #expediteur #linebreak()
          #fonction
          #v(12pt)
          #email.replace("@", "@\u{200b}").replace(".", ".\u{200b}")
          #v(12pt)
          Référence #linebreak() #reference
        ]
      },
      contenu,
    )
  }
}
