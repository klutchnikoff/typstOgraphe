use super::*;
use crate::content::{
    contract::{Settings, State},
    editing::Direction,
    shared::SharedDocument,
};
use serde_json::json;
fn archive(lines: &[&str]) -> Archive {
    let document=serde_json::from_value(json!({"type":"doc","content":lines.iter().map(|s|if s.is_empty(){json!({"type":"paragraph"})}else{json!({"type":"paragraph","content":[{"type":"text","text":s}]})}).collect::<Vec<_>>()})).unwrap();
    let state = State::new(document, Settings::default());
    Archive {
        document: state.document,
        metadata: state.settings.metadata,
        template: state.settings.template.clone(),
        model_id: state.settings.model_id,
        assets: Default::default(),
        dependencies: crate::packages::for_template(&state.settings.template),
        yjs: vec![],
    }
}
fn engine(a: &Archive) -> (EditingSession, ContextId) {
    let guard = Guard {
        document: "document".into(),
        revision: 7,
    };
    let mut engine = EditingSession::open(
        guard.clone(),
        &SharedDocument::from_state(&a.state()).unwrap().update(),
        a.assets.clone(),
    )
    .unwrap();
    let context = engine.open_context(&guard).unwrap();
    (engine, context)
}
#[test]
fn local_reprises_keep_both_states_frozen_advance_their_own_guard_and_undo_separately() {
    let before = archive(&["ancien A", "ancre", "ancien B", ""]);
    let after = archive(&["nouveau A", "ancre", "nouveau B", ""]);
    let (mut e, c) = engine(&after);
    let mut p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::Before,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.report(e.guard()).groups.len(), 2);
    for i in [1, 0] {
        let ready = p.prepare(&e, &[i]).unwrap();
        p.publish(&mut e, c, ready).unwrap();
    }
    assert_eq!(e.state(), &before.state());
    assert_eq!(p.before().state(), before.state());
    assert_eq!(p.after().state(), after.state());
    assert_eq!(e.context(&e.guard().clone(), c).unwrap().undo_steps, 2);
    e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
    assert_eq!(e.state().document.content[0], after.document.content[0]);
    assert!(p.report(e.guard()).stale);
    e.history(&e.guard().clone(), c, Direction::Redo).unwrap();
    assert_eq!(e.state(), &before.state());
    assert!(p.report(e.guard()).stale);
    assert!(p.prepare(&e, &[0]).is_err());
}
#[test]
fn mcp_direction_structural_insertions_and_refusals_use_the_same_engine() {
    let before = archive(&["a", "stable", "b", ""]);
    let after = archive(&["a", "inserted", "stable", "new b", ""]);
    let (mut e, c) = engine(&before);
    let mut p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.groups.len(), 2);
    p.refuse(&[0]).unwrap();
    let guard = e.guard().clone();
    let ready = p.prepare(&e, &[1]).unwrap();
    p.publish(&mut e, c, ready).unwrap();
    assert_eq!(e.state().document.content[0], before.document.content[0]);
    assert_eq!(e.state().document.content[2], after.document.content[3]);
    assert_eq!(e.guard().revision, guard.revision + 1);
    assert!(p.prepare(&e, &[0]).is_err());
    assert!(p.refuse(&[1]).is_err());
}
#[test]
fn malformed_decisions_and_refusal_during_preparation_never_publish() {
    let before = archive(&["a", ""]);
    let after = archive(&["b", ""]);
    let (mut e, c) = engine(&before);
    let mut p = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    for ids in [vec![], vec![99], vec![0, 0]] {
        assert!(p.prepare(&e, &ids).is_err());
    }
    let ready = p.prepare(&e, &[0]).unwrap();
    let guard = e.guard().clone();
    p.refuse(&[0]).unwrap();
    assert!(p.publish(&mut e, c, ready).is_err());
    assert_eq!(e.guard(), &guard);
}
#[test]
fn styles_links_unicode_and_meaningful_spaces_are_not_lost() {
    let before = archive(&["é 😀 x y", ""]);
    let mut after = before.clone();
    after.document.content[0].content[0].marks =
        serde_json::from_value(json!([{"type":"bold"}])).unwrap();
    let (e, _) = engine(&before);
    let p = Comparison::new(before.clone(), after, SourceSide::After, e.guard().clone()).unwrap();
    assert_eq!(p.groups.len(), 1);
    assert!(
        p.groups[0]
            .changes
            .iter()
            .all(|c| c.kind == TextChangeKind::Equal)
    );
    let after = archive(&["é 😀 x\u{a0}y", ""]);
    let p = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    assert!(
        p.groups[0]
            .changes
            .iter()
            .any(|c| c.kind == TextChangeKind::Insert && c.text.contains('\u{a0}'))
    );
}
#[test]
fn word_highlights_are_attached_to_source_leaves_even_beside_inline_badges() {
    use crate::document_types::{DocumentType, Metadata};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/comparison-article-badges.json"
    ))
    .unwrap();
    let mut before = archive(&[""]);
    before.document = serde_json::from_value(fixture["before"].clone()).unwrap();
    before.metadata = Metadata::demo(DocumentType::Article);
    let Metadata::Article(metadata) = &mut before.metadata else {
        unreachable!()
    };
    metadata
        .references
        .push(crate::bibliography::ReferenceFile {
            id: "demo-books".into(),
            name: "books.bib".into(),
            contents: "@book{lefevre2021, author={Lefèvre, Camille}, title={Écrire}, year={2021}}"
                .into(),
        });
    before.template = DocumentType::Article.default_model();
    before.model_id = "article.model.working".into();
    before.dependencies = crate::packages::for_template(&before.template);
    let mut after = before.clone();
    after.document = serde_json::from_value(fixture["after"].clone()).unwrap();
    let (e, _) = engine(&before);
    let p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    let group = &p.groups[0];
    fn marked(archive: &Archive, ranges: &[Highlight]) -> Vec<String> {
        ranges
            .iter()
            .map(|r| {
                let mut node = &archive.document;
                for index in &r.path {
                    node = &node.content[*index];
                }
                let text: Vec<u16> = node.text.as_ref().unwrap().encode_utf16().collect();
                String::from_utf16(&text[r.from as usize..r.to as usize]).unwrap()
            })
            .collect()
    }
    assert_eq!(marked(&before, &group.before_highlights), ["doit"]);
    assert_eq!(marked(&after, &group.after_highlights), ["peut", "xx "]);
    assert_eq!(group.before_highlights[0].path, [0, 2]);
    assert!(group.linked);
    after.document.content[1].content[3].attrs.as_mut().unwrap()["text"] = json!("Une autre note");
    let p = Comparison::new(before.clone(), after, SourceSide::After, e.guard().clone()).unwrap();
    // The annotation text is represented by a localized badge, marked as one atom.
    for ranges in [
        &p.groups[0].before_highlights,
        &p.groups[0].after_highlights,
    ] {
        assert!(ranges.contains(&Highlight {
            path: vec![1, 3],
            from: 0,
            to: 1
        }));
    }

    // Local offsets use UTF-16, including astral characters before the changed word.
    let before = archive(&["😀 doit rester", ""]);
    let after = archive(&["😀 peut rester", ""]);
    let p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.groups[0].before_highlights[0].from, 3);
    assert_eq!(marked(&before, &p.groups[0].before_highlights), ["doit"]);
    assert_eq!(marked(&after, &p.groups[0].after_highlights), ["peut"]);
}
#[test]
fn metadata_and_content_are_one_safe_atomic_unit() {
    let before = archive(&["before", ""]);
    let mut after = archive(&["after", ""]);
    after.metadata.options_mut().landscape = true;
    let (mut e, c) = engine(&before);
    let mut p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.groups.len(), 1);
    assert!(p.groups[0].linked);
    let ready = p.prepare(&e, &[0]).unwrap();
    p.publish(&mut e, c, ready).unwrap();
    assert_eq!(e.state(), &after.state());
    e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
    assert_eq!(e.state(), &before.state());
}
#[test]
fn transport_packages_do_not_create_or_couple_documentary_decisions() {
    let mut before = archive(&["old A", "anchor", "old B", ""]);
    let after = archive(&["new A", "anchor", "new B", ""]);
    let mut old = (*before.dependencies).clone();
    old.insert(
        "capabilities/dates.typ".into(),
        b"#panic(\"Transport copy must not become a runtime library\")"
            .as_slice()
            .into(),
    );
    before.dependencies = std::sync::Arc::new(old);
    for source in [SourceSide::Before, SourceSide::After] {
        let (mut e, c) = engine(match source {
            SourceSide::Before => &after,
            SourceSide::After => &before,
        });
        let mut p =
            Comparison::new(before.clone(), after.clone(), source, e.guard().clone()).unwrap();
        assert_eq!(p.groups.len(), 2);
        assert!(p.groups.iter().all(|g| matches!(g.section, Section::Body)));
        assert!(p.files.is_empty());
        for id in [1, 0] {
            let ready = p.prepare(&e, &[id]).unwrap();
            assert_eq!(ready.archive().dependencies, after.dependencies);
            p.publish(&mut e, c, ready).unwrap();
        }
        assert_eq!(e.context(&e.guard().clone(), c).unwrap().undo_steps, 2);
    }
    let (e, _) = engine(&before);
    let mut same = before.clone();
    same.dependencies = after.dependencies;
    let p = Comparison::new(before, same, SourceSide::After, e.guard().clone()).unwrap();
    assert!(p.groups.is_empty());
    assert!(p.files.is_empty());
}
#[test]
fn a_foreign_guard_or_edit_during_validation_cannot_replace_confirmed_work() {
    let before = archive(&["a", ""]);
    let after = archive(&["b", ""]);
    let (mut e, c) = engine(&before);
    let mut p =
        Comparison::new(before.clone(), after, SourceSide::After, e.guard().clone()).unwrap();
    let ready = p.prepare(&e, &[0]).unwrap();
    e.execute(
        &e.guard().clone(),
        c,
        &[crate::content::commands::Command::Insert {
            target: crate::content::commands::TextTarget {
                zone: crate::content::commands::TextZone::Body { path: vec![0] },
                from: 0,
                to: 0,
            },
            text: "human ".into(),
            marks: vec![],
        }],
    )
    .unwrap();
    let confirmed = e.state().clone();
    assert!(p.publish(&mut e, c, ready).is_err());
    assert_eq!(e.state(), &confirmed);
    assert_eq!(p.groups[0].decision, Decision::Pending);
    let mut p = Comparison::new(
        before.clone(),
        before,
        SourceSide::After,
        Guard {
            document: "foreign".into(),
            revision: e.guard().revision,
        },
    )
    .unwrap();
    assert!(p.prepare(&e, &[0]).is_err());
    assert!(p.refuse(&[0]).is_err());
}

