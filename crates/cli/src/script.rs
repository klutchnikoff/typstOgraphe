//! Résolution CLI dans le brouillon atomique du moteur ; aucune sauvegarde d’étape.
use crate::{
    Report, calls, creation, paths,
    script_format::{self, Document, Save},
};
use serde_json::{Value, json};
use std::path::PathBuf;
use typstographe_core::engine::*;
use typstographe_i18n::tr;

pub fn execute(path: PathBuf) -> Result<Report, CommandError> {
    let (script, base) = script_format::load(&path)?;
    let mut engine = match script.document {
        Document::Open { mut path } => {
            paths::relative_to(&mut path, &base);
            calls::working_copy(path, matches!(script.save, Some(Save::Current {})))?
        }
        Document::New {
            document_type,
            model,
            params,
        } => {
            let mut engine = Engine::empty();
            let mut command = creation::command(&engine, &document_type, model.as_deref(), params)?;
            paths::command(&mut command, &base);
            engine.run(command)?;
            engine
        }
    };
    let save = match script.save {
        Some(Save::Current {}) => Some(BatchSave::Current {}),
        Some(Save::As { mut path }) => {
            paths::relative_to(&mut path, &base);
            Some(BatchSave::As {
                path,
                overwrite: false,
            })
        }
        None => None,
    };
    let steps = script
        .steps
        .iter()
        .map(|s| Call {
            id: s.id.clone(),
            params: Value::Object(s.params.clone()),
        })
        .collect();
    let outcome = engine.run_resolved_batch(
        BatchParams {
            guard: engine.target_guard(),
            steps,
            save,
        },
        |index, draft, call| {
            let spec = catalogue()
                .iter()
                .find(|s| s.id == call.id)
                .expect("preflighted step");
            let (params, selection) = calls::resolve_parameters(
                draft,
                spec,
                script.steps[index].params.clone(),
                script.steps[index].target.clone(),
            )?;
            let mut command = Command::from_call(call.id.as_str(), params)?;
            paths::command(&mut command, &base);
            Ok(ResolvedBatchStep { command, selection })
        },
    )?;
    let saved_path = outcome.value.as_ref().and_then(|v| v["saved"].as_str());
    let saved = saved_path.is_some();
    let text = format!(
        "{}\n{}\n",
        tr!("cli-script-completed", steps = script.steps.len()),
        if let Some(path) = saved_path {
            tr!("letter-saved", path = path)
        } else {
            tr!("cli-not-saved")
        }
    );
    Ok(Report {
        outcome: json!({"batch":outcome,"saved":saved,"path":saved_path,"document":engine.query(Query::Inspect(Empty {}))?,"source":engine.document().source(),"metadata":engine.metadata()}),
        text,
        exit: 0,
    })
}
