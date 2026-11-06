//! Générateur du contrat structuré → Typst. Aucun code Typst fourni par le client.
use crate::{assets::Assets, document::typst_string};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Node {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Value>,
}

impl Node {
    /// Text offsets use UTF-16; an inline object or a line break occupies one unit.
    pub(crate) fn inline_len(&self) -> usize {
        match self.kind.as_str() {
            "mathInline" => 2 + self.content.iter().map(Self::inline_len).sum::<usize>(),
            "text" => self.text.as_deref().unwrap_or("").encode_utf16().count(),
            "softBreak" | "hardBreak" | "footnote" | "citation" | "crossReference" => 1,
            _ => self.content.iter().map(Self::inline_len).sum(),
        }
    }
    fn string(&self, key: &str) -> Result<&str, String> {
        self.attrs
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Attribut {key} absent pour {}", self.kind))
    }
    fn integer(&self, key: &str, fallback: u64) -> u64 {
        self.attrs
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(Value::as_u64)
            .unwrap_or(fallback)
    }
}

pub fn source(document: &Node, assets: &Assets) -> Result<String, String> {
    generate(document, assets, false)
}
pub fn source_with_metadata(
    document: &Node,
    assets: &Assets,
    metadata: &crate::document_types::Metadata,
) -> Result<String, String> {
    generate_resolved(document, assets, false, &citation_keys(metadata))
}
pub(super) type CitationKeys = std::collections::BTreeMap<String, String>;
pub(super) fn citation_keys(metadata: &crate::document_types::Metadata) -> CitationKeys {
    match metadata {
        crate::document_types::Metadata::Article(meta) => meta
            .references
            .iter()
            .map(|r| (String::from(r.id().clone()), r.key().into()))
            .collect(),
        _ => Default::default(),
    }
}
pub(super) fn generate(document: &Node, assets: &Assets, mapped: bool) -> Result<String, String> {
    generate_resolved(document, assets, mapped, &Default::default())
}
pub(super) fn generate_resolved(
    document: &Node,
    assets: &Assets,
    mapped: bool,
    keys: &CitationKeys,
) -> Result<String, String> {
    if document.kind != "doc" || document.text.is_some() || !document.marks.is_empty() {
        return Err("Un document structuré est attendu.".into());
    }
    let mut count = 0;
    limits(document, 0, &mut count, mapped)?;
    super::references::validate(document)?;
    super::mathematics::validate(document)?;
    let mut result = blocks(&document.content, assets, mapped, keys)?;
    fn notes(node: &Node) -> bool {
        matches!(node.kind.as_str(), "postscript" | "attachment") || node.content.iter().any(notes)
    }
    fn citations(node: &Node) -> bool {
        node.kind == "citation" || node.content.iter().any(citations)
    }
    if citations(document) {
        result = format!("#import \"capabilities/references.typ\": citation\n\n{result}");
    }
    fn cross_references(node: &Node) -> bool {
        node.kind == "crossReference" || node.content.iter().any(cross_references)
    }
    if cross_references(document) {
        result =
            format!("#import \"capabilities/cross-references.typ\": cross-reference\n\n{result}");
    }
    fn math(node: &Node) -> bool {
        super::mathematics::is_math(&node.kind) || node.content.iter().any(math)
    }
    if math(document) {
        result = format!("#import \"capabilities/mathematics.typ\": math-formula\n\n{result}");
    }
    if notes(document) {
        result = format!(
            "#import \"capabilities/correspondence.typ\": postscript, attachment\n\n{result}"
        );
    }
    fn row_headers(node: &Node) -> bool {
        matches!(super::tables::scope(node), Some("row" | "both"))
            || node.content.iter().any(row_headers)
    }
    if row_headers(document) {
        result = format!(
            "// External Typst CLI: typst compile --features a11y-extras main.typ\n#import \"capabilities/tables.typ\": table-header-cell\n\n{result}"
        );
    }
    if result.len() > 4 * 1024 * 1024 {
        return Err("Document trop volumineux.".into());
    }
    Ok(result)
}
fn limits(node: &Node, depth: usize, count: &mut usize, mapped: bool) -> Result<(), String> {
    super::attributes::validate(node, mapped)?;
    *count += 1;
    if depth > 16 || *count > 20_000 || node.text.as_ref().is_some_and(|s| s.len() > 512_000) {
        return Err("Structure trop volumineuse ou trop profonde.".into());
    }
    if node.kind != "text" && node.text.is_some() {
        return Err("Texte hors d'un nœud texte.".into());
    }
    if !matches!(
        node.kind.as_str(),
        "text"
            | "softBreak"
            | "hardBreak"
            | "footnote"
            | "citation"
            | "crossReference"
            | "mathInline"
    ) && !node.marks.is_empty()
    {
        return Err("Style hors d'un contenu en ligne.".into());
    }
    for child in &node.content {
        limits(child, depth + 1, count, mapped)?;
    }
    Ok(())
}
fn blocks(
    nodes: &[Node],
    assets: &Assets,
    mapped: bool,
    keys: &CitationKeys,
) -> Result<String, String> {
    nodes
        .iter()
        .map(|node| block(node, assets, mapped, keys))
        .collect::<Result<Vec<_>, _>>()
        .map(|s| s.join("\n\n"))
}
fn block(
    node: &Node,
    assets: &Assets,
    mapped: bool,
    keys: &CitationKeys,
) -> Result<String, String> {
    let result = match node.kind.as_str() {
        "mathBlock" => mapped_math(node, mapped)?,
        "paragraph" => format!("#par[{}]", inline_resolved(&node.content, mapped, keys)?),
        "heading" => {
            let level = node.integer("level", 1);
            if !(1..=3).contains(&level) {
                return Err("Titre limité aux niveaux 1 à 3.".into());
            }
            format!(
                "#heading(level: {level})[{}]",
                inline_resolved(&node.content, mapped, keys)?
            )
        }
        "bulletList" | "orderedList" => {
            let mut items = Vec::new();
            for item in &node.content {
                if item.kind != "listItem"
                    || item.content.is_empty()
                    || item.content.iter().any(|p| p.kind != "paragraph")
                {
                    return Err("Seules les listes non imbriquées sont prises en charge.".into());
                }
                items.push(format!(
                    "[{}]",
                    blocks(&item.content, assets, mapped, keys)?
                ));
            }
            let function = if node.kind == "bulletList" {
                "list"
            } else {
                "enum"
            };
            let start = if function == "enum" {
                format!("start: {}, ", node.integer("start", 1))
            } else {
                String::new()
            };
            format!("#{function}({start}{})", items.join(", "))
        }
        "figure" => {
            let path = node.string("src")?;
            if !crate::assets::valid_path(path) || !assets.contains_key(path) {
                return Err("Image interne absente ou chemin invalide.".into());
            }
            let alt = typst_string(node.string("alt")?);
            figure(
                &format!("image({}, alt: {alt})", typst_string(path)),
                &node.content,
                mapped,
                keys,
            )?
        }
        "table" => table_figure(node, &[], assets, mapped, keys)?,
        "tableFigure" => {
            if node.content.len() != 2
                || node.content[0].kind != "table"
                || node.content[1].kind != "tableCaption"
            {
                return Err("Tableau et légende attendus.".into());
            }
            table_figure(
                &node.content[0],
                &node.content[1].content,
                assets,
                mapped,
                keys,
            )?
        }
        "quote" => {
            let annotation = node
                .content
                .last()
                .filter(|child| child.kind == "quoteAttribution")
                .ok_or("Attribution attendue dans la citation.")?;
            let attribution = inline_resolved(&annotation.content, mapped, keys)?;
            let option = if attribution.is_empty() {
                String::new()
            } else {
                format!(", attribution: [{attribution}]")
            };
            format!(
                "#quote(block: true{option})[{}]",
                blocks(
                    &node.content[..node.content.len() - 1],
                    assets,
                    mapped,
                    keys
                )?
            )
        }
        "descriptionList" => {
            let mut items = Vec::new();
            for item in &node.content {
                if item.kind != "descriptionItem"
                    || item.content.len() != 2
                    || item.content[0].kind != "descriptionTerm"
                    || item.content[1].kind != "descriptionDefinition"
                {
                    return Err("Description attendue : terme puis définition.".into());
                }
                items.push(format!(
                    "terms.item([{}], [{}])",
                    inline_resolved(&item.content[0].content, mapped, keys)?,
                    inline_resolved(&item.content[1].content, mapped, keys)?
                ));
            }
            format!("#terms({})", items.join(", "))
        }
        "postscript" | "attachment" => {
            format!(
                "#{}[{}]",
                node.kind,
                inline_resolved(&node.content, mapped, keys)?
            )
        }
        other => return Err(format!("Structure non prise en charge : {other}")),
    };
    Ok(if let Some(id) = super::references::anchor(node) {
        format!(
            "{result}#label({})",
            typst_string(&super::references::label(id))
        )
    } else {
        result
    })
}
fn table_figure(
    node: &Node,
    caption: &[Node],
    assets: &Assets,
    mapped: bool,
    keys: &CitationKeys,
) -> Result<String, String> {
    let grid = table(node, assets, mapped, keys)?;
    figure(
        grid.strip_prefix('#').expect("table expression"),
        caption,
        mapped,
        keys,
    )
}
/// Common native figure envelope; accessibility descriptions remain on the image body.
fn figure(
    body: &str,
    caption: &[Node],
    mapped: bool,
    keys: &CitationKeys,
) -> Result<String, String> {
    let caption = inline_resolved(caption, mapped, keys)?;
    let option = if caption.is_empty() {
        String::new()
    } else {
        format!(", caption: [{caption}]")
    };
    Ok(format!("#figure({body}{option})"))
}

