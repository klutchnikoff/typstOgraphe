use crate::*;
use serde_json::{Value, json};

fn id(value: &str) -> RecordId {
    RecordId::new(value).unwrap()
}

fn record(identity: &str, key: &str, entry: Value) -> Record {
    serde_json::from_value(json!({"id":identity,"key":key,"entry":entry})).unwrap()
}

fn import(text: &str, format: ImportFormat) -> PreparedImport {
    let source = ImportSource::new(id("source"), "example", format, text.into()).unwrap();
    let mut count = 0;
    prepare_import(source, |_| {
        count += 1;
        id(&format!("import-{count}"))
    })
    .unwrap()
}

mod corpus;
mod imports;
mod native;
mod plans;
mod records;

mod form;

mod classification;
