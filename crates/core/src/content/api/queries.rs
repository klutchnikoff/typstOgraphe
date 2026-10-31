use super::{Call, invalid};
use crate::content::{
    Node,
    commands::{Command, TextTarget, TextZone, TreeRoot, TreeTarget},
    contract::{Error, State},
    shared::targets,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Debug, Serialize)]
pub struct Zone {
    pub zone: TextZone,
    pub text: String,
    pub length: u32,
    pub supports_styles: bool,
}
fn inline(node: &Node) -> String {
    node.content
        .iter()
        .map(|n| match n.kind.as_str() {
            "text" => n.text.clone().unwrap_or_default(),
            "softBreak" | "hardBreak" => "\n".into(),
            _ => "\u{fffc}".into(),
        })
        .collect()
}
fn walk(node: &Node, path: &mut Vec<usize>, root: &TreeRoot, out: &mut Vec<Zone>) {
    if matches!(
        node.kind.as_str(),
        "paragraph"
            | "heading"
            | "descriptionTerm"
            | "descriptionDefinition"
            | "quoteAttribution"
            | "figure"
            | "tableCaption"
            | "postscript"
            | "attachment"
    ) {
        let text = inline(node);
        let zone = match root {
            TreeRoot::Body => TextZone::Body { path: path.clone() },
            TreeRoot::Record {
                collection,
                id,
                field,
            } => TextZone::Record {
                collection: collection.clone(),
                id: id.clone(),
                field: field.clone(),
                path: path.clone(),
            },
        };
        out.push(Zone {
            zone,
            length: text.encode_utf16().count() as u32,
            text,
            supports_styles: true,
        });
    }
    if node.kind == "footnote" {
        let text = node
            .attrs
            .as_ref()
            .and_then(|a| a["text"].as_str())
            .unwrap_or("")
            .to_string();
        // Rich headers do not currently allow notes.
        if matches!(root, TreeRoot::Body) {
            out.push(Zone {
                zone: TextZone::Annotation { path: path.clone() },
                length: text.encode_utf16().count() as u32,
                text,
                supports_styles: false,
            });
        }
    }
    for (i, n) in node.content.iter().enumerate() {
        path.push(i);
        walk(n, path, root, out);
        path.pop();
    }
}
pub fn zones(state: &State) -> Vec<Zone> {
    let mut out = vec![];
    walk(&state.document, &mut vec![], &TreeRoot::Body, &mut out);
    for field in state.settings.metadata.field_values() {
        if let Some(text) = field.value.as_str() {
            out.push(Zone {
                zone: TextZone::Metadata { field: field.field },
                length: text.encode_utf16().count() as u32,
                text: text.into(),
                supports_styles: false,
            });
        }
    }
    let metadata = serde_json::to_value(&state.settings.metadata).unwrap();
    if let Some(schema) = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
    {
        for collection in schema["collections"].as_array().unwrap() {
            let name = collection["parameter"].as_str().unwrap();
            for record in metadata["fields"][name].as_array().into_iter().flatten() {
                let Some(id) = record["id"].as_str() else {
                    continue;
                };
                for field in collection["fields"].as_array().unwrap() {
                    let key = field["key"].as_str().unwrap();
                    let root = TreeRoot::Record {
                        collection: name.into(),
                        id: id.into(),
                        field: key.into(),
                    };
                    if matches!(field["kind"].as_str(), Some("rich" | "rich_inline")) {
                        if let Ok(node) = serde_json::from_value::<Node>(record[key].clone()) {
                            walk(&node, &mut vec![], &root, &mut out)
                        }
                    } else if let Some(text) = record[key].as_str() {
                        out.push(Zone {
                            zone: TextZone::Record {
                                collection: name.into(),
                                id: id.into(),
                                field: key.into(),
                                path: vec![],
                            },
                            length: text.encode_utf16().count() as u32,
                            text: text.into(),
                            supports_styles: false,
                        });
                    }
                }
            }
        }
    }
    out
}
pub fn text(state: &State, target: Option<&TextTarget>) -> Result<String, Error> {
    let all = zones(state);
    if let Some(target) = target {
        let zone = all
            .iter()
            .find(|z| z.zone == target.zone)
            .ok_or_else(|| targets::invalid(crate::content::contract::TargetReason::MissingNode))?;
        if target.from > target.to {
            return Err(targets::invalid(
                crate::content::contract::TargetReason::ReversedRange,
            ));
        }
        let from = targets::byte_at(&zone.text, target.from)?;
        let to = targets::byte_at(&zone.text, target.to)?;
        Ok(zone.text[from..to].into())
    } else {
        Ok(all
            .iter()
            .filter(|z| matches!(z.zone, TextZone::Body { .. }))
            .map(|z| z.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
/// Case-insensitive matching maps expanded Unicode lowercase back to UTF-16.
/// Every result stays in one zone; no match can cross an atom or a cell boundary.
pub fn find(state: &State, query: &str, case_sensitive: bool) -> Vec<Value> {
    if query.is_empty() {
        return vec![];
    }
    let needle = if case_sensitive {
        query.to_owned()
    } else {
        query.to_lowercase()
    };
    let mut found = vec![];
    for zone in zones(state) {
        let mut hay = String::new();
        let mut boundaries = std::collections::BTreeMap::new();
        let mut offset = 0;
        boundaries.insert(0, 0);
        for ch in zone.text.chars() {
            if case_sensitive {
                hay.push(ch)
            } else {
                hay.extend(ch.to_lowercase())
            }
            offset += ch.len_utf16() as u32;
            boundaries.insert(hay.len(), offset);
        }
        for (start, matched) in hay.match_indices(&needle) {
            if let (Some(&from), Some(&to)) = (
                boundaries.get(&start),
                boundaries.get(&(start + matched.len())),
            ) && let (Ok(a), Ok(b)) = (
                targets::byte_at(&zone.text, from),
                targets::byte_at(&zone.text, to),
            ) {
                let original = &zone.text[a..b];
                if !original.contains('\u{fffc}') {
                    found.push(json!({"text":original,"target":TextTarget {zone:zone.zone.clone(),from,to}}));
                }
            }
        }
    }
    found
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selector {
    select: Option<String>,
    occurrence: Option<usize>,
    block: Option<usize>,
}
pub fn resolve_call(state: &State, call: &Call) -> Result<Command, Error> {
    let mut params = call
        .params
        .as_object()
        .cloned()
        .ok_or_else(|| invalid("params"))?;
    if let Some(selector) = params.remove("selector") {
        if params.contains_key("target") {
            return Err(invalid("selector_target_conflict"));
        }
        let selector: Selector =
            serde_json::from_value(selector).map_err(|_| invalid("selector"))?;
        let target = if let Some(number) = selector.block {
            if selector.select.is_some()
                || selector.occurrence.is_some()
                || number == 0
                || number > state.document.content.len()
            {
                return Err(invalid("selector.block"));
            }
            if call.id.starts_with("text.")
                || call.id == "block.split"
                || call.id == "list.item.split"
                || call.id == "inline.replace"
            {
                let zone = TextZone::Body {
                    path: vec![number - 1],
                };
                let all = zones(state);
                let z = all
                    .iter()
                    .find(|z| z.zone == zone)
                    .ok_or_else(|| invalid("selector.block"))?;
                json!(TextTarget {
                    zone,
                    from: 0,
                    to: z.length
                })
            } else {
                json!(TreeTarget {
                    root: TreeRoot::Body,
                    path: vec![number - 1]
                })
            }
        } else {
            let query = selector.select.ok_or_else(|| invalid("selector.select"))?;
            let matches = find(state, &query, true);
            if matches.len() > 1 && selector.occurrence.is_none() {
                return Err(invalid("selector.ambiguous"));
            }
            let i = selector
                .occurrence
                .unwrap_or(1)
                .checked_sub(1)
                .ok_or_else(|| invalid("selector.occurrence"))?;
            matches
                .get(i)
                .ok_or_else(|| invalid("selector.not_found"))?["target"]
                .clone()
        };
        params.insert("target".into(), target);
    }
    serde_json::from_value(json!({"id":call.id,"params":params})).map_err(|_| invalid(&call.id))
}
pub fn run(
    state: &State,
    assets: &crate::assets::Assets,
    id: &str,
    p: &Value,
) -> Result<Value, Error> {
    let target: Option<TextTarget> = p
        .get("target")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| invalid("target"))?;
    Ok(match id {
        "document.validate" => {
            let valid = state.validate(assets);
            json!({"valid":valid.is_ok(),"ready_for_export":valid.is_ok()&&state.settings.metadata.ready_for_export(),"diagnostics":valid.err().map(|e|json!(e)).into_iter().collect::<Vec<_>>()})
        }
        "spelling.suggestions" => {
            json!({"suggestions":crate::spelling::suggestions(serde_json::from_value(p["language"].clone()).map_err(|_|invalid("language"))?,p["word"].as_str().ok_or_else(||invalid("word"))?).map_err(|e|Error::InvalidDocument{reason:e.to_string()})?})
        }
        "document.structure" => {
            json!({"document":state.document,"zones":zones(state),"collections":state.settings.metadata.document_type().structured_metadata()})
        }
        "document.text" => json!({"text":text(state,target.as_ref())?}),
        "document.source" => json!({"source":state.validate(assets)?}),
        "document.statistics" => {
            json!({"document":crate::statistics::TextStatistics::count(&text(state,None)?),"selection":target.as_ref().map(|t|text(state,Some(t)).map(|s|crate::statistics::TextStatistics::count(&s))).transpose()?})
        }
        "search.find" => {
            json!({"matches":find(state,p["query"].as_str().ok_or_else(||invalid("query"))?,p["case_sensitive"].as_bool().unwrap_or(false))})
        }
        "metadata.get" => {
            json!({"metadata":state.settings.metadata,"fields":state.settings.metadata.field_values(),"structured_metadata":state.settings.metadata.document_type().structured_metadata()})
        }
        "models.list" => {
            json!({"models":super::type_description(state.settings.metadata.document_type())["models"],"active":{"id":state.settings.model_id,"embedded":!state.settings.metadata.document_type().models().any(|m|m.model.id()==state.settings.model_id&&m.source==state.settings.template)}})
        }
        "resources.list" => {
            json!({"resources":assets.iter().map(|(path,data)|json!({"path":path,"bytes":data.len()})).collect::<Vec<_>>()})
        }
        "spelling.issues" => {
            let language = state.settings.metadata.options().language;
            let mut issues = vec![];
            let mut available = true;
            for zone in zones(state) {
                let language = zone_language(state, &zone.zone);
                if !crate::spelling::supported(language) {
                    available = false;
                    continue;
                }
                let accepted = crate::spelling::accepted(
                    language,
                    &Default::default(),
                    &state.settings.metadata.options().spelling_ignored,
                );
                for issue in
                    crate::spelling::check_text(language, &zone.text, &accepted).map_err(|e| {
                        Error::InvalidDocument {
                            reason: e.to_string(),
                        }
                    })?
                {
                    let from = zone
                        .text
                        .chars()
                        .take(issue.range.start)
                        .map(|c| c.len_utf16() as u32)
                        .sum();
                    let to = zone
                        .text
                        .chars()
                        .take(issue.range.end)
                        .map(|c| c.len_utf16() as u32)
                        .sum();
                    issues.push(json!({"word":issue.word,"language":language,"target":TextTarget{zone:zone.zone.clone(),from,to}}));
                }
            }
            json!({"available":available,"language":language,"issues":issues})
        }
        "bibliography.search" => {
            let crate::document_types::Metadata::Article(metadata) = &state.settings.metadata
            else {
                return Err(invalid("bibliography_unavailable"));
            };
            let entries = crate::bibliography::index(&metadata.references)
                .map_err(|_| invalid("bibliography"))?;
            json!({"entries":crate::bibliography::search(&entries,p["query"].as_str().ok_or_else(||invalid("query"))?)})
        }
        _ => return Err(invalid("query")),
    })
}

fn zone_language(state: &State, zone: &TextZone) -> crate::document::DocumentLanguage {
    if let crate::document_types::Metadata::Article(article) = &state.settings.metadata
        && let TextZone::Record { collection, id, .. } = zone
        && collection == "localizations"
        && let Some(locale) = article.localizations.iter().find(|l| l.id == *id)
    {
        return locale.language;
    }
    state.settings.metadata.options().language
}
