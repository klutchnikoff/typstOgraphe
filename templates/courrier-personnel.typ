// Maquette du modèle « Courrier français — enveloppe à fenêtre ».
// Référence : DL 110 × 220 mm, fenêtre 100 × 35 mm à 20 mm du bas et de droite.
// GPV 60013 / ES35F110 : https://myprint.gpvfrance.fr/products/60013-boite-de-500-enveloppes-blanches-dl-110x220-90-g-m2-fenetre-35x100-bande-de-protection-es35f110
// Feuille A4 pliée à 99 et 198 mm. Vérification papier à 100 % indispensable.
#let editor-options = (bullet-markers: (), custom-bullet-marker: false)

#import "capabilities/dates.typ": format-date
#import "capabilities/correspondence.typ": postscript, piece-jointe, notes-courrier

#let courrier(
  expediteur: "",
  fonction: "",
  service: "",
  email: "",
  organisme: "",
  adresse_expediteur: "",
  telephone: "",
  signature: none,
  destinataire: "",
  adresse: "",
  objet: "",
  reference: "",
  lieu: "",
  date: "",
  options: (paper: "a4", landscape: false, language: "fr", bullet-marker: auto),
  // Outil de maquette, sans commande ni réglage de composition dans l'éditeur.
  controle: false,
  body,
) = {
  assert(options.paper == "a4" and not options.landscape,
    message: "Le modèle pour enveloppe DL demande une page A4 en portrait.")
  let labels = (
    fr: (postscript: "P.-S.", attachments: "P.J.", subject: "Objet", reference: "Référence", date-prefix: "le "),
    en: (postscript: "P.S.", attachments: "Enclosures", subject: "Subject", reference: "Reference", date-prefix: ""),
    de: (postscript: "P.S.", attachments: "Anlagen", subject: "Betreff", reference: "Referenz", date-prefix: "den "),
    es: (postscript: "P.D.", attachments: "Adjuntos", subject: "Asunto", reference: "Referencia", date-prefix: "el "),
    it: (postscript: "P.S.", attachments: "Allegati", subject: "Oggetto", reference: "Riferimento", date-prefix: "il "),
    pt: (postscript: "P.S.", attachments: "Anexos", subject: "Assunto", reference: "Referência", date-prefix: "em "),
    nl: (postscript: "P.S.", attachments: "Bijlagen", subject: "Onderwerp", reference: "Referentie", date-prefix: "op "),
  ).at(options.language)
  set document(title: objet, author: expediteur)
  set text(font: "Libertinus Serif", size: 11pt, lang: options.language, fill: black)
  set par(justify: false, leading: 0.55em, spacing: 0.95em, first-line-indent: 0pt)
  set heading(numbering: none)
  set list(marker: if options.bullet-marker != auto {
    options.bullet-marker
  } else if options.language == "fr" { ([—], [•], [◦]) } else { ([•], [◦], [▪]) })
  set image(width: 100%, height: 65mm, fit: "contain")
  set figure(placement: none, numbering: none, supplement: none, gap: 5pt)
  show figure.caption: set text(size: 9pt)
  show link: underline
  show quote.where(block: true): it => block(
    width: 100%, inset: (left: 10pt, top: 4pt, bottom: 4pt),
    stroke: (left: 0.7pt + black), above: 0.8em, below: 0.8em,
  )[
    #emph(it.body)
    #if it.attribution != none { v(4pt); align(right, text(size: 9pt, it.attribution)) }
  ]

  // Le fond utilise les coordonnées de la feuille, indépendamment des marges.
  set page(
    paper: options.paper,
    margin: (left: 25mm, right: 25mm, top: 25mm, bottom: 23mm),
    footer: context if counter(page).get().first() > 1 {
      align(center, text(size: 9pt, counter(page).display("1")))
    },
    background: context if counter(page).get().first() == 1 {
      for y in (99mm, 198mm) {
        place(top + left, dx: 7mm, dy: y, line(length: 3mm, stroke: 0.3pt + luma(65%)))
      }
      if controle {
        // Feuille centrée dans l'enveloppe : jeu horizontal 10 mm, vertical 11 mm.
        place(top + left, dx: 95mm, dy: 49.5mm,
          rect(width: 100mm, height: 35mm, fill: none, stroke: (paint: rgb("286b91"), thickness: 0.6pt, dash: "dashed")))
        // Intersection des positions extrêmes : x=100..190, y=55..79 mm.
        place(top + left, dx: 100mm, dy: 55mm,
          rect(width: 90mm, height: 24mm, fill: none, stroke: 0.5pt + rgb("28805a")))
        for y in (99mm, 198mm) {
          place(top + left, dx: 12mm, dy: y,
            line(length: 186mm, stroke: (paint: luma(65%), thickness: 0.4pt, dash: "dashed")))
        }
        place(top + left, dx: 25mm, dy: 267mm, block(width: 160mm)[
          #set text(font: "Libertinus Sans", size: 8pt)
          #set par(leading: 0.4em)
          *CONTRÔLE DU PLIAGE — imprimer à 100 %*
          #linebreak()
          #text(fill: rgb("286b91"))[Bleu : fenêtre, feuille centrée.]
          #linebreak()
          #text(fill: rgb("28805a"))[Vert : zone visible pour un jeu théorique de 10 × 11 mm.]
          #linebreak()
          Plis à 99 et 198 mm. Référence : GPV 60013, DL 110 × 220, fenêtre 100 × 35, position 20/20.
        ])
      }
    },
  )

  let coordonnees = {
    set text(size: 10pt)
    set par(leading: 0.35em)
    let lignes = (
      if expediteur != "" { strong(expediteur) },
      organisme, fonction, service, adresse_expediteur, telephone, email,
    ).filter(it => it != none and it != "")
    lignes.join(linebreak())
  }
  let destinataire-bloc = {
    set text(size: 10.5pt, hyphenate: false)
    set par(justify: false, leading: 0.35em)
    (destinataire, adresse).filter(it => it != "").join(linebreak())
  }
  // Le texte reste dans la partie commune des positions de fenêtre, avec du jeu.
  // Une adresse trop longue est signalée ; elle n'est ni coupée ni miniaturisée.
  context {
    for mot in (destinataire + " " + adresse).split(regex("\\s+")) {
      assert(measure(text(size: 10.5pt, mot)).width <= 82mm,
        message: "Un élément de l'adresse est trop large pour la fenêtre DL.")
    }
    assert(measure(block(width: 82mm, destinataire-bloc)).height <= 18mm,
      message: "L'adresse dépasse la zone de la fenêtre DL : raccourcissez ses lignes ou choisissez un autre modèle.")
    assert(measure(block(width: 70mm, coordonnees)).height <= 59mm,
      message: "Les coordonnées dépassent l'en-tête du courrier : réduisez les champs facultatifs.")
  }
  // Place est relatif au début de la zone de texte (25 mm depuis les bords).
  place(top + left, block(width: 70mm, coordonnees))
  place(top + left, dx: 79mm, dy: 33mm,
    block(width: 82mm, height: 18mm, align(left + horizon, destinataire-bloc)))

  // Seul le premier feuillet réserve une grande zone pour les adresses.
  v(63mm)
  let datation = if date == "" { lieu } else {
    [#if lieu != "" { [#lieu, ] }#labels.date-prefix#format-date(date, lang: options.language)]
  }
  if datation != "" { align(right, datation); v(7mm) }
  if objet != "" { block(below: 3mm, strong[#labels.subject : #objet]) }
  if reference != "" { block(below: 3mm, text(size: 10pt)[#labels.reference : #reference]) }
  v(5mm)
  body
  // Même sans image, réserver la même place pour une signature manuscrite.
  // La signature et le nom restent ensemble ; la signature n'est pas un flottant.
  let elements = ()
  if signature != none {
    elements.push(image(signature, width: 35mm, height: 14mm, fit: "contain", alt: expediteur))
  } else {
    elements.push(box(width: 35mm, height: 14mm))
  }
  if expediteur != "" { elements.push(text(expediteur)) }
  if fonction != "" { elements.push(text(size: 10pt, fonction)) }
  block(width: 100%, breakable: false, above: 8mm)[
    #align(right, box(width: 65mm,
      align(center, stack(dir: ttb, spacing: 2mm, ..elements))))
  ]
  notes-courrier(
    postscript: body => block(above: 6mm)[#strong[#labels.postscript :] #body],
    pieces-jointes: items => block(above: 5mm)[
      #strong[#labels.attachments :]
      #list(..items)
    ],
  )
}
