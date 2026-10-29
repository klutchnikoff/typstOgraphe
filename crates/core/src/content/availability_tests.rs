use super::{api::Session, availability::*};
use serde_json::json;

fn context(s: &mut Session, selection: serde_json::Value) -> serde_json::Value {
    s.execute(
        "context.set",
        json!({"selection":selection,"typing_styles":[]}),
    )
    .unwrap();
    s.query("context.get", json!({})).unwrap()
}
fn selection(path: &[usize], from: u32, to: u32) -> serde_json::Value {
    json!({"kind":"text","anchor":{"zone":{"kind":"body","path":path},"offset":from},"head":{"zone":{"kind":"body","path":path},"offset":to}})
}
#[test]
fn context_availability_is_read_only_type_aware_and_distinguishes_forms_from_edits() {
    let mut s = Session::new();
    s.execute("document.new", json!({"type":"courrier"}))
        .unwrap();
    let update = s.archive().unwrap().yjs;
    let guard = s.guard().cloned();
    let context = context(&mut s, selection(&[0], 0, 0));
    let a = &context["availability"];
    for id in [
        "text.bold",
        "text.italic",
        "insert.footnote",
        "insert.image",
        "insert.table",
        "block.heading_1",
    ] {
        assert_eq!(a[id]["enabled"], true, "{id}");
    }
    assert_eq!(a["text.link"]["reason"], "selection_required");
    assert_eq!(a["insert.citation"]["reason"], "unsupported_document_type");
    // A caret can activate future typing, but cannot style an empty concrete range.
    let concrete = s.query("commands.available", json!({"command":{"id":"text.bold","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0}}}})).unwrap();
    assert_eq!(concrete["enabled"], false);
    assert_eq!(s.archive().unwrap().yjs, update);
    assert_eq!(s.guard().cloned(), guard);
}
#[test]
fn widget_text_keeps_styles_and_inline_objects_but_disallows_titles_and_nested_blocks() {
    let mut s = Session::new();
    s.execute("document.new", json!({"type":"article"}))
        .unwrap();
    s.execute("block.insert", json!({"parent":{"root":{"kind":"body"},"path":[]},"index":0,"node":{"type":"quote","content":[{"type":"paragraph","content":[{"type":"text","text":"Texte"}]},{"type":"quoteAttribution"}]}})).unwrap();
    let a = context(&mut s, selection(&[0, 0], 0, 2))["availability"].clone();
    for id in [
        "text.bold",
        "text.link",
        "insert.footnote",
        "insert.citation",
    ] {
        assert_eq!(a[id]["enabled"], true, "{id}");
    }
    for id in [
        "block.heading_1",
        "block.bullet",
        "insert.image",
        "insert.table",
    ] {
        assert_eq!(a[id]["reason"], "top_level_required", "{id}");
    }
    s.execute("block.insert", json!({"parent":{"root":{"kind":"body"},"path":[]},"index":0,"node":{"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph"}]}]}})).unwrap();
    let a = context(&mut s, selection(&[0, 0, 0], 0, 0))["availability"].clone();
    assert_eq!(a["block.numbered"]["enabled"], true);
    assert_eq!(a["block.quote"]["reason"], "top_level_required");
}
#[test]
fn rich_header_contexts_share_styles_but_never_offer_body_objects() {
    let mut s = Session::new();
    s.execute("document.new", json!({"type":"article"}))
        .unwrap();
    let data = serde_json::to_value(&s.state().unwrap().settings.metadata).unwrap();
    let id = data["fields"]["localizations"][0]["id"].as_str().unwrap();
    let point = json!({"zone":{"kind":"record","collection":"localizations","id":id,"field":"title","path":[0]},"offset":0});
    let a =
        context(&mut s, json!({"kind":"text","anchor":point,"head":point}))["availability"].clone();
    assert_eq!(a["text.bold"]["enabled"], true);
    for id in [
        "block.heading_1",
        "block.quote",
        "insert.footnote",
        "insert.citation",
        "insert.table",
    ] {
        assert_eq!(a[id]["reason"], "body_required", "{id}");
    }
    assert_eq!(
        for_selection(s.state().unwrap(), None)["text.bold"].reason,
        Some(Reason::NoSelection)
    );
}

#[test]
fn structural_selections_do_not_turn_document_boundaries_into_inline_insertion_points() {
    let mut s = Session::new();
    s.execute("document.new", json!({"type":"article"}))
        .unwrap();
    let a = context(&mut s, json!({"kind":"all","root":{"kind":"body"}}))["availability"].clone();
    assert_eq!(a["text.bold"]["enabled"], true);
    assert_eq!(a["block.heading_1"]["enabled"], true);
    assert_eq!(a["insert.footnote"]["reason"], "text_required");
    let a = context(
        &mut s,
        json!({"kind":"node","target":{"root":{"kind":"body"},"path":[0]}}),
    )["availability"]
        .clone();
    assert_eq!(a["block.heading_1"]["enabled"], false);
    s.execute(
        "insert.table",
        json!({"parent":{"root":{"kind":"body"},"path":[]},"index":0,"rows":1,"columns":2,"header":true}),
    )
    .unwrap();
    let a = context(&mut s, json!({"kind":"cells","anchor":{"root":{"kind":"body"},"path":[0,0,0,0]},"head":{"root":{"kind":"body"},"path":[0,0,0,1]}}))["availability"].clone();
    assert_eq!(a["text.bold"]["enabled"], true);
    assert_eq!(a["insert.footnote"]["reason"], "text_required");
    assert_eq!(a["block.heading_1"]["reason"], "top_level_required");
}
