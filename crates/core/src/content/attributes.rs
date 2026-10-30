//! Supported node attributes, derived once from the public capability contract.
use super::{Node, catalogue};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

pub(super) fn validate(node: &Node, internal_mapping: bool) -> Result<(), String> {
    let Some(value) = &node.attrs else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let attrs = value
        .as_object()
        .ok_or_else(|| format!("invalid_node_attributes:{}", node.kind))?;
    static DECLARED: OnceLock<HashMap<&'static str, HashSet<String>>> = OnceLock::new();
    let declared = DECLARED.get_or_init(|| {
        let mut map: HashMap<&str, HashSet<String>> = HashMap::new();
        for capability in catalogue::ALL {
            let Some(kind) = capability.node else {
                continue;
            };
            let schema = catalogue::parameters(capability.id);
            if let Some(properties) = schema["properties"].as_object() {
                for (name, field) in properties {
                    let hint = &field["x-editor"];
                    let external =
                        matches!(hint["placement"].as_str(), Some("structure" | "creation"))
                            || (hint["placement"] == "body"
                                && !matches!(kind, "footnote" | "citation"));
                    if !external {
                        map.entry(kind)
                            .or_default()
                            .insert(hint["attribute"].as_str().unwrap_or(name).into());
                    }
                }
            }
        }
        // Structural attributes are edited through dedicated commands, not panels.
        for (kind, names) in [
            ("heading", &["level"][..]),
            (
                "tableCell",
                &["colspan", "rowspan", "colwidth", "align"][..],
            ),
            (
                "tableHeader",
                &["colspan", "rowspan", "colwidth", "align", "scope"][..],
            ),
        ] {
            map.entry(kind)
                .or_default()
                .extend(names.iter().map(|name| (*name).into()));
        }
        map
    });
    for key in attrs.keys() {
        if internal_mapping
            && key == "_mapping"
            && matches!(node.kind.as_str(), "text" | "footnote" | "citation")
        {
            continue;
        }
        if !declared
            .get(node.kind.as_str())
            .is_some_and(|names| names.contains(key))
        {
            return Err(format!("unsupported_node_attribute:{}:{key}", node.kind));
        }
    }
    Ok(())
}
