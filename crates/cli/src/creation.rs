use crate::{Report, args::Parameters};
use serde_json::json;
use std::path::PathBuf;
use typstographe_core::{
    content::api::Session,
    protocol::{CommandError, ErrorCode},
};
use typstographe_i18n::tr;
pub fn create(
    kind: String,
    model: Option<String>,
    output: PathBuf,
    params: Parameters,
) -> Result<Report, CommandError> {
    let mut s = Session::new();
    let mut p = params.load()?;
    crate::targets::portable(&json!(p))?;
    if p.contains_key("type") || p.contains_key("model") {
        return Err(CommandError::new(
            ErrorCode::InvalidParams,
            "cli_params_conflict",
        ));
    }
    p.insert("type".into(), json!(kind));
    if let Some(model) = model {
        p.insert("model".into(), json!(model));
    }
    let creation = s.execute("document.new", json!(p))?;
    let saved = s.save_as(&output)?;
    Ok(Report {
        outcome: json!({"created":creation,"saved":true,"path":output,"save":saved,"document":s.query("document.inspect",json!({}))?}),
        text: format!(
            "{}\n",
            tr!("cli-created", path = output.display().to_string())
        ),
        exit: 0,
    })
}
