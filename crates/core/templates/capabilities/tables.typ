// Stable typstOgraphe interface for explicit table header roles.
// External compilation: typst compile --features a11y-extras main.typ
// Typst's temporary PDF API is confined to this function.
#let table-header-cell(body, scope: "row") = {
  assert(scope in ("row", "column", "both"), message: "Invalid table header scope")
  assert("header-cell" in pdf, message: "Row headers require: typst compile --features a11y-extras main.typ")
  pdf.header-cell(scope: scope, body)
}
