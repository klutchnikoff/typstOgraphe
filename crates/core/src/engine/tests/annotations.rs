use super::*;
use crate::document::structure::BlockContent;

fn annotated_engine(source: &str) -> Engine {
    let mut e = engine(source);
    if source.contains("image(") {
        e.session.document.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../../../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        e.session.saved.assets = e.session.document.assets.clone();
    }
    e
}

fn attribution_id(e: &Engine) -> crate::document::structure::ZoneId {
    e.session
        .document
        .structure
        .blocks
        .iter()
        .find_map(|block| match &block.content {
            BlockContent::Quote { attribution, .. } => Some(attribution.id),
            _ => None,
        })
        .unwrap()
}

#[test]
fn relative_selection_preserves_content_revision_resources_and_both_history_stacks() {
    for source in [
        "#figure(image(\"images/parc.svg\"))",
        "#figure(image(\"images/parc.svg\"), caption: [Été *日本*])",
        "#quote(block: true)[Une pensée.]",
        "#quote(block: true, attribution: [Élodie *日本*])[Une pensée.]",
    ] {
        let mut e = annotated_engine(source);
        let annotation = e.session.document.view.annotations().remove(0);
        let target = explicit(&e, annotation.main_target());
        let original = e.stored_document();
        let structure = e.query(Query::Structure(Empty {})).unwrap();
        let when = e.session.changed_at;
        e.session.typing_group = true;
        for part in ["annotation", "annotation", "body", "annotation", "body"] {
            let outcome =
                call(&mut e, "text.select", json!({"target":target, "part":part})).unwrap();
            assert_eq!(
                e.selection().range(),
                if part == "body" {
                    annotation.main_target()
                } else {
                    annotation.content.start..annotation.content.start
                }
            );
            assert!(!outcome.changed);
            assert_eq!(outcome.effects, [Effect::RevealSelection]);
            assert!(outcome.selection.is_some());
            assert!(e.stored_document() == original);
            assert_eq!(e.query(Query::Structure(Empty {})).unwrap(), structure);
            assert_eq!(e.session.revision, 0);
            assert_eq!(e.session.changed_at, when);
            assert!(!e.session.typing_group);
            assert!(e.session.undo.is_empty() && e.session.redo.is_empty() && !e.is_dirty());
        }
        call(&mut e, "text.bold", json!({"target":target})).unwrap();
        // Les cas au curseur ne créent pas d’historique : prendre une sélection
        // textuelle pour vérifier aussi la conservation de la pile de redo.
        let target = explicit(&e, annotation.content.clone());
        call(
            &mut e,
            "text.insert",
            json!({"target":target, "text":"Noël"}),
        )
        .unwrap();
        call(&mut e, "edit.undo", json!({})).unwrap();
        let counts = (e.session.undo.len(), e.session.redo.len());
        call(&mut e, "text.select", json!({"part":"body"})).unwrap();
        assert_eq!((e.session.undo.len(), e.session.redo.len()), counts);
    }
}

#[test]
fn exiting_preserves_selected_annotation_styles_assets_and_restores_selection_on_undo_redo() {
    for source in [
        "#figure(image(\"images/parc.svg\"), caption: [Été *日本*])\n\nSuite.",
        "#quote(block: true, attribution: [Été *日本*])[Une pensée.]\n\nSuite.",
    ] {
        for from_body in [false, true] {
            let mut e = annotated_engine(source);
            let annotation = e.session.document.view.annotations().remove(0);
            let range = if from_body {
                annotation.main_target()
            } else {
                annotation.content.clone()
            };
            let mut selection = e.resolve(explicit(&e, range)).unwrap();
            std::mem::swap(&mut selection.primary, &mut selection.secondary);
            e.set_adapter_selection(selection).unwrap();
            let before = e.session.document.clone();
            let saved = e.stored_document();
            let outcome = call(&mut e, "block.exit", json!({})).unwrap();
            assert!(outcome.changed);
            assert_eq!(outcome.effects, [Effect::Render, Effect::RevealSelection]);
            assert_eq!(e.session.undo.len(), 1);
            assert_eq!(e.session.document.view.annotations()[0], annotation);
            assert_eq!(e.session.document.assets, saved.assets);
            let after = e.session.document.clone();
            let after_selection = e.selection();
            assert_eq!(
                after_selection.range(),
                annotation.structure.end + 1..annotation.structure.end + 1
            );
            assert_eq!(
                e.session.document.view.blocks[e
                    .session
                    .document
                    .view
                    .line_at(after_selection.primary.index)],
                crate::document::BlockKind::Paragraph
            );
            assert_eq!(
                e.session.document.view.styles[..annotation.structure.end],
                before.view.styles[..annotation.structure.end]
            );
            // Le sérialiseur existant compacte les séparateurs vides après
            // une figure. Le contenu annoté et le source restent stables.
            let reopened = Document::from_source(e.session.document.source()).unwrap();
            assert_eq!(reopened.source(), e.session.document.source());
            assert_eq!(
                reopened.view.annotations(),
                e.session.document.view.annotations()
            );
            call(&mut e, "edit.undo", json!({})).unwrap();
            assert_eq!(e.session.document, before);
            assert_eq!(e.selection(), selection);
            call(&mut e, "edit.redo", json!({})).unwrap();
            assert_eq!(e.session.document, after);
            assert_eq!(e.selection(), after_selection);
        }
    }
}

