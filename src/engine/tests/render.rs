use super::*;
use crate::rendering::{TypstWorld, render};

fn publish(e: &mut Engine) -> RenderInfo {
    let request = e.preview_request().unwrap();
    let output = render(&mut TypstWorld::default(), &request).unwrap();
    e.install_render(e.target_guard(), output.sync).unwrap()
}
fn locate(
    e: &Engine,
    info: &RenderInfo,
    position: RenderPosition,
) -> Result<LocateResult, CommandError> {
    let call = Query::Locate(LocateParams {
        render: info.reference,
        position,
    })
    .to_call();
    serde_json::from_value(e.query_call(call)?).map_err(CommandError::invalid)
}
fn word(e: &Engine, text: &str) -> std::ops::Range<usize> {
    let byte = e.document().view.text.find(text).unwrap();
    let start = e.document().view.text[..byte].chars().count();
    start..start + text.chars().count()
}

#[test]
fn render_queries_bridge_logical_unicode_zones_and_typst_points_without_mutation() {
    let mut e = engine(
        "#import \"modele.typ\": postscript, attachment\n\n= Échéance\n\nNoël écrit un *courrier* avec _Élodie_.\n\n/ Terme: Définition\n\n#quote(block: true, attribution: [Attribution])[Une citation.]\n\n#figure(image(\"images/parc.svg\", alt: \"Des arbres\"), caption: [Le parc.])\n\n#postscript[Merci]\n\n#attachment[Rapport]",
    );
    e.session.document.assets.insert(
        "images/parc.svg".into(),
        include_bytes!("../../../tests/fixtures/parc.svg")
            .as_slice()
            .into(),
    );
    let info = publish(&mut e);
    assert_eq!(
        e.query(Query::Inspect(Empty {})).unwrap()["render"],
        serde_json::to_value(&info).unwrap()
    );
    let stored = e.stored_document();
    let before = e.query(Query::Inspect(Empty {})).unwrap();
    assert!(!info.pages.is_empty());
    for text in [
        "Échéance",
        "Noël",
        "courrier",
        "Élodie",
        "Terme",
        "Définition",
        "citation",
        "Attribution",
        "Le parc",
        "Merci",
        "Rapport",
    ] {
        let range = word(&e, text);
        let target = explicit(&e, range.start..range.start);
        let result = locate(&e, &info, RenderPosition::ToPreview { target }).unwrap();
        assert_eq!(result.target.map(TextTarget::from), Some(target));
        let point = result.point.expect(text);
        let backward = locate(&e, &info, RenderPosition::ToSource { point }).unwrap();
        let (anchor, head) = e.resolve_target(backward.target.expect(text)).unwrap();
        assert_eq!((anchor, head), (range.start, range.start), "{text}");
        let boxes: SelectionBoxesResult = serde_json::from_value(
            e.query_call(
                Query::SelectionBoxes(SelectionBoxesParams {
                    render: info.reference,
                    target: explicit(&e, range.clone()),
                })
                .to_call(),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!boxes.boxes.is_empty(), "{text}");
        for b in &boxes.boxes {
            assert!(b.page < info.pages.len());
            assert!(b.corners.iter().flatten().all(|value| value.is_finite()));
        }
        assert_eq!(
            boxes,
            e.selection_boxes(SelectionBoxesParams {
                render: info.reference,
                target: explicit(&e, range.end..range.start),
            })
            .unwrap()
        );
    }
    assert_eq!(e.query(Query::Inspect(Empty {})).unwrap(), before);
    assert_eq!(e.stored_document(), stored);
    assert!(e.session.undo.is_empty() && e.session.redo.is_empty());
}

#[test]
fn render_queries_refuse_stale_foreign_missing_and_replaced_render_references() {
    let mut e = engine("Un texte.");
    let first = publish(&mut e);
    let second = publish(&mut e);
    assert_ne!(first.reference, second.reference);
    assert_eq!(
        locate(
            &e,
            &first,
            RenderPosition::ToPreview {
                target: TextTarget::Current {}
            }
        )
        .unwrap_err()
        .reason(),
        "render_unavailable"
    );
    let other = engine("Autre.");
    assert_eq!(
        locate(
            &other,
            &second,
            RenderPosition::ToPreview {
                target: TextTarget::Current {}
            }
        )
        .unwrap_err()
        .reason(),
        "foreign_document"
    );
    let selected = explicit(&e, 0..2);
    let old_request = e.preview_request().unwrap();
    let old_guard = e.target_guard();
    call(&mut e, "text.insert", json!({"text":"Suite "})).unwrap();
    assert_eq!(
        e.selection_boxes(SelectionBoxesParams {
            render: second.reference,
            target: selected
        })
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    assert!(e.query(Query::Inspect(Empty {})).unwrap()["render"].is_null());
    let current = publish(&mut e);
    assert_eq!(
        e.selection_boxes(SelectionBoxesParams {
            render: current.reference,
            target: selected
        })
        .unwrap_err()
        .reason(),
        "stale_revision"
    );
    // Une réponse tardive ne remplace pas le rendu valide déjà enregistré.
    let old = render(&mut TypstWorld::default(), &old_request).unwrap();
    assert_eq!(
        e.install_render(old_guard, old.sync).unwrap_err().reason(),
        "stale_revision"
    );
    assert!(
        locate(
            &e,
            &current,
            RenderPosition::ToPreview {
                target: TextTarget::Current {}
            }
        )
        .is_ok()
    );
    e.render = None;
    assert_eq!(
        locate(
            &e,
            &current,
            RenderPosition::ToPreview {
                target: TextTarget::Current {}
            }
        )
        .unwrap_err()
        .reason(),
        "render_unavailable"
    );
    e.open = false;
    assert_eq!(
        locate(
            &e,
            &current,
            RenderPosition::ToPreview {
                target: TextTarget::Current {}
            }
        )
        .unwrap_err()
        .reason(),
        "no_document"
    );
    assert_eq!(e.preview_request().err().unwrap().reason(), "no_document");
}

#[test]
fn render_points_are_validated_and_model_decorations_have_no_editable_target() {
    let mut e = engine("Un texte.");
    let info = publish(&mut e);
    for point in [
        PagePoint {
            page: info.pages.len(),
            point: [0.0, 0.0],
        },
        PagePoint {
            page: 0,
            point: [-1.0, 0.0],
        },
        PagePoint {
            page: 0,
            point: [0.0, f32::NAN],
        },
        PagePoint {
            page: 0,
            point: [f32::INFINITY, 0.0],
        },
        PagePoint {
            page: 0,
            point: [info.pages[0][0] + 1.0, 0.0],
        },
    ] {
        assert_eq!(
            e.locate(LocateParams {
                render: info.reference,
                position: RenderPosition::ToSource { point }
            })
            .unwrap_err()
            .code,
            ErrorCode::InvalidParams
        );
    }
    let decoration = locate(
        &e,
        &info,
        RenderPosition::ToSource {
            point: PagePoint {
                page: 0,
                point: [0.0, 0.0],
            },
        },
    )
    .unwrap();
    assert!(decoration.target.is_none());
    assert!(
        e.selection_boxes(SelectionBoxesParams {
            render: info.reference,
            target: TextTarget::Current {}
        })
        .unwrap()
        .boxes
        .is_empty()
    );
    let forged: RenderReference =
        serde_json::from_value(json!({"guard":info.reference.guard,"id":0})).unwrap();
    assert_eq!(
        e.locate(LocateParams {
            render: forged,
            position: RenderPosition::ToPreview {
                target: TextTarget::Current {}
            }
        })
        .unwrap_err()
        .reason(),
        "render_unavailable"
    );
    for (id, params) in [
        (
            "render.locate",
            json!({"render":info.reference,"position":{"direction":"to_preview","offset":1}}),
        ),
        (
            "render.locate",
            json!({"render":info.reference,"position":{"direction":"to_source","point":{"page":0,"point":[0,0],"pixels":true}}}),
        ),
        (
            "render.selection_boxes",
            json!({"render":info.reference,"range":[0,3]}),
        ),
        ("document.statistics", json!({"offset":3})),
    ] {
        assert!(Query::from_call(id, params).is_err(), "{id}");
    }
}

#[test]
fn render_queries_keep_page_numbers_and_cross_page_selection_geometry() {
    let mut e = engine(&"Un paragraphe à retrouver.\n\n".repeat(100));
    let info = publish(&mut e);
    assert!(info.pages.len() > 1);
    let end = e.document().view.len();
    let last = locate(
        &e,
        &info,
        RenderPosition::ToPreview {
            target: explicit(&e, end - 12..end - 12),
        },
    )
    .unwrap()
    .point
    .unwrap();
    assert!(last.page > 0);
    let result = e
        .selection_boxes(SelectionBoxesParams {
            render: info.reference,
            target: TextTarget::All {},
        })
        .unwrap();
    assert!(result.boxes.iter().any(|b| b.page == 0));
    assert!(result.boxes.iter().any(|b| b.page == last.page));
}
