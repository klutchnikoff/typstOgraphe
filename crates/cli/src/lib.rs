//! Adaptateur synchrone : aucune logique documentaire ni préférence graphique.
pub mod args;
mod creation;
mod output;
mod parameters;
use args::{Action, CatalogueAction, Cli};
use serde_json::{Value, json};
use typstographe_core::engine::*;
use typstographe_i18n::tr;

pub struct Report {
    pub outcome: Value,
    pub text: String,
    pub exit: u8,
}

pub fn execute(cli: Cli) -> Result<Report, CommandError> {
    match cli.action {
        Action::New {
            document_type,
            model,
            output,
            params,
        } => creation::create(document_type, model, output, params),
        Action::Commands { action } => catalogue_report(action),
        Action::Inspect { file } => {
            let engine = open(file)?;
            let outcome = json!({
                "document":engine.query(Query::Inspect(Empty {}))?,
                "metadata":engine.query(Query::Metadata(Empty {}))?,
                "structure":engine.query(Query::Structure(Empty {}))?,
                "statistics":engine.query(Query::Statistics(StatisticsParams::default()))?,
            });
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
            let engine = open(file)?;
            let validation = engine.validate(GuardParams {
                guard: engine.target_guard(),
            })?;
            let issues: Option<SpellingIssues> = spelling
                .then(|| {
                    engine
                        .query(Query::SpellingIssues(Empty {}))
                        .map(|value| serde_json::from_value(value).expect("spelling issues"))
                })
                .transpose()?;
            let passed = validation.valid
                && (!for_export || validation.ready_for_export)
                && issues
                    .as_ref()
                    .is_none_or(|s| s.available && s.issues.is_empty());
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
            let mut engine = open(file)?;
            let outcome = engine.run(Command::Export(ExportParams {
                guard: engine.target_guard(),
                directory: output.clone(),
                format: format.into(),
                page_images: pages,
            }))?;
            Ok(Report {
                outcome: serde_json::to_value(outcome).expect("serializable outcome"),
                text: format!(
                    "{}\n",
                    tr!("cli-exported", path = output.display().to_string())
                ),
                exit: 0,
            })
        }
    }
}

fn open(file: std::path::PathBuf) -> Result<Engine, CommandError> {
    let mut engine = Engine::empty();
    engine.run(Command::OpenDocument(OpenDocumentParams {
        guard: engine.target_guard(),
        path: file,
        read_only: true,
        discard_changes: false,
    }))?;
    Ok(engine)
}

fn catalogue_report(action: CatalogueAction) -> Result<Report, CommandError> {
    let outcome = match action {
        CatalogueAction::List { markdown: true, .. } => {
            return Ok(Report {
                outcome: Value::Null,
                text: documentation(),
                exit: 0,
            });
        }
        CatalogueAction::List { query, .. } => {
            let entries: Vec<_> = catalogue()
                .iter()
                .filter(|s| !query || s.family == Family::Query)
                .collect();
            json!({"entries": entries})
        }
        CatalogueAction::Describe { id } => {
            let spec = catalogue()
                .iter()
                .find(|s| s.id.as_str() == id)
                .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
            serde_json::to_value(spec).expect("serializable specification")
        }
    };
    Ok(Report {
        text: output::catalogue(&outcome),
        outcome,
        exit: 0,
    })
}

pub fn documentation() -> String {
    let mut text = String::from(include_str!("reference.md"));
    for spec in catalogue() {
        let family = serde_json::to_value(spec.family).expect("family");
        text.push_str(&format!(
            "| `{}` | {} | {} |\n",
            spec.id.as_str(),
            family.as_str().unwrap(),
            spec.description.replace('|', "\\|")
        ));
    }
    text
}
