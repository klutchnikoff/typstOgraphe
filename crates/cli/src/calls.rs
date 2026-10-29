use crate::{
    Report,
    args::{Parameters, Target},
};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use typstographe_core::{
    content::api::{self, Session},
    engine::{CommandError, ErrorCode},
};
use typstographe_i18n::tr;
pub fn params(mut params: Map<String, Value>, target: Target) -> Result<Value, CommandError> {
    crate::targets::portable(&json!(params))?;
    if target.select.is_some() || target.block.is_some() {
        if params.contains_key("target") || params.contains_key("selector") {
            return Err(CommandError::new(
                ErrorCode::InvalidParams,
                "cli_params_conflict",
            ));
        }
        let mut selector = Map::new();
        if let Some(s) = target.select {
            selector.insert("select".into(), json!(s));
        }
        if let Some(n) = target.block {
            selector.insert("block".into(), json!(n));
        }
        if let Some(n) = target.occurrence {
            selector.insert("occurrence".into(), json!(n));
        }
        params.insert("selector".into(), json!(selector));
    }
    Ok(json!(params))
}
pub fn open(path: PathBuf, writable: bool) -> Result<Session, CommandError> {
    let mut s = Session::new();
    s.execute("document.open", json!({"path":path,"read_only":!writable}))?;
    Ok(s)
}
pub fn query(
    id: String,
    file: Option<PathBuf>,
    input: Parameters,
    target: Target,
) -> Result<Report, CommandError> {
    if !api::is_query(&id) {
        return Err(CommandError::new(
            ErrorCode::InvalidParams,
            "cli_call_family",
        ));
    }
    let mut s = match file {
        Some(p) => open(p, false)?,
        None => Session::new(),
    };
    let mut p = params(input.load()?, target)?;
    // Query selection conveniences resolve through the same typed zone search.
    if let Some(selector) = p.as_object_mut().unwrap().remove("selector") {
        let call = api::Call {
            id: "text.delete".into(),
            params: json!({"selector":selector}),
        };
        let command = api::resolve_call(s.state()?, &call).map_err(api::content_error)?;
        let typstographe_core::content::commands::Command::Delete { target } = command else {
            unreachable!()
        };
        p["target"] = json!(target);
    }
    let outcome = s.query(&id, p)?;
    Ok(Report {
        text: format!("{}\n", serde_json::to_string_pretty(&outcome).unwrap()),
        outcome,
        exit: 0,
    })
}
pub fn run(
    id: String,
    file: PathBuf,
    input: Parameters,
    target: Target,
    save: bool,
    output: Option<PathBuf>,
) -> Result<Report, CommandError> {
    if api::is_query(&id) || api::describe(&id).is_none_or(|s| s["family"] != "edit") {
        return Err(CommandError::new(
            ErrorCode::Unavailable,
            "cli_call_unavailable",
        ));
    }
    let mut s = if save {
        open(file.clone(), true)?
    } else {
        open(file.clone(), false)?.fork()?
    };
    let command = s.execute(&id, params(input.load()?, target)?)?;
    let path = if save {
        s.save()?;
        Some(file)
    } else if let Some(path) = output {
        s.save_as(&path)?;
        Some(path)
    } else {
        None
    };
    let saved = path.is_some();
    Ok(Report {
        outcome: json!({"command":command,"saved":saved,"path":path,"document":s.query("document.inspect",json!({}))?,"source":s.query("document.source",json!({}))?["source"],"metadata":s.state()?.settings.metadata}),
        text: format!(
            "{}\n{}\n",
            tr!("cli-applied", id = id),
            path.map(|p| tr!("letter-saved", path = p.display().to_string()))
                .unwrap_or_else(|| tr!("cli-not-saved"))
        ),
        exit: 0,
    })
}
