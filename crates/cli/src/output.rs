use serde_json::Value;
use typstographe_i18n::{LocalizedLabel, tr};
pub fn catalogue(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap() + "\n"
}
pub fn inspect(v: &Value) -> String {
    let kind: typstographe_core::document_types::DocumentType =
        serde_json::from_value(v["document"]["document_type"].clone()).unwrap();
    format!(
        "{}\n{}\n",
        kind.label(),
        tr!(
            "document-stats",
            words = v["statistics"]["document"]["words"].as_u64().unwrap_or(0),
            signs = v["statistics"]["document"]["signs"].as_u64().unwrap_or(0)
        )
    )
}
pub fn check(v: &Value) -> String {
    let mut text = format!(
        "{}\n",
        if v["passed"] == true {
            tr!("cli-check-passed")
        } else {
            tr!("cli-check-failed")
        }
    );
    for d in v["validation"]["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Ok(error) = serde_json::from_value(d.clone()) {
            text.push_str(&typstographe_i18n::content_error(&error));
            text.push('\n');
        }
    }
    if v["spelling"]["available"] == false {
        text.push_str(&tr!("cli-dictionary-unavailable"));
        text.push('\n');
    }
    for issue in v["spelling"]["issues"].as_array().into_iter().flatten() {
        text.push_str(&tr!(
            "cli-spelling-issue",
            word = issue["word"].as_str().unwrap_or("")
        ));
        text.push('\n');
    }
    text
}
