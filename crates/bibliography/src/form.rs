//! Native form projection and guarded-field patches; unexposed data stays in the record.
use crate::{Code, Error, FieldKind, FieldPatch, Record, Result};
use hayagriva::types::{ChunkKind, ChunkedString, FormatString, Person, StringChunk};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct FormField {
    pub path: String,
    pub kind: FieldKind,
    pub group: String,
    pub recommended: bool,
    pub value: Value,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonValue {
    id: String,
    name: String,
    given: String,
}
fn error(path: &str, detail: &str) -> Error {
    Error {
        code: Code::InvalidField,
        key: None,
        field: Some(path.into()),
        detail: detail.into(),
    }
}
fn people(value: &Value) -> Result<Vec<Person>> {
    let list = if value.is_array() {
        value.clone()
    } else if value.is_null() {
        json!([])
    } else {
        json!([value])
    };
    serde_json::from_value(list).map_err(|_| error("/author", "Invalid native people"))
}
fn plain(value: &Value, kind: FieldKind) -> Value {
    if matches!(kind, FieldKind::People) {
        return json!(
            people(value)
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(i, p)| PersonValue {
                    id: format!("person-{i}"),
                    name: p.name.clone(),
                    given: p.given_name.clone().unwrap_or_default()
                })
                .collect::<Vec<_>>()
        );
    }
    if matches!(kind, FieldKind::FormattedText)
        && !value.is_null()
        && let Ok(text) = serde_json::from_value::<FormatString>(value.clone())
    {
        return json!(
            text.value
                .0
                .iter()
                .map(|c| c.value.as_str())
                .collect::<String>()
        );
    }
    json!(match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}
fn find_field(value: &Value, path: &str, key: &str) -> Option<String> {
    if value.get(key).is_some() {
        return Some(format!("{path}/{key}"));
    }
    value
        .get("parent")?
        .as_array()?
        .iter()
        .enumerate()
        .find_map(|(i, v)| find_field(v, &format!("{path}/parent/{i}"), key))
}
fn publication(value: &Value, path: &str, genre: crate::Profile) -> Option<String> {
    let preferred = match genre {
        crate::Profile::BookChapter => &["book", "anthology"][..],
        crate::Profile::ConferencePaper => &["proceedings"][..],
        _ => &["periodical", "blog"][..],
    };
    let parents = value.get("parent")?.as_array()?;
    for (i, parent) in parents.iter().enumerate() {
        let path = format!("{path}/parent/{i}");
        if parent["type"]
            .as_str()
            .is_some_and(|t| preferred.contains(&t))
        {
            return Some(path);
        }
        if let Some(path) = publication(parent, &path, genre) {
            return Some(path);
        }
    }
    None
}
pub fn form_fields(record: &Record) -> Vec<FormField> {
    let native = record.native_value();
    let mut result = vec![];
    for field in record.profile().descriptor().fields {
        let mut path = field.path;
        if let Some(key) = path.strip_prefix("/parent/0/") {
            if let Some(parent) = publication(&native, "", record.profile()) {
                path = format!("{parent}/{key}");
            }
        } else if matches!(
            path.as_str(),
            "/author"
                | "/date"
                | "/publisher"
                | "/editor"
                | "/organization"
                | "/volume"
                | "/issue"
                | "/edition"
        ) && let Some(actual) = find_field(&native, "", &path[1..])
        {
            path = actual;
        }
        let expansions: Vec<(String, FieldKind)> = match field.kind {
            FieldKind::Publisher => vec![
                (format!("{path}/name"), FieldKind::FormattedText),
                (format!("{path}/location"), FieldKind::FormattedText),
            ],
            FieldKind::Identifiers => ["doi", "isbn", "issn", "arxiv"]
                .iter()
                .map(|s| (format!("{path}/{s}"), FieldKind::NumberOrText))
                .collect(),
            FieldKind::Url => vec![
                (format!("{path}/value"), FieldKind::NumberOrText),
                (format!("{path}/date"), FieldKind::Date),
            ],
            kind => vec![(path.clone(), kind)],
        };
        for (path, kind) in expansions {
            let mut value = native.pointer(&path).cloned().unwrap_or(Value::Null);
            if path.ends_with("/publisher/name") && value.is_null() {
                value = native
                    .pointer(path.trim_end_matches("/name"))
                    .filter(|v| v.is_string())
                    .cloned()
                    .unwrap_or(Value::Null);
            }
            if path == "/url/value" && value.is_null() {
                value = native
                    .get("url")
                    .filter(|v| v.is_string())
                    .cloned()
                    .unwrap_or(Value::Null);
            }
            let recommended =
                field.recommended && !path.ends_with("/location") && path != "/url/date";
            result.push(FormField {
                path,
                kind,
                group: field.group.into(),
                recommended,
                value: plain(&value, kind),
            });
        }
    }
    result
}
fn text_edit(old: Option<FormatString>, next: &str) -> FormatString {
    let mut old = old.unwrap_or_default();
    let chars: Vec<_> = old
        .value
        .0
        .iter()
        .flat_map(|c| c.value.chars().map(move |v| (v, c.kind)))
        .collect();
    let new: Vec<_> = next.chars().collect();
    let mut prefix = 0;
    let mut suffix = 0;
    while prefix < chars.len().min(new.len()) && chars[prefix].0 == new[prefix] {
        prefix += 1;
    }
    while suffix < chars.len() - prefix
        && suffix < new.len() - prefix
        && chars[chars.len() - 1 - suffix].0 == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let kind = if !chars.is_empty() && chars.iter().all(|(_, k)| *k == ChunkKind::Verbatim) {
        ChunkKind::Verbatim
    } else {
        ChunkKind::Normal
    };
    let values = chars[..prefix]
        .iter()
        .copied()
        .chain(new[prefix..new.len() - suffix].iter().map(|c| (*c, kind)))
        .chain(chars[chars.len() - suffix..].iter().copied());
    let mut chunks: Vec<StringChunk> = vec![];
    for (ch, kind) in values {
        if let Some(last) = chunks.last_mut().filter(|c| c.kind == kind) {
            last.value.push(ch)
        } else {
            chunks.push(StringChunk {
                value: ch.to_string(),
                kind,
            })
        }
    }
    old.value = ChunkedString(chunks);
    old
}
fn set(value: &mut Value, parts: &[&str], data: Option<Value>) -> Result<()> {
    let (key, rest) = parts.split_first().ok_or_else(|| error("", "Empty path"))?;
    if value.is_array() {
        let index: usize = key.parse().map_err(|_| error("", "Invalid index"))?;
        let item = value
            .as_array_mut()
            .unwrap()
            .get_mut(index)
            .ok_or_else(|| error("", "Missing parent"))?;
        if rest.is_empty() {
            *item = data.unwrap_or(Value::Null);
            Ok(())
        } else {
            set(item, rest, data)
        }
    } else {
        if !value.is_object() {
            *value = if value.is_null() {
                json!({})
            } else if *key == "value" || *key == "date" {
                json!({"value":value.clone()})
            } else {
                json!({"name":value.clone()})
            };
        }
        let map = value.as_object_mut().unwrap();
        if rest.is_empty() {
            if let Some(data) = data {
                map.insert((*key).into(), data);
            } else {
                map.remove(*key);
            }
            Ok(())
        } else {
            let child = map.entry((*key).to_string()).or_insert_with(|| json!({}));
            set(child, rest, data)
        }
    }
}
/// Only changed controls become root-field patches. Missing controls are untouched.
pub fn form_patches(record: &Record, values: &BTreeMap<String, Value>) -> Result<Vec<FieldPatch>> {
    let fields = form_fields(record);
    let before = record.native_value();
    let mut after = before.clone();
    let mut roots = BTreeSet::new();
    for (path, value) in values {
        let field = fields
            .iter()
            .find(|f| &f.path == path)
            .ok_or_else(|| error(path, "Unknown form field"))?;
        if &field.value == value {
            continue;
        }
        let mut original = before.pointer(path).cloned().unwrap_or(Value::Null);
        if path.ends_with("/publisher/name") && original.is_null() {
            original = before
                .pointer(path.trim_end_matches("/name"))
                .filter(|v| v.is_string())
                .cloned()
                .unwrap_or(Value::Null);
        }
        let data = if matches!(field.kind, FieldKind::People) {
            let rows: Vec<PersonValue> =
                serde_json::from_value(value.clone()).map_err(|_| error(path, "Invalid people"))?;
            let old = people(&original)?;
            let mut seen = BTreeSet::new();
            let mut people = vec![];
            for row in rows {
                if row.name.trim().is_empty() && row.given.trim().is_empty() {
                    continue;
                }
                if !seen.insert(row.id.clone()) || row.name.trim().is_empty() {
                    return Err(error(path, "Invalid or repeated person"));
                }
                let mut person = if let Some(index) = row
                    .id
                    .strip_prefix("person-")
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    old.get(index)
                        .cloned()
                        .ok_or_else(|| error(path, "Unknown person identity"))?
                } else {
                    serde_json::from_value::<Person>(json!({"name":row.name}))
                        .map_err(|_| error(path, "Invalid name"))?
                };
                person.name = row.name;
                person.given_name = (!row.given.is_empty()).then_some(row.given);
                people.push(person);
            }
            (!people.is_empty()).then(|| json!(people))
        } else {
            let text = value.as_str().ok_or_else(|| error(path, "Expected text"))?;
            if text.trim().is_empty() {
                None
            } else if matches!(field.kind, FieldKind::FormattedText) {
                Some(json!(text_edit(
                    serde_json::from_value(original).ok(),
                    text
                )))
            } else if matches!(field.kind, FieldKind::Date) {
                Some(
                    text.parse::<i32>()
                        .map_or_else(|_| json!(text), |v| json!(v)),
                )
            } else {
                Some(json!(text))
            }
        };
        let parts: Vec<_> = path.trim_start_matches('/').split('/').collect();
        let mut field_index = 0;
        while parts.get(field_index) == Some(&"parent") {
            field_index += 2;
        }
        roots.insert(format!("/{}", parts[..=field_index].join("/")));
        if parts[0] == "parent" && after.get("parent").is_none() {
            after["parent"] = json!([{}]);
        }
        set(&mut after, &parts, data)?;
    }
    for key in ["publisher", "url", "serial-number"] {
        if after
            .get(key)
            .and_then(Value::as_object)
            .is_some_and(|v| v.is_empty())
        {
            after.as_object_mut().unwrap().remove(key);
        }
    }
    Ok(roots
        .into_iter()
        .filter(|r| before.pointer(r) != after.pointer(r))
        .map(|path| FieldPatch {
            value: after.pointer(&path).cloned(),
            path,
        })
        .collect())
}
