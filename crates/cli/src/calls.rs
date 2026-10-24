//! L’adaptateur résout le contexte ; le moteur valide et exécute les appels typés.
use crate::{
    Report,
    args::{Parameters, Target},
    open, targets,
};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use typstographe_core::engine::*;
use typstographe_i18n::tr;

fn specification(id: &str, query: bool) -> Result<&'static CommandSpec, CommandError> {
    let spec = catalogue()
        .iter()
        .find(|s| s.id.as_str() == id)
        .ok_or_else(|| CommandError::new(ErrorCode::NotFound, "unknown_command"))?;
    if (spec.family == Family::Query) != query {
        return Err(CommandError::new(
            ErrorCode::InvalidParams,
            "cli_call_family",
        ));
    }
    if id.starts_with("recovery.")
        || matches!(spec.family, Family::File | Family::Batch | Family::History)
    {
        return Err(CommandError::new(
            ErrorCode::Unavailable,
            "cli_call_unavailable",
        ));
    }
    Ok(spec)
}

fn parameters(
    engine: &mut Engine,
    spec: &CommandSpec,
    input: Parameters,
    target: Target,
) -> Result<Value, CommandError> {
    let mut params = input.load()?;
    targets::portable(&Value::Object(params.clone()))?;
    target.bind(engine, spec, &mut params)?;
    if !spec.params_schema["properties"]["guard"].is_null() {
        params.insert("guard".into(), json!(engine.target_guard()));
    }
    prepare_render(engine, spec, &mut params)?;
    Ok(Value::Object(params))
}

fn prepare_render(
    engine: &Engine,
    spec: &CommandSpec,
    params: &mut Map<String, Value>,
) -> Result<(), CommandError> {
    if spec.params_schema["properties"]["render"].is_null() {
        return Ok(());
    }
    let compiled = engine.compile(GuardParams {
        guard: engine.target_guard(),
    })?;
    let render = compiled.render.ok_or_else(|| {
        compiled
            .diagnostics
            .into_iter()
            .find_map(|d| d.cause)
            .map(|cause| CommandError::with_cause(ErrorCode::Template, "render_compile", cause))
            .unwrap_or_else(|| CommandError::new(ErrorCode::Template, "render_unavailable"))
    })?;
    params.insert("render".into(), json!(render.reference));
    Ok(())
}

pub fn query(
    id: String,
    file: Option<PathBuf>,
    params: Parameters,
    target: Target,
) -> Result<Report, CommandError> {
    let spec = specification(&id, true)?;
    let mut engine = match file {
        Some(file) => open(file)?,
        None => Engine::empty(),
    };
    let params = parameters(&mut engine, spec, params, target)?;
    let query = Query::from_call(&id, params)?;
    if !engine.is_open()
        && spec.availability != AvailabilityRule::Always
        && !matches!(
            query,
            Query::SpellingSuggestions(SpellingSuggestionsParams {
                target: SuggestionTarget::Word { .. }
            })
        )
    {
        return Err(CommandError::new(ErrorCode::Unavailable, "no_document"));
    }
    let outcome = engine.query(query)?;
    Ok(Report {
        text: format!(
            "{}\n",
            serde_json::to_string_pretty(&outcome).expect("query result")
        ),
        outcome,
        exit: 0,
    })
}

pub fn run(
    id: String,
    file: PathBuf,
    params: Parameters,
    target: Target,
    save: bool,
    output: Option<PathBuf>,
) -> Result<Report, CommandError> {
    let spec = specification(&id, false)?;
    // --save réserve le fichier AVANT sa lecture. Une copie/prévisualisation reste indépendante.
    let mut engine = if save {
        let mut engine = Engine::empty();
        engine.run(Command::OpenDocument(OpenDocumentParams {
            guard: engine.target_guard(),
            path: file.clone(),
            read_only: false,
            discard_changes: false,
        }))?;
        engine
    } else {
        Engine::from_stored(open(file.clone())?.stored_document(), None)?
    };
    let params = parameters(&mut engine, spec, params, target)?;
    let command = engine.run_call(Call {
        id: spec.id.clone(),
        params,
    })?;
    let path = if save {
        engine.run(Command::SaveDocument(SaveDocumentParams {
            guard: engine.target_guard(),
            path: None,
        }))?;
        Some(file)
    } else if let Some(output) = output {
        engine.run(Command::SaveAs(SaveAsParams {
            guard: engine.target_guard(),
            path: output.clone(),
            overwrite: false,
        }))?;
        Some(output)
    } else {
        None
    };
    let saved = path.is_some();
    Ok(Report {
        outcome: json!({"command":command,"saved":saved,"path":path,"document":engine.query(Query::Inspect(Empty {}))?,"source":engine.document().source(),"metadata":engine.metadata()}),
        text: format!(
            "{}\n{}\n",
            tr!("cli-applied", id = id),
            if let Some(path) = path {
                tr!("letter-saved", path = path.display().to_string())
            } else {
                tr!("cli-not-saved")
            }
        ),
        exit: 0,
    })
}
