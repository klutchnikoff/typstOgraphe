//! Options, signature and model changes preserve all existing shared branches.
use super::{projection, records};
use crate::{
    assets::Assets,
    content::{
        commands::{Command, OptionName},
        contract::{Error, State},
    },
};
use serde_json::{Value, json};
use yrs::{Any, Doc, Map, Transact};
fn parameter(name: &str) -> Error {
    Error::InvalidParameters {
        parameter: name.into(),
    }
}
fn any(value: &Value) -> Any {
    Any::from_json(&value.to_string()).expect("JSON")
}
fn set(doc: &Doc, key: &str, value: &Value) {
    let root = doc.get_or_insert_map("settings");
    let mut tx = doc.transact_mut();
    root.insert(&mut tx, key, any(value));
}
pub(super) fn set_option(
    doc: &Doc,
    state: &State,
    name: OptionName,
    value: &Value,
) -> Result<(), Error> {
    let mut options = serde_json::to_value(state.settings.metadata.options()).expect("options");
    let key = name.key();
    if options[key] == *value {
        return Ok(());
    }
    options[key] = value.clone();
    let options: crate::document::DocumentOptions =
        serde_json::from_value(options).map_err(|_| parameter("value"))?;
    if options
        .bullet_marker
        .as_ref()
        .is_some_and(|v| !crate::document::valid_bullet_marker(v))
    {
        return Err(parameter("value"));
    }
    set(doc, &format!("options.{key}"), value);
    if name == OptionName::Language {
        // Changing the primary language adds missing localized records, never replaces existing ones.
        if let Some(schema) = state
            .settings
            .metadata
            .document_type()
            .structured_metadata()
        {
            let fields = serde_json::to_value(&state.settings.metadata).expect("metadata");
            for definition in schema["collections"].as_array().into_iter().flatten() {
                let Some(key) = definition["language_key"].as_str() else {
                    continue;
                };
                let name = definition["parameter"].as_str().unwrap();
                let items = fields["fields"][name].as_array().unwrap();
                if items.iter().any(|item| item[key] == *value) {
                    continue;
                }
                let mut index = 1;
                let id = loop {
                    let id = format!("primary-{}-{index}", options.language.code());
                    if !items.iter().any(|item| item["id"] == id) {
                        break id;
                    }
                    index += 1;
                };
                records::apply(
                    doc,
                    state,
                    &Command::AddRecord {
                        collection: name.into(),
                        id,
                        values: serde_json::Map::from_iter([(key.into(), value.clone())]),
                        after: None,
                    },
                )?;
            }
        }
    }
    Ok(())
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    match command {
        Command::SetSignature { path } => {
            if state.settings.metadata.document_type()
                != crate::document_types::DocumentType::Courrier
            {
                return Err(parameter("signature"));
            }
            if path.as_ref().is_some_and(|p| !crate::assets::valid_path(p)) {
                return Err(parameter("path"));
            }
            if state.settings.metadata.signature_path() == path.as_deref() {
                return Ok(());
            }
            set(doc, "signature", &json!(path));
        }
        Command::SetModel { model_id, source } => {
            if model_id.trim().is_empty() || model_id.len() > 256 || source.len() > 1024 * 1024 {
                return Err(parameter("model"));
            }
            let builtin = crate::document_types::models::ALL
                .iter()
                .find(|m| m.model.id() == model_id);
            if builtin.is_some_and(|m| m.kind != state.settings.metadata.document_type()) {
                return Err(parameter("model"));
            }
            if state.settings.model_id == *model_id && state.settings.template == *source {
                return Ok(());
            }
            // Update the scalar base only: collections, rich fragments and overrides retain identity.
            let root = doc.get_or_insert_map("settings");
            let value = root
                .get(&doc.transact(), "document")
                .ok_or_else(|| parameter("model"))?;
            let yrs::Out::Any(value) = value else {
                return Err(parameter("model"));
            };
            let mut base = serde_json::to_value(value).map_err(|_| parameter("model"))?;
            base["modelId"] = json!(model_id);
            base["template"] = json!(source);
            set(doc, "document", &base);
            if let Some((paper, landscape)) =
                builtin.filter(|m| m.source == source).and_then(|m| m.page)
            {
                set(doc, "options.paper", &json!(paper));
                set(doc, "options.landscape", &json!(landscape));
            }
        }
        _ => unreachable!(),
    }
    Ok(())
}
pub(super) fn validate_model(state: &State, assets: &Assets) -> Result<(), Error> {
    let template = &state.settings.template;
    let dependencies = crate::packages::for_template(template);
    crate::packages::validate(template, &dependencies).map_err(|e| Error::InvalidSettings {
        reason: e.to_string(),
    })?;
    let source = state.validate(assets)?;
    let mut world = crate::rendering::TypstWorld::default();
    world.set_assets(assets);
    world.set_dependencies(&dependencies);
    world
        .validate_model(&source, &state.settings.metadata, template)
        .map_err(|e| projection::invalid(format!("model_validation: {e}")))?;
    Ok(())
}