#[test]
fn images_and_their_immutable_bytes_travel_together_and_survive_undo() {
    let before = archive(&["base", ""]);
    let mut after = before.clone();
    let image=crate::content::commands::ImageResource::prepare("svg",br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2"/></svg>"#.to_vec()).unwrap();
    after.assets.insert(image.path.clone(), image.data.into());
    after.document.content.insert(
        1,
        serde_json::from_value(
            json!({"type":"figure","attrs":{"src":image.path,"alt":"A square"}}),
        )
        .unwrap(),
    );
    let (mut e, c) = engine(&before);
    let mut p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    let report = p.report(e.guard());
    assert_eq!(report.groups.len(), 1);
    assert_eq!(report.files.len(), 1);
    assert!(report.files[0].before.is_none());
    let ready = p.prepare(&e, &[0]).unwrap();
    let pub_ = p.publish(&mut e, c, ready).unwrap();
    assert_eq!(pub_.resources.len(), 1);
    assert_eq!(e.state(), &after.state());
    e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
    assert_eq!(e.state(), &before.state());
    assert!(e.assets().contains_key(&image.path));
    e.history(&e.guard().clone(), c, Direction::Redo).unwrap();
    assert_eq!(e.state(), &after.state());
}
#[test]
fn resource_collisions_fail_without_replacing_the_pool_or_advancing_decisions() {
    let mut before = archive(&["base", ""]);
    let mut after = before.clone();
    let svg = |size| {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="2"/>"#)
            .into_bytes()
    };
    before
        .assets
        .insert("images/shared.svg".into(), svg(1).into());
    after
        .assets
        .insert("images/shared.svg".into(), svg(2).into());
    let (e, _) = engine(&before);
    let p = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    let guard = e.guard().clone();
    let bytes = e.update();
    assert!(p.prepare(&e, &[0]).is_err());
    assert_eq!(e.guard(), &guard);
    assert_eq!(e.update(), bytes);
    assert_eq!(p.groups[0].decision, Decision::Pending);
}
#[test]
fn one_note_is_always_applied_with_its_text_and_identical_snapshots_have_no_groups() {
    let before = archive(&["Text", ""]);
    let mut after = before.clone();
    after.document.content[0].content.push(
        serde_json::from_value(json!({"type":"footnote","attrs":{"text":"A source é😀"}})).unwrap(),
    );
    let (mut e, c) = engine(&before);
    let mut p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.groups.len(), 1);
    let ready = p.prepare(&e, &[0]).unwrap();
    p.publish(&mut e, c, ready).unwrap();
    assert_eq!(e.state(), &after.state());
    let p = Comparison::new(before.clone(), before, SourceSide::After, e.guard().clone()).unwrap();
    assert!(p.report(e.guard()).groups.is_empty());
}

