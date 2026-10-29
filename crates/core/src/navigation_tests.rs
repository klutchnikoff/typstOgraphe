use super::*;
use crate::{
    assets::Assets,
    content::{Node, source_mapped},
    document_types::{DocumentType, Metadata},
};
use serde_json::json;
fn compile(world: &mut TypstWorld, node: &Node) -> (RenderSync, String, SourceMap) {
    let (source, map) = source_mapped(node, &Assets::default()).unwrap();
    let document=world.compile_with_template(&source,&Metadata::demo(DocumentType::Courrier),
        "#let courrier(..args,body) = {set page(width: 140mm,height: 160mm,margin: 15mm); body}").unwrap();
    (
        RenderSync::new(7, world.clone(), document, map.clone()),
        source,
        map,
    )
}
fn paragraph(text: &str) -> Node {
    serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":text}]}]})).unwrap()
}
fn contains(boxes: &[SelectionHighlight], point: PagePoint) -> bool {
    boxes.iter().any(|b| {
        b.page == point.page && {
            let c = &b.corners;
            let cross: Vec<_> = (0..4)
                .map(|i| {
                    let a = c[i];
                    let d = c[(i + 1) % 4];
                    (d[0] - a[0]) * (point.point[1] - a[1])
                        - (d[1] - a[1]) * (point.point[0] - a[0])
                })
                .collect();
            cross.iter().all(|v| *v >= -0.01) || cross.iter().all(|v| *v <= 0.01)
        }
    })
}
#[test]
fn structured_unicode_styles_links_and_repeated_passages_keep_bidirectional_geometry() {
    let node=serde_json::from_value(json!({"type":"doc","content":[
        {"type":"paragraph","content":[{"type":"text","text":"Noël 😀 écrit "},{"type":"text","text":"courrier","marks":[{"type":"bold"}]},{"type":"text","text":" avec "},{"type":"text","text":"Élodie","marks":[{"type":"italic"}]},{"type":"text","text":" # ici."}]},
        {"type":"paragraph","content":[{"type":"text","text":"Consulter le dossier","marks":[{"type":"link","attrs":{"href":"https://example.org"}}]}]},
        {"type":"paragraph","content":[{"type":"text","text":"Passage identique."}]},
        {"type":"paragraph","content":[{"type":"text","text":"Passage identique."}]}
    ]})).unwrap();
    let (sync, source, map) = compile(&mut TypstWorld::default(), &node);
    for word in ["Noël", "courrier", "Élodie", "\\#", "Consulter", "dossier"] {
        let index = map.to_visible(source.find(word).unwrap());
        assert_eq!(
            sync.backward(sync.forward(index).unwrap()),
            Some(index),
            "{word}"
        );
    }
    let first = map.to_visible(source.find("Passage identique").unwrap());
    let last = map.to_visible(source.rfind("Passage identique").unwrap());
    let a = sync.forward(first).unwrap();
    let b = sync.forward(last).unwrap();
    assert!(b.page > a.page || b.point[1] > a.point[1]);
    assert_eq!(sync.backward(b), Some(last));
    assert!(
        sync.backward(PagePoint {
            page: 0,
            point: [0.0, 0.0]
        })
        .is_none()
    );
    let from = map.to_visible(source.find("courrier").unwrap());
    let to = map.to_visible(source.find("dossier").unwrap()) + 7;
    let boxes = sync.selection_highlights_exact(from..to);
    for word in ["courrier", "Élodie", "dossier"] {
        let index = map.to_visible(source.find(word).unwrap());
        assert!(contains(&boxes, sync.forward(index).unwrap()), "{word}");
    }
    assert!(!contains(&boxes, a));
}
#[test]
fn multipage_selection_ligatures_and_old_snapshots_survive_recompilation_of_the_same_world() {
    let mut world = TypstWorld::default();
    let (sync, source, map) = compile(
        &mut world,
        &paragraph(&"Une lettre à Élodie et une formule officielle. ".repeat(400)),
    );
    let start = map.to_visible(source.find("lettre").unwrap());
    let last = map.to_visible(source.rfind("officielle").unwrap());
    let location = sync.forward(last).unwrap();
    assert!(location.page >= 2);
    let back = sync.backward(location).unwrap();
    assert!(back.abs_diff(last) <= 1);
    let boxes = sync.selection_highlights_exact(start..last + 5);
    assert!(boxes.iter().any(|b| b.page >= 1));
    assert!(contains(&boxes, sync.forward(start).unwrap()));
    assert!(contains(&boxes, location));
    let (short, source, map) = compile(&mut world, &paragraph("Office efficace."));
    let start = map.to_visible(source.find("Office").unwrap());
    let boxes = short.selection_highlights_exact(start + 2..start + 3);
    assert!(contains(&boxes, short.forward(start + 2).unwrap()));
    assert!(!contains(&boxes, short.forward(start + 5).unwrap()));
    assert_eq!(sync.backward(location), Some(back));
    assert_eq!(
        short.backward(short.forward(start + 7).unwrap()),
        Some(start + 7)
    );
}
