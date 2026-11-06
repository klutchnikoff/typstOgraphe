use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Retain the transitive closure of referenced definitions, including recursion.
/// This changes neither accepted inputs nor the schemas' reference identities.
pub fn prune_schema(schema: &mut Value) {
    match schema {
        Value::Array(items) => {
            for item in items {
                prune_schema(item);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                prune_schema(value);
            }
            let Some(Value::Object(definitions)) = map.remove("$defs") else {
                return;
            };
            fn references(value: &Value, found: &mut BTreeSet<String>) {
                match value {
                    Value::Array(items) => {
                        for item in items {
                            references(item, found);
                        }
                    }
                    Value::Object(map) => {
                        if let Some(name) = map
                            .get("$ref")
                            .and_then(Value::as_str)
                            .and_then(|reference| reference.strip_prefix("#/$defs/"))
                        {
                            found.insert(
                                name.split('/')
                                    .next()
                                    .unwrap()
                                    .replace("~1", "/")
                                    .replace("~0", "~"),
                            );
                        }
                        for (key, value) in map {
                            if key != "$defs" {
                                references(value, found);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let mut needed = BTreeSet::new();
            for value in map.values() {
                references(value, &mut needed);
            }
            let mut retained = BTreeMap::new();
            while let Some(name) = needed.pop_first() {
                if retained.contains_key(&name) {
                    continue;
                }
                if let Some(definition) = definitions.get(&name) {
                    references(definition, &mut needed);
                    retained.insert(name, definition.clone());
                }
            }
            if !retained.is_empty() {
                map.insert("$defs".into(), json!(retained));
            }
        }
        _ => {}
    }
}