#[test]
fn prepared_decisions_are_bound_to_their_frozen_pair_even_under_the_same_guard() {
    let before = archive(&["a", ""]);
    let after = archive(&["b", ""]);
    let (mut e, c) = engine(&before);
    let one = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    let mut other = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    let ready = one.prepare(&e, &[0]).unwrap();
    let bytes = e.update();
    assert!(other.publish(&mut e, c, ready).is_err());
    assert_eq!(e.update(), bytes);
    assert_eq!(other.groups[0].decision, Decision::Pending);
}
#[test]
fn unchanged_branches_keep_selection_and_typing_styles_through_a_structural_reprise() {
    use crate::content::{
        commands::{Style, TextZone},
        editing::{Selection, TextPoint},
    };
    let before = archive(&["first", "stable", ""]);
    let after = archive(&["inserted", "first", "stable", ""]);
    let (mut e, c) = engine(&before);
    let point = TextPoint {
        zone: TextZone::Body { path: vec![1] },
        offset: 2,
    };
    let selection = Selection::Text {
        anchor: point.clone(),
        head: point,
    };
    e.set_context(
        &e.guard().clone(),
        c,
        Some(selection.clone()),
        vec![Style::Italic],
    )
    .unwrap();
    let mut plan = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    let ready = plan.prepare(&e, &[0]).unwrap();
    plan.publish(&mut e, c, ready).unwrap();
    let context = e.context(&e.guard().clone(), c).unwrap();
    assert_eq!(context.typing_styles, vec![Style::Italic]);
    assert!(!context.selection_lost);
    let point = TextPoint {
        zone: TextZone::Body { path: vec![2] },
        offset: 2,
    };
    assert_eq!(
        context.selection,
        Some(Selection::Text {
            anchor: point.clone(),
            head: point
        })
    );
    e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
    assert_eq!(
        e.context(&e.guard().clone(), c).unwrap().selection,
        Some(selection)
    );
}