fn table(
    node: &Node,
    assets: &Assets,
    mapped: bool,
    keys: &CitationKeys,
) -> Result<String, String> {
    let width = node.content.first().map_or(0, |r| r.content.len());
    if !(1..=12).contains(&width) || node.content.len() > 100 {
        return Err("Tableau limité à 12 colonnes et 100 lignes.".into());
    }
    let (column_headers, row_headers) = super::tables::headers(node);
    let mut cells = Vec::new();
    for (index, row) in node.content.iter().enumerate() {
        if row.kind != "tableRow" || row.content.len() != width {
            return Err("Tableau non rectangulaire.".into());
        }
        let header = index == 0 && column_headers;
        let mut content = Vec::new();
        for (column, cell) in row.content.iter().enumerate() {
            if !matches!(cell.kind.as_str(), "tableCell" | "tableHeader")
                || cell.integer("colspan", 1) != 1
                || cell.integer("rowspan", 1) != 1
            {
                return Err("Les cellules fusionnées sont hors du diagnostic.".into());
            }
            if cell.content.iter().any(|p| p.kind != "paragraph") {
                return Err("Une cellule contient seulement des paragraphes.".into());
            }
            let scope = super::tables::scope(cell);
            if scope != super::tables::cell_scope(column_headers, row_headers, index, column)
                || cell
                    .attrs
                    .as_ref()
                    .and_then(|a| a.get("scope"))
                    .is_some_and(|v| {
                        !v.is_null()
                            && (cell.kind == "tableCell"
                                || !matches!(v.as_str(), Some("column" | "row" | "both")))
                    })
            {
                return Err("Invalid table header scope.".into());
            }
            let body = format!("[{}]", blocks(&cell.content, assets, mapped, keys)?);
            content.push(match scope {
                Some("row" | "both") => format!(
                    "table-header-cell({body}, scope: {})",
                    typst_string(scope.unwrap())
                ),
                _ => body,
            });
        }
        cells.push(if header {
            format!("table.header({})", content.join(", "))
        } else {
            content.join(", ")
        });
    }
    Ok(format!("#table(columns: {width}, {})", cells.join(", ")))
}
fn inline_resolved(nodes: &[Node], mapped: bool, keys: &CitationKeys) -> Result<String, String> {
    nodes
        .iter()
        .map(|node| {
            let text = match node.kind.as_str() {
                "text" => {
                    if !node.content.is_empty() {
                        return Err("Un texte ne contient pas de blocs.".into());
                    }
                    let value = super::mapping::literal(node.text.as_deref().unwrap_or_default());
                    if mapped {
                        format!("/*@typstographe:{}*/{value}", node.integer("_mapping", 0))
                    } else {
                        value
                    }
                }
                "mathInline" => mapped_math(node, mapped)?,
                "hardBreak" if node.content.is_empty() => "#linebreak()".into(),
                "softBreak" if node.content.is_empty() => " ".into(),
                "citation" if node.content.is_empty() => {
                    let mut attrs = node.attrs.clone().unwrap_or_default();
                    if let Some(object) = attrs.as_object_mut() {
                        object.remove("_mapping");
                    }
                    let citation: crate::bibliography::Citation =
                        serde_json::from_value(attrs).map_err(|e| e.to_string())?;
                    citation.validate()?;
                    let items = citation
                        .items
                        .iter()
                        .map(|item| {
                            Ok(format!(
                                "#cite(label({}), supplement: {})",
                                typst_string(
                                    keys.get(&String::from(item.record_id.clone())).ok_or_else(
                                        || format!(
                                            "Missing bibliography record: {}",
                                            String::from(item.record_id.clone())
                                        )
                                    )?
                                ),
                                if item.supplement.is_empty() {
                                    "none".into()
                                } else {
                                    format!("[{}]", super::mapping::literal(&item.supplement))
                                }
                            ))
                        })
                        .collect::<Result<String, String>>()?;
                    let expression =
                        format!("#citation(form: {})[{items}]", typst_string(&citation.form));
                    if mapped {
                        format!(
                            "/*@typstographe:{}:{}*/{expression}",
                            node.integer("_mapping", 0),
                            expression.len()
                        )
                    } else {
                        expression
                    }
                }
                "crossReference" if node.content.is_empty() => {
                    let mut attrs = node.attrs.clone().unwrap_or_default();
                    if let Some(a) = attrs.as_object_mut() {
                        a.remove("_mapping");
                    }
                    let r: super::references::Reference =
                        serde_json::from_value(attrs).map_err(|_| "invalid_cross_reference")?;
                    r.validate()?;
                    let expression = format!(
                        "#cross-reference(label({}), include-label: {}, include-page: {})",
                        typst_string(&super::references::label(&r.target)),
                        r.include_label,
                        r.include_page
                    );
                    if mapped {
                        format!(
                            "/*@typstographe:{}:{}*/{expression}",
                            node.integer("_mapping", 0),
                            expression.len()
                        )
                    } else {
                        expression
                    }
                }
                "footnote" if node.content.is_empty() => {
                    let value = super::mapping::literal(node.string("text")?);
                    let marker = if mapped {
                        format!("/*@typstographe:{}*/", node.integer("_mapping", 0))
                    } else {
                        String::new()
                    };
                    format!("#footnote[{marker}{value}]")
                }
                other => return Err(format!("Contenu en ligne non pris en charge : {other}")),
            };
            node.marks.iter().try_fold(text, |text, mark| {
                Ok(match mark.kind.as_str() {
                    "bold" => format!("#strong[{text}]"),
                    "italic" => format!("#emph[{text}]"),
                    "link" => {
                        let url = mark
                            .attrs
                            .as_ref()
                            .and_then(|v| v.get("href"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if !["https://", "http://", "mailto:"]
                            .iter()
                            .any(|prefix| url.starts_with(prefix))
                        {
                            return Err("Le lien doit utiliser http, https ou mailto.".into());
                        }
                        format!("#link({})[{text}]", typst_string(url))
                    }
                    other => return Err(format!("Style non pris en charge : {other}")),
                })
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|items| items.concat())
}

fn mapped_math(node: &Node, mapped: bool) -> Result<String, String> {
    let expression = super::mathematics::source(node)?;
    Ok(if mapped {
        format!(
            "/*@typstographe:{}:{}*/{expression}",
            node.integer("_mapping", 0),
            expression.len()
        )
    } else {
        expression
    })
}

pub(super) fn inline(nodes: &[Node], mapped: bool) -> Result<String, String> {
    inline_resolved(nodes, mapped, &Default::default())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    fn node(value: Value) -> Node {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn text_and_attributes_never_become_executable_typst() {
        let d = node(
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"#panic(\"oops\") $x$ [é]"}]}]}),
        );
        let s = source(&d, &Assets::default()).unwrap();
        assert!(s.contains(r#"\#panic("oops") \$x\$ \[é\]"#));
        let invalid = node(
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"x","marks":[{"type":"link","attrs":{"href":"javascript:alert(1)"}}]}]}]}),
        );
        assert!(source(&invalid, &Assets::default()).is_err());
    }
    #[test]
    fn invalid_cells_nested_lists_missing_images_and_unknown_nodes_are_refused() {
        for content in [
            serde_json::json!([{"type":"table","content":[{"type":"tableRow","content":[]}]}]),
            serde_json::json!([{"type":"figure","attrs":{"src":"images/missing.svg","alt":""}}]),
            serde_json::json!([{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"bulletList"}]}]}]),
            serde_json::json!([{"type":"script"}]),
        ] {
            assert!(
                source(
                    &node(serde_json::json!({"type":"doc","content":content})),
                    &Assets::default()
                )
                .is_err()
            );
        }
    }
    #[test]
    fn soft_and_forced_breaks_have_distinct_typst_meanings() {
        let document = node(
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"a"},{"type":"softBreak"},{"type":"text","text":"b"},{"type":"hardBreak"},{"type":"text","text":"c"}]}]}),
        );
        assert_eq!(
            source(&document, &Assets::default()).unwrap(),
            "#par[a b#linebreak()c]"
        );
    }
    #[test]
    fn pasted_inline_annotations_can_keep_their_surrounding_style() {
        let document = node(
            serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"hardBreak","marks":[{"type":"bold"}]},{"type":"footnote","attrs":{"text":"Annotation"},"marks":[{"type":"italic"}]}]}]}),
        );
        let source = source(&document, &Assets::default()).unwrap();
        assert!(source.contains("#strong[#linebreak()]"));
        assert!(source.contains("#emph[#footnote[Annotation]]"));
    }
}