#[test]
fn split_quote_replaces_only_the_body_selection_and_keeps_attribution_identity() {
    let mut e = engine("#quote(block: true, attribution: [Élodie *日本*])[Un été à Paris.]");
    let id = attribution_id(&e);
    let before = e.session.document.clone();
    let mut selection = e.resolve(explicit(&e, 3..6)).unwrap();
    std::mem::swap(&mut selection.primary, &mut selection.secondary);
    e.set_adapter_selection(selection).unwrap();
    let outcome = call(&mut e, "block.split", json!({})).unwrap();
    assert_eq!(outcome.effects, [Effect::Render]);
    assert_eq!(e.session.document.view.text, "Un \n à Paris.\nÉlodie 日本");
    assert_eq!(e.selection().range(), 4..4);
    assert_eq!(attribution_id(&e), id);
    assert_eq!(e.session.undo.len(), 1);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.document, before);
    assert_eq!(e.selection(), selection);
}

#[test]
fn splitting_a_fully_selected_quote_replaces_the_whole_structure_with_one_undo() {
    let mut e = engine("#quote(block: true, attribution: [Élodie])[Une pensée.]");
    let range = e.session.document.view.annotations()[0].structure.clone();
    e.set_adapter_selection(e.resolve(explicit(&e, range)).unwrap())
        .unwrap();
    let before = e.session.document.clone();
    call(&mut e, "block.split", json!({})).unwrap();
    assert!(e.session.document.view.annotations().is_empty());
    assert!(
        e.session
            .document
            .view
            .blocks
            .iter()
            .all(|block| *block == crate::document::BlockKind::Paragraph)
    );
    assert_eq!(e.session.document.view.text, "\n");
    assert_eq!(e.selection().range(), 1..1);
    assert_eq!(e.session.undo.len(), 1);
    call(&mut e, "edit.undo", json!({})).unwrap();
    assert_eq!(e.session.document, before);
}

#[test]
fn empty_quote_lines_exit_without_losing_the_attribution_or_following_body() {
    for body in ["", "Alpha\n", "Alpha\n\nBeta"] {
        let mut e = engine(&format!(
            "#quote(block: true, attribution: [Élodie])[{body}]"
        ));
        let id = attribution_id(&e);
        let at = e
            .session
            .document
            .view
            .line_ranges()
            .iter()
            .enumerate()
            .find(|(line, range)| {
                range.is_empty()
                    && e.session.document.view.blocks[*line] == crate::document::BlockKind::Quote
            })
            .unwrap()
            .1
            .start;
        e.set_adapter_selection(e.resolve(explicit(&e, at..at)).unwrap())
            .unwrap();
        let before = e.session.document.clone();
        call(&mut e, "block.split", json!({})).unwrap();
        assert_eq!(attribution_id(&e), id);
        assert!(e.session.document.view.text.contains("Élodie"));
        if body.contains("Beta") {
            assert!(e.session.document.source().contains("Beta"));
            assert_eq!(e.session.document.view.quote_groups()[0].body.len(), 2);
        }
        assert_eq!(
            e.session.document.view.blocks
                [e.session.document.view.line_at(e.selection().primary.index)],
            crate::document::BlockKind::Paragraph
        );
        assert_eq!(e.session.undo.len(), 1);
        call(&mut e, "edit.undo", json!({})).unwrap();
        assert_eq!(e.session.document, before);
    }
}

#[test]
fn annotation_commands_reject_stale_foreign_missing_and_cross_boundary_targets_atomically() {
    let source = "#quote(block: true, attribution: [Élodie])[Une pensée.]\n\nSuite.";
    let mut e = annotated_engine(source);
    let annotation = e.session.document.view.annotations().remove(0);
    let stale = explicit(&e, annotation.content.clone());
    let other = engine(source);
    let foreign = explicit(&other, annotation.content.clone());
    let target = explicit(&e, 0..3);
    call(&mut e, "text.bold", json!({"target":target})).unwrap();
    let outside = explicit(
        &e,
        e.session.document.view.len()..e.session.document.view.len(),
    );
    for (target, reason) in [
        (stale, "stale_revision"),
        (foreign, "foreign_document"),
        (outside, "annotation_required"),
    ] {
        for (id, part) in [
            ("text.select", Some("annotation")),
            ("block.exit", None),
            ("block.split", None),
        ] {
            let before = e.stored_document();
            let selection = e.selection();
            let state = (
                e.session.revision,
                e.session.undo.len(),
                e.session.redo.len(),
                e.session.changed_at,
            );
            let params = if let Some(part) = part {
                json!({"target":target, "part":part})
            } else {
                json!({"target":target})
            };
            let command = Command::from_call(id, params.clone()).unwrap();
            assert!(matches!(
                e.available(&command),
                Availability::Disabled { .. }
            ));
            let expected = if reason == "annotation_required" && id != "text.select" {
                "structured_block_required"
            } else {
                reason
            };
            assert_eq!(call(&mut e, id, params).unwrap_err().reason(), expected);
            assert!(e.stored_document() == before);
            assert_eq!(e.selection(), selection);
            assert_eq!(
                (
                    e.session.revision,
                    e.session.undo.len(),
                    e.session.redo.len(),
                    e.session.changed_at
                ),
                state
            );
        }
    }
    let target = explicit(&e, annotation.content.start - 1..annotation.content.end);
    let before = e.session.document.clone();
    assert_eq!(
        call(&mut e, "block.split", json!({"target":target}))
            .unwrap_err()
            .reason(),
        "protected_boundary"
    );
    assert_eq!(e.session.document, before);
    // Une sortie explicite conserve les deux zones, même si la sélection les traverse.
    call(&mut e, "block.exit", json!({"target":target})).unwrap();
    assert!(e.session.document.view.text.contains("Une pensée.\nÉlodie"));
}
