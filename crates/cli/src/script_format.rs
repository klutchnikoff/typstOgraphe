//! Enveloppe CLI versionnée. Les étapes conservent les appels du registre.
use crate::{args::Target, parameters::read_payload, targets};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use typstographe_core::engine::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    pub version: u32,
    pub document: Document,
    pub steps: Vec<Step>,
    #[serde(default)]
    pub save: Option<Save>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Document {
    Open {
        path: PathBuf,
    },
    New {
        #[serde(rename = "type")]
        document_type: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        params: Map<String, Value>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: String,
    #[serde(default)]
    pub params: Map<String, Value>,
    #[serde(default)]
    pub target: Target,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Save {
    Current {},
    As { path: PathBuf },
}

pub fn load(path: &Path) -> Result<(Script, PathBuf), CommandError> {
    let (bytes, base) = if path == Path::new("-") {
        (
            read_payload(
                std::io::stdin().lock(),
                "cli_script_io",
                "cli_script_too_large",
            )?,
            std::env::current_dir().map_err(io_error)?,
        )
    } else {
        let path = path.canonicalize().map_err(io_error)?;
        (
            read_payload(
                File::open(&path).map_err(io_error)?,
                "cli_script_io",
                "cli_script_too_large",
            )?,
            path.parent().expect("absolute script parent").to_owned(),
        )
    };
    let script: Script = serde_json::from_slice(&bytes).map_err(CommandError::invalid)?;
    script.validate()?;
    Ok((script, base))
}
fn io_error(error: std::io::Error) -> CommandError {
    CommandError::detail(ErrorCode::Io, "cli_script_io", error)
}

impl Script {
    fn validate(&self) -> Result<(), CommandError> {
        if self.version != 1 {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_script_version",
            ));
        }
        if self.steps.is_empty()
            || self.steps.len() > typstographe_core::content::api::MAX_BATCH_STEPS
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "batch_step_limit",
            ));
        }
        if matches!(self.document, Document::New { .. })
            && matches!(self.save, Some(Save::Current {}))
        {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_script_save",
            ));
        }
        if let Document::New { params, .. } = &self.document {
            targets::portable(&Value::Object(params.clone()))?;
            if params.contains_key("type") || params.contains_key("model") {
                return Err(CommandError::new(
                    ErrorCode::InvalidParams,
                    "cli_params_conflict",
                ));
            }
        }
        for (index, step) in self.steps.iter().enumerate() {
            let check = || {
                let spec = typstographe_core::content::api::describe(&step.id)
                    .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
                if spec["batch_step"] != true {
                    return Err(CommandError::new(
                        ErrorCode::Unavailable,
                        "batch_step_forbidden",
                    ));
                }
                targets::portable(&Value::Object(step.params.clone()))?;
                step.target.validate()
            };
            check().map_err(|e| e.at_batch_step(index, &step.id))?;
        }
        Ok(())
    }
}
