// Contrat du type Compte-rendu. Le modèle est autonome et entièrement Typst.
#let editor-options = (bullet-markers: (), custom-bullet-marker: false)
#let compte-rendu(
  titre: "", date: "", lieu: "", redacteur: "", participants: "", excuses: "",
  options: (paper: "a4", landscape: false, language: "fr", bullet-marker: auto),
  body,
) = {
  let en = options.language == "en"
  let labels = if en {
    (title: "Meeting minutes", date: "Date", place: "Location / video call", author: "Written by", attendees: "Attendees", apologies: "Apologies")
  } else {
    (title: "Compte rendu de réunion", date: "Date", place: "Lieu / visioconférence", author: "Rédaction", attendees: "Personnes présentes", apologies: "Personnes excusées")
  }
  let accent = rgb("285c77")
  set document(title: if titre == "" { labels.title } else { titre }, author: redacteur)
  set page(paper: options.paper, flipped: options.landscape, margin: 24mm,
    footer: context align(center, text(size: 9pt, fill: luma(110))[#counter(page).display("1")]))
  set text(font: "Libertinus Serif", size: 11pt, lang: options.language)
  set par(justify: true, leading: 0.6em, spacing: 1em)
  set heading(numbering: none)
  show heading: set text(fill: accent)
  show link: set text(fill: accent)
  set list(marker: if options.bullet-marker != auto { options.bullet-marker } else if options.language == "fr" { [—] } else { [•] })
  set image(width: 100%, height: 65mm, fit: "contain")
  set figure(placement: none, numbering: none, supplement: none)
  text(size: 18pt, weight: "bold", fill: accent)[#labels.title]
  if titre != "" { parbreak(); text(size: 14pt, weight: "bold")[#titre] }
  v(6pt)
  line(length: 100%, stroke: 0.7pt + accent)
  v(8pt)
  for (label, value) in ((labels.date, date), (labels.place, lieu), (labels.author, redacteur)) {
    if value != "" { [#strong[#label :] #value]; parbreak() }
  }
  for (label, people) in ((labels.attendees, participants), (labels.apologies, excuses)) {
    if people != "" {
      strong[#label :]
      for person in people.split("\n") { linebreak(); [#person] }
      parbreak()
    }
  }
  v(8pt)
  body
}
