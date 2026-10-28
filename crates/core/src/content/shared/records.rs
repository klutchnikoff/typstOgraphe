//! Ordered records: stable public IDs, descriptor fields, no collection replacement.
use super::{seed, targets};
use crate::content::{
    commands::Command,
    contract::{Error, State, TargetReason},
};
use serde_json::Value;
use yrs::{Any, Array, ArrayRef, Doc, Map, MapPrelim, MapRef, Out, ReadTxn, Transact};

fn parameter(name: &str) -> Error {
    Error::InvalidParameters {
        parameter: name.into(),
    }
}
fn collection<T: ReadTxn>(
    tx: &T,
    state: &State,
    name: &str,
) -> Result<(MapRef, ArrayRef, Value), Error> {
    let schema = state
        .settings
        .metadata
        .document_type()
        .structured_metadata()
        .ok_or_else(|| targets::invalid(TargetReason::MissingCollection))?;
    let definition = schema["collections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["parameter"] == name)
        .ok_or_else(|| targets::invalid(TargetReason::MissingCollection))?
        .clone();
    let settings = tx
        .get_map("settings")
        .ok_or_else(|| targets::invalid(TargetReason::MissingCollection))?;
    let Some(Out::YMap(collections)) = settings.get(tx, "collections") else {
        return Err(targets::invalid(TargetReason::MissingCollection));
    };
    let Some(Out::YMap(records)) = collections.get(tx, name) else {
        return Err(targets::invalid(TargetReason::MissingCollection));
    };
    let Some(Out::YArray(order)) = settings.get(tx, &format!("order:{name}")) else {
        return Err(targets::invalid(TargetReason::MissingCollection));
    };
    Ok((records, order, definition))
}
fn ids<T: ReadTxn>(order: &ArrayRef, records: &MapRef, tx: &T) -> Vec<String> {
    let mut ids = Vec::new();
    for out in order.iter(tx) {
        if let Out::Any(Any::String(id)) = out
            && records.contains_key(tx, &id)
            && !ids.iter().any(|value| value == id.as_ref())
        {
            ids.push(id.to_string());
        }
    }
    ids
}
fn remove_order(order: &ArrayRef, tx: &mut yrs::TransactionMut, id: &str) {
    for index in (0..order.len(tx)).rev() {
        if matches!(order.get(tx, index), Some(Out::Any(Any::String(value))) if value.as_ref() == id)
        {
            order.remove(tx, index);
        }
    }
}
fn anchor<T: ReadTxn>(order: &ArrayRef, tx: &T, id: &str) -> Result<u32, Error> {
    order
        .iter(tx)
        .position(|out| matches!(out, Out::Any(Any::String(value)) if value.as_ref() == id))
        .map(|n| n as u32)
        .ok_or_else(|| targets::invalid(TargetReason::MissingRecord))
}
pub(super) fn apply(doc: &Doc, state: &State, command: &Command) -> Result<(), Error> {
    let (name, id) = match command {
        Command::AddRecord { collection, id, .. }
        | Command::RemoveRecord { collection, id }
        | Command::MoveRecord { collection, id, .. } => (collection, id),
        _ => unreachable!(),
    };
    let (records, order, definition) = collection(&doc.transact(), state, name)?;
    let visible = ids(&order, &records, &doc.transact());
    match command {
        Command::AddRecord { values, after, .. } => {
            if id.is_empty()
                || id.len() > 128
                || id.chars().any(char::is_control)
                || records.contains_key(&doc.transact(), id)
            {
                return Err(parameter("id"));
            }
            let fields = definition["fields"].as_array().unwrap();
            if values
                .keys()
                .any(|key| !fields.iter().any(|f| f["key"] == key.as_str()))
            {
                return Err(parameter("values"));
            }
            if after.as_ref().is_some_and(|id| !visible.contains(id)) {
                return Err(targets::invalid(TargetReason::MissingRecord));
            }
            // Do not allow malformed rich documents to be projected away by the XML writer.
            for field in fields {
                let key = field["key"].as_str().unwrap();
                let value = values.get(key).unwrap_or(&field["default"]);
                let valid = match field["kind"].as_str() {
                    Some("text" | "multiline" | "source" | "language") => value.is_string(),
                    Some("boolean") => value.is_boolean(),
                    Some("tokens" | "references") => value
                        .as_array()
                        .is_some_and(|v| v.iter().all(Value::is_string)),
                    Some("rich" | "rich_inline") => {
                        serde_json::from_value::<crate::content::editorial::RichText>(value.clone())
                            .is_ok()
                    }
                    _ => false,
                };
                if !valid {
                    return Err(parameter("values"));
                }
            }
            let mut tx = doc.transact_mut();
            // An orphaned order entry must not resurrect this ID at an old position.
            remove_order(&order, &mut tx, id);
            let index = match after {
                Some(id) => anchor(&order, &tx, id)? + 1,
                None => order.len(&tx),
            };
            let record = records.insert(&mut tx, id.as_str(), MapPrelim::default());
            seed::fill_record(
                record,
                &mut tx,
                &definition,
                &Value::Object(values.clone()),
                id,
            )?;
            order.insert(&mut tx, index, id.as_str());
        }
        Command::RemoveRecord { .. } => {
            if !visible.contains(id) {
                return Err(targets::invalid(TargetReason::MissingRecord));
            }
            // Associations are explicit edits. Final batch validation refuses dangling references.
            let mut tx = doc.transact_mut();
            remove_order(&order, &mut tx, id);
            records.remove(&mut tx, id);
        }
        Command::MoveRecord { before, .. } => {
            if !visible.contains(id) || before.as_ref().is_some_and(|key| !visible.contains(key)) {
                return Err(targets::invalid(TargetReason::MissingRecord));
            }
            if before.as_ref() == Some(id) {
                return Ok(());
            }
            let mut expected: Vec<_> = visible.iter().filter(|key| *key != id).cloned().collect();
            let index = before.as_ref().map_or(expected.len(), |key| {
                expected.iter().position(|k| k == key).unwrap()
            });
            expected.insert(index, id.clone());
            if expected == visible {
                return Ok(());
            }
            let mut tx = doc.transact_mut();
            remove_order(&order, &mut tx, id);
            let index = before
                .as_ref()
                .map_or_else(|| Ok(order.len(&tx)), |key| anchor(&order, &tx, key))?;
            order.insert(&mut tx, index, id.as_str());
        }
        _ => unreachable!(),
    }
    Ok(())
}
