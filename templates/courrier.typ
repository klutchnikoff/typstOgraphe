// Contrat du type « courrier » : métadonnées nommées et corps en dernier argument.
// Le créateur du modèle contrôle les personnalisations proposées dans l’UI.
// Pour autoriser quelques choix : bullet-markers: ("—", "•", "▪").
// custom-bullet-marker: true autorise aussi un caractère Unicode libre.
#let editor-options = (bullet-markers: (), custom-bullet-marker: false)

// Conventions du modèle, indépendantes de la langue de l’interface.
#let marqueurs-listes(language) = if language == "fr" { ([—], [•], [◦]) } else { ([•], [◦], [▪]) }

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
  options: (paper: "a4", landscape: false, language: "fr", bullet-marker: auto),
  body,
) = {
  let labels = (
    fr: (direction: "DIRECTION GÉNÉRALE", subject: "Objet", contact: "Votre interlocutrice", reference: "Référence", date-prefix: "le "),
    en: (direction: "GENERAL MANAGEMENT", subject: "Subject", contact: "Your contact", reference: "Reference", date-prefix: ""),
    de: (direction: "DIREKTION", subject: "Betreff", contact: "Ihre Ansprechperson", reference: "Referenz", date-prefix: "den "),
    es: (direction: "DIRECCIÓN GENERAL", subject: "Asunto", contact: "Su contacto", reference: "Referencia", date-prefix: "el "),
    it: (direction: "DIREZIONE GENERALE", subject: "Oggetto", contact: "Referente", reference: "Riferimento", date-prefix: "il "),
    pt: (direction: "DIREÇÃO GERAL", subject: "Assunto", contact: "Seu contato", reference: "Referência", date-prefix: "em "),
    nl: (direction: "ALGEMENE DIRECTIE", subject: "Onderwerp", contact: "Uw contactpersoon", reference: "Referentie", date-prefix: "op "),
  ).at(options.language)
  let accent = rgb("285c77")
  set document(title: objet, author: expediteur)
  set page(
    paper: options.paper,
    flipped: options.landscape,
    margin: (top: 24mm, bottom: 23mm, left: 24mm, right: 24mm),
    footer: context {
      set text(size: 8pt, fill: rgb("697587"))
      line(length: 100%, stroke: 0.4pt + rgb("dfe4eb"))
      v(3pt)
      align(center)[Ville de Valmont · 1, place de la Mairie · 35000 Valmont]
      align(center)[#counter(page).display("1")]
    },
  )
  set text(font: "Libertinus Serif", size: 11pt, lang: options.language)
  set par(justify: true, leading: 0.6em, spacing: 1.1em)
  set heading(numbering: none)
  set list(marker: if options.bullet-marker == auto {
    marqueurs-listes(options.language)
  } else {
    options.bullet-marker
  })
  // La charte du courrier décide des dimensions et des légendes, sans flottants.
  set image(width: 100%, height: 65mm, fit: "contain")
  set figure(placement: none, numbering: none, supplement: none, gap: 5pt)
  show figure.caption: set text(size: 9pt, fill: rgb("586578"))
  show heading: set text(fill: accent)
  show link: set text(fill: accent)
  // Les citations restent sémantiques dans le contenu ; le modèle les compose.
  show quote.where(block: true): it => block(
    width: 100%,
    inset: (left: 10pt, top: 5pt, bottom: 5pt),
    stroke: (left: (if compact { 0.5pt } else { 1pt }) + accent),
    above: 0.8em,
    below: 0.8em,
  )[
    #text(style: "italic")[#it.body]
    #if it.attribution != none {
      v(4pt)
      align(right, text(size: 9pt, fill: rgb("586578"))[#it.attribution])
    }
  ]

  text(size: 16pt, weight: "bold", fill: accent)[VILLE DE VALMONT]
  linebreak()
  text(size: 9pt, fill: accent)[#if compact { service } else { [#labels.direction] }]
  v(8pt)
  line(length: 100%, stroke: (if compact { 0.5pt } else { 1.5pt }) + accent)
  v(16pt)

  let contenu = {
    align(right, block(width: 62%)[#destinataire #linebreak() #adresse])
    v(10pt)
    align(right)[#lieu, #labels.date-prefix#date]
    v(10pt)
    strong[#labels.subject : #objet]
    v(9pt)
    body
    v(15pt)
    align(right)[#expediteur #linebreak() #text(size: 9pt)[#fonction]]
  }
  if compact {
    text(size: 9pt)[#expediteur · #service #linebreak() #email #linebreak() #labels.reference #reference]
    v(12pt)
    contenu
  } else {
    grid(
      columns: (30mm, 1fr),
      gutter: 8mm,
      {
        set par(justify: false)
        text(size: 8pt, fill: rgb("586578"))[
          *#labels.contact* #linebreak()
          #expediteur #linebreak()
          #fonction
          #v(12pt)
          #email.replace("@", "@\u{200b}").replace(".", ".\u{200b}")
          #v(12pt)
          #labels.reference #linebreak() #reference
        ]
      },
      contenu,
    )
  }
}
