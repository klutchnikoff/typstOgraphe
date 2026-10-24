// Interface publique de la capacité Dates, indépendante de Datify.
// format-date(value: str | datetime, lang: str = "fr") -> content
// Une chaîne ISO désigne une date fixe ; un libellé libre est conservé tel quel.
// Aucun accès à l’horloge : « aujourd’hui » est résolu par l’application à la saisie.
#import "../packages/datify-1.3.0/src/main.typ": custom-date-format

#let format-date(value, lang: "fr") = {
  assert(type(value) in (str, datetime), message: "Une date doit être une chaîne ou un datetime.")
  let date = if type(value) == str {
    if value.match(regex("^\\d{4}-\\d{2}-\\d{2}$")) == none { return [#value] }
    let parts = value.split("-").map(int)
    datetime(year: parts.at(0), month: parts.at(1), day: parts.at(2))
  } else { value }
  // Convention française : premier jour en ordinal, avec exposant.
  if lang == "fr" and date.day() == 1 {
    [1#super[er] #custom-date-format(date, pattern: "MMMM y", lang: lang)]
  } else {
    [#custom-date-format(date, pattern: "long", lang: lang)]
  }
}
