//! Isolated protocol/representation probe, not an application document engine.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
    time::Instant,
};
use yrs::updates::{decoder::Decode, encoder::Encode};
use yrs::{
    Any, Array, Doc, GetString, Map, OffsetKind, Options, Out, ReadTxn, StateVector, Text,
    Transact, TransactionMut, Update, Xml, XmlElementPrelim, XmlFragment, XmlTextPrelim,
};

type Result<T> = std::result::Result<T, String>;
fn any(v: &Value) -> Any {
    Any::from_json(&v.to_string()).unwrap()
}
fn raw<T: ReadTxn>(v: Out, tx: &T) -> Value {
    match v {
        Out::Any(v) => json!({"kind":"json","value":v}),
        Out::YMap(v) => {
            json!({"kind":"map","entries":v.iter(tx).map(|(k,v)|(k.to_owned(),raw(v,tx))).collect::<BTreeMap<_,_>>()})
        }
        Out::YArray(v) => {
            json!({"kind":"array","values":v.iter(tx).map(|v|raw(v,tx)).collect::<Vec<_>>()})
        }
        Out::YText(v) => json!({"kind":"text","delta":v.diff(tx, |_|()).into_iter().map(|d|{
            let mut value=json!({"insert":plain(d.insert,tx)});
            if let Some(a)=d.attributes && !a.is_empty(){value["attributes"]=json!(a);}
            value
        }).collect::<Vec<_>>()}),
        Out::YXmlText(v) => json!({"kind":"xml-text","delta":v.diff(tx, |_|()).into_iter().map(|d|{
            let mut value=json!({"insert":plain(d.insert,tx)});
            if let Some(a)=d.attributes && !a.is_empty(){value["attributes"]=json!(a);}
            value
        }).collect::<Vec<_>>()}),
        Out::YXmlElement(v) => {
            json!({"kind":"xml-element","name":v.tag(),"attrs":v.attributes(tx).map(|(k,v)|(k.to_owned(),plain(v,tx))).collect::<BTreeMap<_,_>>(),"children":v.children(tx).map(|v|raw(v.into(),tx)).collect::<Vec<_>>()})
        }
        Out::YXmlFragment(v) => {
            json!({"kind":"xml-fragment","children":v.children(tx).map(|v|raw(v.into(),tx)).collect::<Vec<_>>()})
        }
        other => panic!("Unsupported shared type in this probe: {other:?}"),
    }
}
fn plain<T: ReadTxn>(v: Out, tx: &T) -> Value {
    match v {
        Out::Any(v) => serde_json::to_value(v).unwrap(),
        Out::YText(v) => json!(v.get_string(tx)),
        other => raw(other, tx),
    }
}
fn resolve<T: ReadTxn>(_doc: &Doc, tx: &T, path: &[Value]) -> Result<Out> {
    let name = path
        .first()
        .and_then(Value::as_str)
        .ok_or("root expected")?;
    let mut value = if name == "default" {
        Out::YXmlFragment(tx.get_xml_fragment(name).ok_or("root fragment absent")?)
    } else {
        Out::YMap(tx.get_map(name).ok_or("root map absent")?)
    };
    for key in &path[1..] {
        value = match value {
            Out::YMap(v) => v
                .get(tx, key.as_str().ok_or("map key expected")?)
                .ok_or("map value absent")?,
            Out::YArray(v) => v
                .get(tx, key.as_u64().ok_or("index expected")? as u32)
                .ok_or("array value absent")?,
            Out::YXmlElement(v) => v
                .get(tx, key.as_u64().ok_or("index expected")? as u32)
                .ok_or("xml value absent")?
                .into(),
            Out::YXmlFragment(v) => v
                .get(tx, key.as_u64().ok_or("index expected")? as u32)
                .ok_or("xml value absent")?
                .into(),
            _ => return Err("cannot descend into value".into()),
        }
    }
    Ok(value)
}
// Application-specific projection: Yrs supplies shared XML, not ProseMirror JSON.
fn pm_children<T: ReadTxn>(items: impl Iterator<Item = yrs::XmlOut>, tx: &T) -> Result<Vec<Value>> {
    let mut nodes = vec![];
    for item in items {
        match item {
            yrs::XmlOut::Element(v) => {
                let mut node = json!({"type":v.tag()});
                let attrs: BTreeMap<_, _> = v
                    .attributes(tx)
                    .map(|(k, v)| (k.to_owned(), plain(v, tx)))
                    .collect();
                if !attrs.is_empty() {
                    node["attrs"] = json!(attrs);
                }
                let children = pm_children(v.children(tx), tx)?;
                if !children.is_empty() {
                    node["content"] = json!(children);
                }
                nodes.push(node);
            }
            yrs::XmlOut::Text(v) => {
                for d in v.diff(tx, |_| ()) {
                    let Out::Any(Any::String(s)) = d.insert else {
                        return Err("non-text embed in PM XML text".into());
                    };
                    let mut node = json!({"type":"text","text":s});
                    if let Some(attrs) = d.attributes {
                        let mut marks = vec![];
                        for (key, value) in attrs.iter() {
                            // y-tiptap hashes attributed mark names; the suffix is -- plus 8 base64 characters.
                            let name = key
                                .rsplit_once("--")
                                .filter(|(_, tail)| {
                                    tail.len() == 8
                                        && tail.bytes().all(|c| {
                                            c.is_ascii_alphanumeric() || b"+/=".contains(&c)
                                        })
                                })
                                .map(|(name, _)| name)
                                .unwrap_or(key);
                            let mut mark = json!({"type":name});
                            mark["attrs"] = json!(value);
                            marks.push(mark);
                        }
                        marks.sort_by_key(|m| m["type"].as_str().unwrap().to_owned());
                        if !marks.is_empty() {
                            node["marks"] = json!(marks);
                        }
                    }
                    nodes.push(node);
                }
            }
            _ => return Err("unsupported XML child".into()),
        }
    }
    Ok(nodes)
}
fn project<T: ReadTxn>(doc: &Doc, tx: &T, schema: &Value) -> Result<Value> {
    let settings = tx.get_map("settings").ok_or("settings absent")?;
    let mut saved = plain(
        settings
            .get(tx, "document")
            .ok_or("document settings absent")?,
        tx,
    );
    let fields = saved["metadata"]["fields"]
        .as_object_mut()
        .ok_or("metadata fields absent")?;
    for field in schema["fields"].as_array().into_iter().flatten() {
        if let Some(value) = settings.get(tx, field["field"].as_str().ok_or("field key expected")?)
        {
            fields.insert(
                field["parameter"]
                    .as_str()
                    .ok_or("parameter expected")?
                    .into(),
                plain(value, tx),
            );
        }
    }
    for key in [
        "paper",
        "language",
        "landscape",
        "bullet_marker",
        "spelling_ignored",
    ] {
        if let Some(value) = settings.get(tx, &format!("options.{key}")) {
            fields["options"][key] = plain(value, tx);
        }
    }
    if let Some(value) = settings.get(tx, "signature") {
        fields.insert("signature".into(), plain(value, tx));
    }
    if let Some(Out::YMap(collections)) = settings.get(tx, "collections") {
        for definition in schema["structured"]["collections"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let parameter = definition["parameter"]
                .as_str()
                .ok_or("collection parameter absent")?;
            let Some(Out::YMap(records)) = collections.get(tx, parameter) else {
                continue;
            };
            let Some(Out::YArray(order)) = settings.get(tx, &format!("order:{parameter}")) else {
                continue;
            };
            let mut values = vec![];
            let mut ids = std::collections::BTreeSet::new();
            for id in order.iter(tx) {
                let Out::Any(Any::String(id)) = id else {
                    return Err("record ID expected".into());
                };
                if !ids.insert(id.to_string()) {
                    continue;
                }
                let Some(Out::YMap(record)) = records.get(tx, &id) else {
                    continue;
                };
                let mut item = json!({"id":id});
                for field in definition["fields"]
                    .as_array()
                    .ok_or("record fields absent")?
                {
                    let key = field["key"].as_str().ok_or("record key absent")?;
                    item[key] = match record.get(tx, key) {
                        Some(Out::YXmlFragment(fragment)) => {
                            json!({"type":"doc","content":pm_children(fragment.children(tx),tx)?})
                        }
                        Some(value) => plain(value, tx),
                        None => field["default"].clone(),
                    };
                }
                values.push(item);
            }
            fields.insert(parameter.into(), json!(values));
        }
    }
    let fragment = tx.get_xml_fragment("default").ok_or("body absent")?;
    let _ = doc;
    Ok(
        json!({"document":{"type":"doc","content":pm_children(fragment.children(tx),tx)?},"metadata":saved["metadata"]}),
    )
}
fn insert_node(parent: &Out, tx: &mut TransactionMut, index: u32, node: &Value) -> Result<()> {
    let name = node["type"].as_str().ok_or("node type expected")?;
    if name == "text" {
        let text = node["text"].as_str().ok_or("text expected")?;
        let child = match parent {
            Out::YXmlElement(p) => p.insert(tx, index, XmlTextPrelim::new(text)),
            Out::YXmlFragment(p) => p.insert(tx, index, XmlTextPrelim::new(text)),
            _ => return Err("XML parent expected".into()),
        };
        if let Some(marks) = node["marks"].as_array() {
            let attrs = marks
                .iter()
                .map(|m| {
                    (
                        m["type"].as_str().unwrap().into(),
                        any(m.get("attrs").unwrap_or(&json!({}))),
                    )
                })
                .collect();
            child.format(tx, 0, text.encode_utf16().count() as u32, attrs);
        }
    } else {
        let child = match parent {
            Out::YXmlElement(p) => p.insert(tx, index, XmlElementPrelim::empty(name)),
            Out::YXmlFragment(p) => p.insert(tx, index, XmlElementPrelim::empty(name)),
            _ => return Err("XML parent expected".into()),
        };
        if let Some(attrs) = node["attrs"].as_object() {
            for (k, v) in attrs {
                child.insert_attribute(tx, k.as_str(), any(v));
            }
        }
        if let Some(nodes) = node["content"].as_array() {
            for (index, n) in nodes.iter().enumerate() {
                insert_node(&Out::YXmlElement(child.clone()), tx, index as u32, n)?;
            }
        }
    }
    Ok(())
}
fn operation(doc: &Doc, step: &Value) -> Result<()> {
    // Resolve before opening a write transaction; get_or_insert_* acquires its own lock.
    let value = {
        let tx = doc.transact();
        resolve(doc, &tx, step["path"].as_array().ok_or("path expected")?)?
    };
    let mut tx = doc.transact_mut_with("yrs-probe");
    let index = step["index"].as_u64().unwrap_or(0) as u32;
    let length = step["length"].as_u64().unwrap_or(0) as u32;
    let text = step["text"].as_str().unwrap_or("");
    match step["id"].as_str().unwrap_or("") {
        "text.insert" => match value {
            Out::YText(v) => v.insert(&mut tx, index, text),
            Out::YXmlText(v) => v.insert(&mut tx, index, text),
            _ => return Err("text expected".into()),
        },
        "text.delete" => match value {
            Out::YText(v) => v.remove_range(&mut tx, index, length),
            Out::YXmlText(v) => v.remove_range(&mut tx, index, length),
            _ => return Err("text expected".into()),
        },
        "text.format" => {
            let attrs = step["attrs"]
                .as_object()
                .ok_or("attrs expected")?
                .iter()
                .map(|(k, v)| (k.as_str().into(), any(v)))
                .collect();
            match value {
                Out::YText(v) => v.format(&mut tx, index, length, attrs),
                Out::YXmlText(v) => v.format(&mut tx, index, length, attrs),
                _ => return Err("text expected".into()),
            }
        }
        "map.set" => {
            if let Out::YMap(v) = value {
                v.insert(
                    &mut tx,
                    step["key"].as_str().ok_or("key expected")?,
                    any(&step["value"]),
                );
            } else {
                return Err("map expected".into());
            }
        }
        "xml.attr" => {
            if let Out::YXmlElement(v) = value {
                v.insert_attribute(
                    &mut tx,
                    step["key"].as_str().ok_or("key expected")?,
                    any(&step["value"]),
                );
            } else {
                return Err("element expected".into());
            }
        }
        "array.insert" => {
            if let Out::YArray(v) = value {
                v.insert(&mut tx, index, any(&step["value"]));
            } else {
                return Err("array expected".into());
            }
        }
        "array.delete" => {
            if let Out::YArray(v) = value {
                v.remove_range(&mut tx, index, length);
            } else {
                return Err("array expected".into());
            }
        }
        "xml.insert" => insert_node(&value, &mut tx, index, &step["node"])?,
        "xml.delete" => match value {
            Out::YXmlElement(v) => v.remove_range(&mut tx, index, length),
            Out::YXmlFragment(v) => v.remove_range(&mut tx, index, length),
            _ => return Err("XML parent expected".into()),
        },
        _ => return Err("operation unknown".into()),
    }
    Ok(())
}
fn probe(input: &Value) -> Result<Value> {
    let start = Instant::now();
    let mut options = Options::with_client_id(yrs::ClientID::new(
        input["client_id"].as_u64().unwrap_or(830001),
    ));
    options.offset_kind = if input["offset_kind"] == "bytes" {
        OffsetKind::Bytes
    } else {
        OffsetKind::Utf16
    };
    options.skip_gc = input["skip_gc"].as_bool().unwrap_or(true);
    let doc = Doc::with_options(options);
    // Root type declarations are app schema, not included in the binary wire update.
    doc.get_or_insert_xml_fragment("default");
    for name in input["map_roots"].as_array().into_iter().flatten() {
        doc.get_or_insert_map(name.as_str().ok_or("root name expected")?);
    }
    let bytes = STANDARD
        .decode(input["update"].as_str().ok_or("update expected")?)
        .map_err(|e| e.to_string())?;
    doc.transact_mut()
        .apply_update(Update::decode_v1(&bytes).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let decode_us = start.elapsed().as_secs_f64() * 1e6;
    let before = doc.transact().state_vector();
    let edit = Instant::now();
    let mut history = yrs::undo::UndoManager::<()>::new();
    if input["history"] == true {
        history.expand_scope(&doc, &doc.get_or_insert_xml_fragment("default"));
        history.expand_scope(&doc, &doc.get_or_insert_map("settings"));
        history.include_origin("yrs-probe");
    }
    for step in input["steps"].as_array().into_iter().flatten() {
        operation(&doc, step)?;
    }
    let undo = if input["undo"] == true {
        Some(history.undo_blocking())
    } else {
        None
    };
    let redo = if input["redo"] == true {
        Some(history.redo_blocking())
    } else {
        None
    };
    let edit_us = edit.elapsed().as_secs_f64() * 1e6;
    let encode = Instant::now();
    let tx = doc.transact();
    let full = tx.encode_state_as_update_v1(&StateVector::default());
    let diff = tx.encode_state_as_update_v1(&before);
    let state = tx.state_vector();
    let encode_us = encode.elapsed().as_secs_f64() * 1e6;
    let tree = if input["tree"] == false {
        Value::Null
    } else {
        json!(
            tx.root_refs()
                .map(|(k, v)| (k.to_owned(), raw(v, &tx)))
                .collect::<BTreeMap<_, _>>()
        )
    };
    let projected = if input["project_schema"].is_object() {
        project(&doc, &tx, &input["project_schema"])?
    } else {
        Value::Null
    };
    let anchor = if let Some(encoded) = input["anchor"].as_str() {
        let bytes = STANDARD.decode(encoded).map_err(|e| e.to_string())?;
        let anchor = yrs::StickyIndex::decode_v1(&bytes).map_err(|e| e.to_string())?;
        json!({"offset":anchor.get_offset(&tx).map(|o|o.index),"encoded":STANDARD.encode(anchor.encode_v1())})
    } else {
        Value::Null
    };
    Ok(
        json!({"full":STANDARD.encode(&full),"diff":STANDARD.encode(&diff),"state_vector":STANDARD.encode(state.encode_v1()),"tree":tree,"projected":projected,"anchor":anchor,"undo":undo,"redo":redo,
        "timings_us":{"decode":decode_us,"edit":edit_us,"encode":encode_us,"total":start.elapsed().as_secs_f64()*1e6},"full_bytes":full.len(),"diff_bytes":diff.len()}),
    )
}
fn main() {
    for line in io::stdin().lock().lines() {
        let output = match line
            .map_err(|e| e.to_string())
            .and_then(|line| serde_json::from_str::<Value>(&line).map_err(|e| e.to_string()))
            .and_then(|input| probe(&input))
        {
            Ok(value) => json!({"ok":true,"result":value}),
            Err(error) => json!({"ok":false,"error":error}),
        };
        println!("{output}");
        io::stdout().flush().unwrap();
    }
}