#[test]
fn article_frontmatter_citations_and_bibliography_are_published_and_undone_as_one_coherent_group() {
    use crate::{
        bibliography::ReferenceFile,
        content::editorial::RichText,
        document_types::{DocumentType, Metadata},
    };
    let mut before = archive(&["Article", ""]);
    before.metadata = Metadata::demo(DocumentType::Article);
    before.template = DocumentType::Article.default_model();
    before.model_id = "article.model.working".into();
    before.dependencies = crate::packages::for_template(&before.template);
    let mut after = before.clone();
    let Metadata::Article(metadata) = &mut after.metadata else {
        unreachable!()
    };
    metadata
        .affiliations
        .push(crate::document_types::article::Affiliation {
            id: "official".into(),
            text: "Official free-text affiliation".into(),
        });
    metadata.localizations[0].summary = RichText::plain("A revised abstract é😀");
    metadata.references.push(ReferenceFile {
        id: "local".into(),
        name: "sources.bib".into(),
        contents: "@book{sample, author={Martin, Camille}, title={A source}, year={2020}}".into(),
    });
    after.document.content[0].content.push(serde_json::from_value(json!({"type":"citation","attrs":{"items":[{"file_id":"local","key":"sample","supplement":""}],"form":"normal"}})).unwrap());
    let (mut e, c) = engine(&before);
    let mut plan = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    let report = plan.report(e.guard());
    assert_eq!(report.groups.len(), 1);
    assert!(report.groups[0].linked);
    assert!(report.files.iter().any(|f| f.kind == "bibliography"));
    let ready = plan.prepare(&e, &[0]).unwrap();
    plan.publish(&mut e, c, ready).unwrap();
    assert_eq!(e.state(), &after.state());
    e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
    assert_eq!(e.state(), &before.state());
    e.history(&e.guard().clone(), c, Direction::Redo).unwrap();
    assert_eq!(e.state(), &after.state());
}
