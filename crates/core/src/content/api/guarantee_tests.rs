use super::*;
#[test]
fn search_and_replacement_keep_exact_utf16_targets_in_notes_cells_captions_and_abstracts() {
    let mut s = new("article");
    s.batch(&[
        call("block.replace",json!({"target":{"root":{"kind":"body"},"path":[0]},"node":{"type":"paragraph","content":[{"type":"text","text":"😀commun"},{"type":"footnote","attrs":{"text":"😀commun"}}]}})),
        call("insert.table",json!({"parent":{"root":{"kind":"body"},"path":[]},"index":1,"rows":1,"columns":1,"header":false})),
        call("text.insert",json!({"target":target(&[1,0,0,0,0],0,0),"text":"😀commun"})),
        call("text.insert",json!({"target":target(&[1,1],0,0),"text":"😀commun"})),
        call("record.add",json!({"collection":"localizations","id":"en","values":{"language":"en"}})),
        call("text.insert",json!({"target":{"zone":{"kind":"record","collection":"localizations","id":"en","field":"abstract","path":[0]},"from":0,"to":0},"text":"😀commun"})),
    ], None).unwrap();
    let before = s.archive().unwrap().yjs;
    let found = s.query("search.find", json!({"query":"COMMUN"})).unwrap();
    let matches = found["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 5, "{found}");
    let mut replacements = vec![];
    for (index, entry) in matches.iter().enumerate() {
        let t = entry["target"].clone();
        assert_eq!(t["from"], 2);
        assert_eq!(t["to"], 8);
        assert_eq!(
            s.query("document.text", json!({"target":t})).unwrap()["text"],
            "commun"
        );
        let mut invalid = t.clone();
        invalid["from"] = json!(1);
        assert!(s.query("document.text", json!({"target":invalid})).is_err());
        assert_eq!(s.archive().unwrap().yjs, before);
        replacements.push(call(
            "text.insert",
            json!({"target":t,"text":format!("zone{index}")}),
        ));
    }
    s.batch(&replacements, None).unwrap();
    assert!(
        s.query("search.find", json!({"query":"commun"})).unwrap()["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for index in 0..5 {
        assert_eq!(
            s.query("search.find", json!({"query":format!("zone{index}")}))
                .unwrap()["matches"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    s.execute("edit.undo", json!({})).unwrap();
    assert_eq!(
        s.query("search.find", json!({"query":"commun"})).unwrap(),
        found
    );
}
#[test]
fn bibliography_search_uses_project_authors_and_titles_without_mutating_the_document() {
    let mut s = new("article");
    s.batch(&[
        call("record.add",json!({"collection":"references","id":"books","values":{"name":"ouvrages.bib","contents":"@book{martin2026, author={Martin, Camille}, title={Les archives municipales}, year={2026}}"}})),
        call("record.add",json!({"collection":"references","id":"papers","values":{"name":"articles.bib","contents":"@article{durand2025, author={Durand, Alex}, title={Une autre étude}, year={2025}, journal={Revue}}"}})),
    ],None).unwrap();
    let before = s.archive().unwrap().yjs;
    for query in ["Camille", "archives municipales", "archivs municiples"] {
        let result = s
            .query("bibliography.search", json!({"query":query}))
            .unwrap();
        let entries = result["entries"].as_array().unwrap();
        assert!(!entries.is_empty(), "{query}: {result}");
        assert_eq!(entries[0]["key"], "martin2026");
        assert_eq!(entries[0]["file_id"], "books");
    }
    assert_eq!(s.archive().unwrap().yjs, before);
    let mut letter = new("courrier");
    assert!(
        letter
            .query("bibliography.search", json!({"query":"Camille"}))
            .is_err()
    );
}
