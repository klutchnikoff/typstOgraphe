use serde_json::Value;
use typstographe_i18n::{LocalizedLabel, tr};

pub fn catalogue(value: &Value) -> String {
    if let Some(entries) = value["entries"].as_array() {
        entries
            .iter()
            .map(|s| {
                format!(
                    "{}\t{}\n",
                    s["id"].as_str().unwrap(),
                    s["description"].as_str().unwrap()
                )
            })
            .collect()
    } else {
        format!(
            "{}\n{}\n\n{}\n",
            value["id"].as_str().unwrap(),
            value["description"].as_str().unwrap(),
            serde_json::to_string_pretty(&value["params_schema"]).expect("schema")
        )
    }
}

pub fn inspect(value: &Value) -> String {
    let kind: typstographe_core::document_types::DocumentType =
        serde_json::from_value(value["document"]["document_type"].clone()).expect("document type");
    let stats: typstographe_core::engine::StatisticsResult =
        serde_json::from_value(value["statistics"].clone()).expect("statistics");
    let mut text = format!(
        "{}\n{}\n",
        kind.label(),
        tr!(
            "document-stats",
            words = stats.document.words,
            signs = stats.document.signs
        )
    );
    if let Some(fields) = value["metadata"]["fields"].as_array() {
        for field in fields {
            text.push_str(&format!(
                "{}: {}\n",
                field["field"].as_str().unwrap_or("?"),
                field["value"].as_str().unwrap_or("")
            ));
        }
    }
    text
}

pub fn check(value: &Value) -> String {
    let mut text = format!(
        "{}\n",
        if value["passed"] == true {
            tr!("cli-check-passed")
        } else {
            tr!("cli-check-failed")
        }
    );
    let validation: typstographe_core::engine::ValidationResult =
        serde_json::from_value(value["validation"].clone()).expect("validation");
    for diagnostic in validation.diagnostics {
        let detail = diagnostic
            .cause
            .as_ref()
            .map(typstographe_i18n::error)
            .or(diagnostic.detail)
            .unwrap_or_else(|| format!("{:?}", diagnostic.field));
        text.push_str(&format!("{}: {detail}\n", diagnostic.code));
    }
    if !value["spelling"].is_null() {
        if value["spelling"]["available"] == false {
            text.push_str(&format!("{}\n", tr!("cli-dictionary-unavailable")));
        }
        if let Some(issues) = value["spelling"]["issues"].as_array() {
            for issue in issues {
                text.push_str(&format!(
                    "{}\n",
                    tr!(
                        "cli-spelling-issue",
                        word = issue["word"].as_str().unwrap_or("")
                    )
                ));
            }
        }
    }
    text
}
