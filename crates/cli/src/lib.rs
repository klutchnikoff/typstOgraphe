//! CLI adapter: presentation and portable path resolution over the common structured session.
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
pub mod args;
mod calls;
mod creation;
pub use typstographe_mcp as mcp;
mod output;
mod parameters;
mod paths;
mod render;
mod script;
mod script_format;
use args::{Action, CatalogueAction, Cli};
use serde_json::{Value, json};
use typstographe_core::{
    content::api::{self},
    engine::{CommandError, ErrorCode},
};
use typstographe_i18n::tr;
use typstographe_mcp::targets;
pub struct Report {
    pub outcome: Value,
    pub text: String,
    pub exit: u8,
}
pub fn execute(cli: Cli) -> Result<Report, CommandError> {
    match cli.action {
        Action::Mcp { .. } => Err(CommandError::new(
            ErrorCode::Unavailable,
            "mcp_stdio_required",
        )),
        Action::Render { file, page, png } => render::execute(file, page, png),
        Action::Script { file } => script::execute(file),
        Action::Query {
            id,
            file,
            params,
            target,
        } => calls::query(id, file, params, target),
        Action::Run {
            id,
            file,
            params,
            target,
            save,
            output,
        } => calls::run(id, file, params, target, save, output),
        Action::New {
            document_type,
            model,
            output,
            params,
        } => creation::create(document_type, model, output, params),
        Action::Commands { action } => catalogue_report(action),
        Action::Inspect { file } => {
            let mut s = calls::open(file, false)?;
            let outcome = json!({"document":s.query("document.inspect",json!({}))?,"metadata":s.query("metadata.get",json!({}))?,"structure":s.query("document.structure",json!({}))?,"statistics":s.query("document.statistics",json!({}))?});
            Ok(Report {
                text: output::inspect(&outcome),
                outcome,
                exit: 0,
            })
        }
        Action::Check {
            file,
            for_export,
            spelling,
        } => {
            let mut s = calls::open(file, false)?;
            let validation = s.query("document.validate", json!({}))?;
            let issues = if spelling {
                s.query("spelling.issues", json!({}))?
            } else {
                Value::Null
            };
            let passed = validation["valid"] == true
                && (!for_export || validation["ready_for_export"] == true)
                && (!spelling
                    || issues["available"] == true
                        && issues["issues"].as_array().is_some_and(|v| v.is_empty()));
            let outcome = json!({"passed":passed,"for_export":for_export,"validation":validation,"spelling":issues});
            Ok(Report {
                text: output::check(&outcome),
                outcome,
                exit: u8::from(!passed),
            })
        }
        Action::Export {
            file,
            format,
            output,
            pages,
        } => {
            let mut s = calls::open(file, false)?;
            let outcome = s.export(&output, format.into(), pages)?;
            Ok(Report {
                outcome,
                text: format!(
                    "{}\n",
                    tr!("cli-exported", path = output.display().to_string())
                ),
                exit: 0,
            })
        }
    }
}
fn catalogue_report(action: CatalogueAction) -> Result<Report, CommandError> {
    let outcome = match action {
        CatalogueAction::Export { markdown: true }
        | CatalogueAction::List { markdown: true, .. } => {
            return Ok(Report {
                outcome: Value::Null,
                text: documentation(),
                exit: 0,
            });
        }
        CatalogueAction::Export { markdown: false } => api::catalogue(),
        CatalogueAction::List { query, .. } => {
            let mut c = api::catalogue();
            if query {
                c["entries"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|s| s["family"] == "query");
            }
            c
        }
        CatalogueAction::Describe { id } => api::describe(&id)
            .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?,
    };
    Ok(Report {
        text: output::catalogue(&outcome),
        outcome,
        exit: 0,
    })
}
pub fn documentation() -> String {
    let mut text = include_str!("reference.md").replace("\r\n", "\n");
    for spec in api::catalogue()["entries"].as_array().unwrap() {
        text.push_str(&format!(
            "| `{}` | {} | {} |\n",
            spec["id"].as_str().unwrap(),
            spec["family"].as_str().unwrap(),
            spec["description"].as_str().unwrap().replace('|', "\\|")
        ));
    }
    text
}
