//! Canonical encodings for native values whose shorthand is not roundtrip-safe.
use hayagriva::Entry;
use serde_json::{Value, json};

const FORMATTED: &[&str] = &[
    "title",
    "location",
    "organization",
    "archive",
    "archive-location",
    "call-number",
    "note",
    "abstract",
    "genre",
];

/// Paths of unsupported members, shared by strict record decoding and import review.
pub(crate) fn unsupported_fields(value: &Value) -> Vec<String> {
    fn members(value: &Value, path: &str, allowed: &[&str], out: &mut Vec<String>) {
        if let Some(map) = value.as_object() {
            for key in map.keys().filter(|k| !allowed.contains(&k.as_str())) {
                out.push(format!("{path}/{key}"));
            }
        }
    }
    fn format(value: &Value, path: &str, out: &mut Vec<String>) {
        members(value, path, &["value", "short", "verbatim"], out);
        for key in ["value", "short"] {
            if let Some(value) = value.get(key) {
                members(value, &format!("{path}/{key}"), &["value", "verbatim"], out);
            }
        }
    }
    fn many(
        value: &Value,
        path: &str,
        out: &mut Vec<String>,
        visit: fn(&Value, &str, &mut Vec<String>),
    ) {
        if let Some(values) = value.as_array() {
            for (i, value) in values.iter().enumerate() {
                visit(value, &format!("{path}/{i}"), out);
            }
        } else {
            visit(value, &format!("{path}/0"), out);
        }
    }
    fn person(value: &Value, path: &str, out: &mut Vec<String>) {
        members(
            value,
            path,
            &["name", "given-name", "prefix", "suffix", "alias"],
            out,
        );
    }
    fn affiliated(value: &Value, path: &str, out: &mut Vec<String>) {
        members(value, path, &["role", "names"], out);
        if let Some(names) = value.get("names") {
            many(names, &format!("{path}/names"), out, person);
        }
    }
    fn entry(value: &Value, path: &str, out: &mut Vec<String>) {
        let Some(map) = value.as_object() else {
            return;
        };
        for (key, value) in map {
            let path = format!("{path}/{key}");
            if !crate::import::native_field(key) {
                out.push(path);
                continue;
            }
            if FORMATTED.contains(&key.as_str()) {
                format(value, &path, out);
            }
            match key.as_str() {
                "author" | "editor" => many(value, &path, out, person),
                "affiliated" => many(value, &path, out, affiliated),
                "parent" => many(value, &path, out, entry),
                "publisher" => {
                    members(value, &path, &["name", "location"], out);
                    for key in ["name", "location"] {
                        if let Some(value) = value.get(key) {
                            format(value, &format!("{path}/{key}"), out);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut result = vec![];
    entry(value, "", &mut result);
    result
}

/// FormatString's outer map contains a ChunkedString in `value` and optional `short`.
/// In 0.10.1 the outer-map parser ignores a sibling `verbatim` in this input
/// shorthand. Serialization expresses literal segments with protected braces.
/// Keep the literal flag on the nested ChunkedString, where Hayagriva reads it.
fn formatted(value: &mut Value) {
    if let Some(object) = value.as_object_mut()
        && let Some(verbatim) = object.remove("verbatim")
    {
        let long = object.remove("value").unwrap_or(Value::Null);
        object.insert("value".into(), json!({"value":long,"verbatim":verbatim}));
    }
}

pub(crate) fn normalize_entry(value: &mut Value) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    for &key in FORMATTED {
        if let Some(value) = object.get_mut(key) {
            formatted(value);
        }
    }
    if let Some(publisher) = object.get_mut("publisher").and_then(Value::as_object_mut) {
        for field in ["name", "location"] {
            if let Some(value) = publisher.get_mut(field) {
                formatted(value);
            }
        }
    }
    if let Some(parent) = object.get_mut("parent") {
        if parent.is_object() {
            *parent = Value::Array(vec![parent.take()]);
        }
        if let Some(parents) = parent.as_array_mut() {
            parents.iter_mut().for_each(normalize_entry);
        }
    }
}

pub(crate) fn canonical_entry(entry: &Entry) -> Value {
    let mut value = serde_json::to_value(entry).expect("native entry");
    normalize_entry(&mut value);
    value
}
