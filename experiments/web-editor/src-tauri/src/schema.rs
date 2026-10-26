//! Adaptateur expérimental Tiptap → Typst. Aucun code Typst fourni par le client.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use typstographe_core::{assets::Assets, document::typst_string};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
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
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Value>,
}

impl Node {
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
    if document.kind != "doc" || document.text.is_some() || !document.marks.is_empty() {
        return Err("Un document structuré est attendu.".into());
    }
    let mut count = 0;
    limits(document, 0, &mut count)?;
    let result = blocks(&document.content, assets)?;
    if result.len() > 4 * 1024 * 1024 {
        return Err("Document trop volumineux.".into());
    }
    Ok(result)
}
fn limits(node: &Node, depth: usize, count: &mut usize) -> Result<(), String> {
    *count += 1;
    if depth > 16 || *count > 20_000 || node.text.as_ref().is_some_and(|s| s.len() > 512_000) {
        return Err("Structure trop volumineuse ou trop profonde.".into());
    }
    if node.kind != "text" && node.text.is_some() {
        return Err("Texte hors d'un nœud texte.".into());
    }
    if node.kind != "text" && !node.marks.is_empty() {
        return Err("Style hors d'un nœud texte.".into());
    }
    for child in &node.content {
        limits(child, depth + 1, count)?;
    }
    Ok(())
}
fn blocks(nodes: &[Node], assets: &Assets) -> Result<String, String> {
    nodes
        .iter()
        .map(|node| block(node, assets))
        .collect::<Result<Vec<_>, _>>()
        .map(|s| s.join("\n\n"))
}
fn block(node: &Node, assets: &Assets) -> Result<String, String> {
    Ok(match node.kind.as_str() {
        "paragraph" => format!("#par[{}]", inline(&node.content)?),
        "heading" => {
            let level = node.integer("level", 1);
            if !(1..=3).contains(&level) {
                return Err("Titre limité aux niveaux 1 à 3.".into());
            }
            format!("#heading(level: {level})[{}]", inline(&node.content)?)
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
                items.push(format!("[{}]", blocks(&item.content, assets)?));
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
            if !typstographe_core::assets::valid_path(path) || !assets.contains_key(path) {
                return Err("Image interne absente ou chemin invalide.".into());
            }
            let alt = typst_string(node.string("alt")?);
            let caption = inline(&node.content)?;
            let caption = if caption.is_empty() {
                String::new()
            } else {
                format!(", caption: [{caption}]")
            };
            format!(
                "#figure(image({}, alt: {alt}){caption})",
                typst_string(path)
            )
        }
        "table" => table(node, assets)?,
        other => return Err(format!("Structure non prise en charge : {other}")),
    })
}
fn table(node: &Node, assets: &Assets) -> Result<String, String> {
    let width = node.content.first().map_or(0, |r| r.content.len());
    if !(1..=12).contains(&width) || node.content.len() > 100 {
        return Err("Tableau limité à 12 colonnes et 100 lignes.".into());
    }
    let mut cells = Vec::new();
    for (index, row) in node.content.iter().enumerate() {
        if row.kind != "tableRow" || row.content.len() != width {
            return Err("Tableau non rectangulaire.".into());
        }
        let header = index == 0 && row.content.iter().all(|c| c.kind == "tableHeader");
        let mut content = Vec::new();
        for cell in &row.content {
            if !matches!(cell.kind.as_str(), "tableCell" | "tableHeader")
                || cell.integer("colspan", 1) != 1
                || cell.integer("rowspan", 1) != 1
            {
                return Err("Les cellules fusionnées sont hors du diagnostic.".into());
            }
            if cell.content.iter().any(|p| p.kind != "paragraph") {
                return Err("Une cellule contient seulement des paragraphes.".into());
            }
            content.push(format!("[{}]", blocks(&cell.content, assets)?));
        }
        cells.push(if header {
            format!("table.header({})", content.join(", "))
        } else {
            content.join(", ")
        });
    }
    Ok(format!("#table(columns: {width}, {})", cells.join(", ")))
}
fn inline(nodes: &[Node]) -> Result<String, String> {
    nodes
        .iter()
        .map(|node| {
            Ok(match node.kind.as_str() {
                "text" => {
                    if !node.content.is_empty() {
                        return Err("Un texte ne contient pas de blocs.".into());
                    }
                    let mut text = format!(
                        "#text({})",
                        typst_string(node.text.as_deref().unwrap_or_default())
                    );
                    for mark in &node.marks {
                        text = match mark.kind.as_str() {
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
                                    return Err(
                                        "Le lien doit utiliser http, https ou mailto.".into()
                                    );
                                }
                                format!("#link({})[{text}]", typst_string(url))
                            }
                            other => return Err(format!("Style non pris en charge : {other}")),
                        };
                    }
                    text
                }
                "hardBreak" if node.content.is_empty() => "#linebreak()".into(),
                "softBreak" if node.content.is_empty() => " ".into(),
                "footnote" if node.content.is_empty() => {
                    format!("#footnote[#text({})]", typst_string(node.string("text")?))
                }
                other => return Err(format!("Contenu en ligne non pris en charge : {other}")),
            })
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|items| items.concat())
}

#[cfg(test)]
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
        assert!(s.contains("#text(\"#panic(\\\"oops\\\") $x$ [é]\")"));
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
            "#par[#text(\"a\") #text(\"b\")#linebreak()#text(\"c\")]"
        );
    }
}
