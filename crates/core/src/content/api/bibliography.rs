//! Read-only preparation. These results are not active document records or edit commands.
use super::invalid;
use crate::content::contract::{Error, State, UnavailableReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use typstographe_bibliography::{
    self as bibliography, Diagnostic, Fingerprint, ImportFormat, Profile, Record, RecordId,
};

use crate::bibliography::editions::SourceInput as ImportInput;

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct RecordInput {
    pub id: RecordId,
    pub profile: Profile,
    /// Omit to suggest an available key; an explicit occupied key is refused.
    #[serde(default)]
    pub key: Option<String>,
    /// Native Hayagriva fields over the genre's starting structure, not a source document.
    #[serde(default)]
    pub entry: Map<String, Value>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RecordIdentity {
    id: RecordId,
}
#[derive(Serialize, JsonSchema)]
struct SourceSummary {
    id: RecordId,
    name: String,
    format: ImportFormat,
    digest: Fingerprint,
}

#[derive(Serialize, JsonSchema)]
struct ImportResult {
    /// False: preparation never inserts references, publishes edits or saves a project.
    #[schemars(extend("const"=false))]
    active: bool,
    library_fingerprint: Fingerprint,
    plan: Value,
    source: SourceSummary,
    records: Vec<Record>,
    summaries: Vec<crate::bibliography::Entry>,
    diagnostics: Vec<Diagnostic>,
    fingerprint: Fingerprint,
}

#[derive(Serialize, JsonSchema)]
struct RecordResult {
    #[schemars(extend("const"=false))]
    active: bool,
    record: Record,
    profile: Profile,
    diagnostics: Vec<Diagnostic>,
    fingerprint: Fingerprint,
}

#[derive(Serialize, JsonSchema)]
struct FormResult {
    #[schemars(extend("const"=false))]
    active: bool,
    record: Record,
    profile: Profile,
    profiles: Vec<Profile>,
    fields: Vec<bibliography::form::FormField>,
    diagnostics: Vec<Diagnostic>,
    patches: Vec<bibliography::FieldPatch>,
    fingerprint: Fingerprint,
}

#[derive(Serialize, JsonSchema)]
struct ProfilesResult {
    profiles: Vec<bibliography::ProfileDescriptor>,
}

pub(super) fn parameter_schema(id: &str) -> Option<Value> {
    Some(match id {
        "bibliography.record" | "bibliography.usage" => {
            json!(schemars::schema_for!(RecordIdentity))
        }
        "bibliography.form" | "bibliography.prepare_form" => {
            json!(schemars::schema_for!(FormInput))
        }
        "bibliography.review_import" => json!(schemars::schema_for!(ReviewInput)),
        "bibliography.browse" => {
            json!({"type":"object","properties":{"query":{"type":"string"}},"additionalProperties":false})
        }
        "bibliography.prepare_import" => json!(schemars::schema_for!(ImportInput)),
        "bibliography.prepare_record" => json!(schemars::schema_for!(RecordInput)),
        _ => return None,
    })
}
pub(super) fn result_schema(id: &str) -> Option<Value> {
    Some(match id {
        "bibliography.profiles" => json!(schemars::schema_for!(ProfilesResult)),
        "bibliography.form" | "bibliography.prepare_form" => {
            json!(schemars::schema_for!(FormResult))
        }
        "bibliography.prepare_import" => json!(schemars::schema_for!(ImportResult)),
        "bibliography.prepare_record" => json!(schemars::schema_for!(RecordResult)),
        _ => return None,
    })
}

fn occupied(state: &State, command: &str) -> Result<std::collections::BTreeSet<String>, Error> {
    let crate::document_types::Metadata::Article(meta) = &state.settings.metadata else {
        return Err(Error::CommandUnavailable {
            command: command.into(),
            reason: UnavailableReason::UnsupportedDocumentType,
        });
    };
    crate::bibliography::index(&meta.references)
        .map(|v| v.into_iter().map(|e| e.key).collect())
        .map_err(|reason| Error::InvalidDocument { reason })
}

pub(super) fn run(state: &State, command: &str, params: &Value) -> Result<Value, Error> {
    let occupied = occupied(state, command)?;
    Ok(match command {
        "bibliography.record" => {
            let identity: RecordIdentity = serde_json::from_value(params.clone())
                .map_err(|_| invalid("bibliography_record"))?;
            let library = crate::bibliography::editions::library(state, command)?;
            let record = library.get(&identity.id).ok_or_else(|| {
                Error::bibliography(bibliography::Error {
                    code: bibliography::Code::MissingRecord,
                    key: None,
                    field: None,
                    detail: "Record not found".into(),
                })
            })?;
            json!({"record":record,"profile":record.profile(),"fingerprint":record.fingerprint(),"diagnostics":bibliography::completeness(record)})
        }
        "bibliography.profiles" => json!(ProfilesResult {
            profiles: bibliography::profiles()
        }),
        "bibliography.prepare_import" => {
            let input: ImportInput = serde_json::from_value(params.clone())
                .map_err(|_| invalid("bibliography_import"))?;
            let prepared = crate::bibliography::editions::prepare(input)?;
            let library = crate::bibliography::editions::library(state, command)?;
            let plan = crate::bibliography::editions::plan(&library, &prepared)?;
            json!(ImportResult {
                active: false,
                library_fingerprint: library.fingerprint(),
                plan: json!({"blocked":plan.blocked(),"items":plan.items()}),
                source: SourceSummary {
                    id: prepared.source.id().clone(),
                    name: prepared.source.name().into(),
                    format: prepared.source.format(),
                    digest: prepared.source.digest().clone()
                },
                fingerprint: prepared.fingerprint(),
                summaries: crate::bibliography::index(&prepared.records)
                    .map_err(|reason| Error::InvalidDocument { reason })?,
                records: prepared.records,
                diagnostics: prepared.diagnostics
            })
        }
        "bibliography.prepare_record" => {
            let input: RecordInput = serde_json::from_value(params.clone())
                .map_err(|_| invalid("bibliography_record"))?;
            // Use the crate's strict native decoder, preserving field paths on errors.
            let seed_key = input.key.as_deref().unwrap_or("pending");
            let entry = input.profile.empty_entry(seed_key);
            let mut value = serde_json::to_value(entry).expect("native entry");
            value.as_object_mut().unwrap().extend(input.entry);
            let pending =
                Record::from_native(input.id, seed_key, value).map_err(Error::bibliography)?;
            let key = input
                .key
                .unwrap_or_else(|| bibliography::suggest_key(pending.entry(), &occupied));
            if occupied.contains(&key) {
                return Err(Error::bibliography(bibliography::Error {
                    code: bibliography::Code::DuplicateKey,
                    key: Some(key),
                    field: Some("key".into()),
                    detail: "The requested key is already used by the project".into(),
                }));
            }
            let record = pending
                .renamed(&pending.fingerprint(), &key)
                .map_err(Error::bibliography)?;
            json!(RecordResult {
                active: false,
                fingerprint: record.fingerprint(),
                diagnostics: bibliography::completeness(&record),
                profile: record.profile(),
                record
            })
        }
        _ => return Err(invalid("query")),
    })
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FormInput {
    record: Record,
    #[serde(default)]
    values: std::collections::BTreeMap<String, Value>,
    key: Option<String>,
    /// A transient native classification change, never a stored form genre.
    profile: Option<Profile>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ReviewInput {
    source: crate::bibliography::editions::SourceInput,
    expected: Fingerprint,
    decisions: Vec<bibliography::Decision>,
}
pub(super) fn manager(state: &State, command: &str, params: &Value) -> Result<Value, Error> {
    let library = crate::bibliography::editions::library(state, command)?;
    match command {
        "bibliography.review_import" => {
            let input: ReviewInput = serde_json::from_value(params.clone())
                .map_err(|_| invalid("bibliography_import"))?;
            if library.fingerprint() != input.expected {
                return Err(Error::bibliography(bibliography::Error {
                    code: bibliography::Code::StaleBase,
                    key: None,
                    field: None,
                    detail: "Library changed since import preparation".into(),
                }));
            }
            let prepared = crate::bibliography::editions::prepare(input.source)?;
            let plan = crate::bibliography::editions::plan(&library, &prepared)?
                .resolve(&input.decisions)
                .map_err(Error::bibliography)?;
            Ok(json!({"plan":{"blocked":plan.blocked(),"items":plan.items()}}))
        }
        "bibliography.form" | "bibliography.prepare_form" => {
            let input: FormInput =
                serde_json::from_value(params.clone()).map_err(|_| invalid("bibliography_form"))?;
            let patches = bibliography::form::form_patches(&input.record, &input.values)
                .map_err(Error::bibliography)?;
            let mut record = input
                .record
                .edited(&input.record.fingerprint(), &patches)
                .map_err(Error::bibliography)?;
            if let Some(profile) = input.profile {
                record = record
                    .reclassified(&record.fingerprint(), profile)
                    .map_err(Error::bibliography)?;
            }
            let keys = library
                .records()
                .iter()
                .filter(|r| r.id() != record.id())
                .map(|r| r.key().to_string())
                .collect();
            let key = input.key.unwrap_or_else(|| {
                if library.get(record.id()).is_some() {
                    record.key().into()
                } else {
                    bibliography::suggest_key(record.entry(), &keys)
                }
            });
            if keys.contains(&key) {
                return Err(Error::bibliography(bibliography::Error {
                    code: bibliography::Code::DuplicateKey,
                    key: Some(key),
                    field: Some("key".into()),
                    detail: "Citation key is occupied".into(),
                }));
            }
            let record = record
                .renamed(&record.fingerprint(), &key)
                .map_err(Error::bibliography)?;
            Ok(json!(FormResult {
                active: false,
                profile: record.profile(),
                profiles: bibliography::profiles()
                    .into_iter()
                    .map(|p| p.profile)
                    .collect(),
                fields: bibliography::form::form_fields(&record),
                diagnostics: bibliography::completeness(&record),
                patches,
                fingerprint: record.fingerprint(),
                record
            }))
        }
        "bibliography.browse" => {
            let query = params["query"].as_str().unwrap_or("");
            let ids = bibliography::search(library.records(), query, bibliography::MAX_RECORDS);
            Ok(
                json!({"fingerprint":library.fingerprint(),"total":library.records().len(),"entries":ids.iter().map(|id|{
                let record=library.get(id).unwrap();let summary=bibliography::summarize(record.entry());
                json!({"id":id,"key":record.key(),"title":summary.title,"names":summary.names,"authors":summary.authors,"year":summary.year,"profile":record.profile(),"incomplete":!bibliography::completeness(record).is_empty()})
            }).collect::<Vec<_>>() }),
            )
        }
        "bibliography.usage" => {
            let identity: RecordIdentity =
                serde_json::from_value(params.clone()).map_err(|_| invalid("id"))?;
            if library.get(&identity.id).is_none() {
                return Err(invalid("id"));
            }
            let mut targets = vec![];
            fn visit(
                node: &crate::content::Node,
                root: &crate::content::commands::TreeRoot,
                path: &mut Vec<usize>,
                id: &RecordId,
                out: &mut Vec<crate::content::commands::TreeTarget>,
            ) {
                if node.kind == "citation"
                    && node
                        .attrs
                        .as_ref()
                        .and_then(|v| v.get("items"))
                        .and_then(Value::as_array)
                        .is_some_and(|items| items.iter().any(|i| i["record_id"] == json!(id)))
                {
                    out.push(crate::content::commands::TreeTarget {
                        root: root.clone(),
                        path: path.clone(),
                    });
                }
                for (i, child) in node.content.iter().enumerate() {
                    path.push(i);
                    visit(child, root, path, id, out);
                    path.pop();
                }
            }
            visit(
                &state.document,
                &crate::content::commands::TreeRoot::Body,
                &mut vec![],
                &identity.id,
                &mut targets,
            );
            Ok(json!({"targets":targets}))
        }
        _ => Err(invalid("query")),
    }
}
