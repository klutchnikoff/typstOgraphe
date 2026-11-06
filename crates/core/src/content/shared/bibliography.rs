//! Publish changed native records only, preserving other shared branches and identities.
use crate::content::contract::{Error, State};
use typstographe_bibliography::Fingerprint;
use yrs::{Any, Array, Doc, Map, MapPrelim, Out, Transact};
pub(super) fn check_generic(state: &State, name: &str) -> Result<(), Error> {
    if state
        .settings
        .metadata
        .document_type()
        .definition()
        .definition()
        .collections
        .iter()
        .any(|c| {
            c.id == name
                && c.kind != crate::document_types::definition::CollectionKind::OrderedRecords
        })
    {
        return Err(Error::InvalidParameters {
            parameter: "bibliography.apply_required".into(),
        });
    }
    Ok(())
}
pub(super) fn apply(
    doc: &Doc,
    state: &State,
    expected: &Fingerprint,
    change: &crate::bibliography::editions::Change,
) -> Result<(), Error> {
    let after = crate::bibliography::editions::apply(state, expected, change)?;
    let before = crate::bibliography::editions::metadata(state, "bibliography.apply")?;
    let before = serde_json::to_value(before).unwrap();
    let after = serde_json::to_value(after).unwrap();
    let settings = doc.get_or_insert_map("settings");
    let schema = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
        .unwrap();
    let mut tx = doc.transact_mut();
    let Some(Out::YMap(collections)) = settings.get(&tx, "collections") else {
        return Err(super::projection::invalid("collections_absent"));
    };
    for name in ["references", "reference_sources"] {
        let Some(Out::YMap(records)) = collections.get(&tx, name) else {
            return Err(super::projection::invalid("bibliography_absent"));
        };
        let Some(Out::YArray(order)) = settings.get(&tx, &format!("order:{name}")) else {
            return Err(super::projection::invalid("bibliography_order_absent"));
        };
        let old = before[name].as_array().unwrap();
        let new = after[name].as_array().unwrap();
        let definition = schema["collections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["parameter"] == name)
            .unwrap();
        for record in old
            .iter()
            .filter(|r| !new.iter().any(|n| n["id"] == r["id"]))
        {
            let id = record["id"].as_str().unwrap();
            records.remove(&mut tx, id);
            for i in (0..order.len(&tx)).rev() {
                if matches!(order.get(&tx,i),Some(Out::Any(Any::String(key))) if key.as_ref()==id) {
                    order.remove(&mut tx, i);
                }
            }
        }
        for value in new {
            let id = value["id"].as_str().unwrap();
            if old.iter().any(|v| v == value) {
                continue;
            }
            if let Some(Out::YMap(record)) = records.get(&tx, id) {
                for (key, value) in value.as_object().unwrap() {
                    if key == "id" {
                        continue;
                    }
                    let data = Any::from_json(&value.to_string())
                        .map_err(|e| super::projection::invalid(e.to_string()))?;
                    record.insert(&mut tx, key.as_str(), data);
                }
            } else {
                let record = records.insert(&mut tx, id, MapPrelim::default());
                super::seed::fill_record(record, &mut tx, definition, value, id)?;
                let index = order.len(&tx);
                order.insert(&mut tx, index, id);
            }
        }
    }
    Ok(())
}
