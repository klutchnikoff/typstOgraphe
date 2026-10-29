use crate::{
    Report, calls,
    script_format::{self, Document, Save},
};
use serde_json::json;
use std::path::PathBuf;
use typstographe_core::{
    content::api::{self, Call, Session},
    engine::CommandError,
};
use typstographe_i18n::tr;
pub fn execute(path: PathBuf) -> Result<Report, CommandError> {
    let (script, base) = script_format::load(&path)?;
    let mut s = match script.document {
        Document::Open { mut path } => {
            crate::paths::relative_to(&mut path, &base);
            if matches!(script.save, Some(Save::Current {})) {
                calls::open(path, true)?
            } else {
                calls::open(path, false)?.fork()?
            }
        }
        Document::New {
            document_type,
            model,
            params,
        } => {
            let mut s = Session::new();
            let mut p = params;
            p.insert("type".into(), json!(document_type));
            if let Some(m) = model {
                p.insert("model".into(), json!(m));
            }
            s.execute("document.new", json!(p))?;
            s
        }
    };
    let save = match script.save {
        Some(Save::Current {}) => Some(api::Save::Current),
        Some(Save::As { mut path }) => {
            crate::paths::relative_to(&mut path, &base);
            Some(api::Save::As { path })
        }
        None => None,
    };
    let steps = script
        .steps
        .iter()
        .map(|step| {
            Ok(Call {
                id: step.id.clone(),
                params: calls::params(step.params.clone(), step.target.clone())?,
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    let outcome = s.batch(&steps, save)?;
    let saved = outcome["saved"] == true;
    let path = if saved {
        json!(s.file_path())
    } else {
        serde_json::Value::Null
    };
    Ok(Report {
        text: format!(
            "{}\n{}\n",
            tr!("cli-script-completed", steps = steps.len()),
            if saved {
                tr!("letter-saved", path = path.as_str().unwrap_or(""))
            } else {
                tr!("cli-not-saved")
            }
        ),
        outcome: json!({"batch":outcome,"saved":saved,"path":path,"document":s.query("document.inspect",json!({}))?,"source":s.query("document.source",json!({}))?["source"],"metadata":s.state()?.settings.metadata}),
        exit: 0,
    })
}
