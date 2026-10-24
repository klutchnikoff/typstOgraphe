//! Création via le catalogue et les commandes du moteur, sans profil implicite.
use crate::{Report, args::Parameters};
use serde_json::{Value, json};
use std::path::PathBuf;
use typstographe_core::engine::*;
use typstographe_i18n::tr;

pub fn create(
    document_type: String,
    model: Option<String>,
    output: PathBuf,
    params: Parameters,
) -> Result<Report, CommandError> {
    let mut engine = Engine::empty();
    let types = engine.query(Query::DocumentTypes(Empty {}))?;
    let entry = types["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|kind| kind["id"] == document_type)
        .ok_or_else(|| CommandError::new(ErrorCode::InvalidParams, "cli_document_type"))?;
    let models = entry["models"].as_array().expect("models");
    let model = match model {
        Some(id) => id,
        None if models.len() == 1 => models[0]["id"].as_str().expect("model ID").into(),
        None if models.is_empty() => {
            return Err(CommandError::new(ErrorCode::Unavailable, "creation_model"));
        }
        None => {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_model_required",
            ));
        }
    };
    let mut params = params.load()?;
    crate::targets::portable(&Value::Object(params.clone()))?;
    for (key, value) in [("type", entry["id"].clone()), ("model", json!(model))] {
        if params.insert(key.into(), value).is_some() {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_params_conflict",
            ));
        }
    }
    params
        .entry("guard")
        .or_insert_with(|| json!(engine.target_guard()));
    let creation = engine.run(Command::from_call("document.new", Value::Object(params))?)?;
    let saved = engine.run(Command::SaveAs(SaveAsParams {
        guard: engine.target_guard(),
        path: output.clone(),
        overwrite: false,
    }))?;
    Ok(Report {
        outcome: json!({"created":creation,"saved":true,"path":output,"save":saved,"document":engine.query(Query::Inspect(Empty {}))?}),
        text: format!(
            "{}\n",
            tr!("cli-created", path = output.display().to_string())
        ),
        exit: 0,
    })
}
