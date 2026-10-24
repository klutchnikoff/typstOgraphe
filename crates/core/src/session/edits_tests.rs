use super::*;
use crate::{
    document::{BlockKind, Mark},
    document_types::DocumentType,
};

fn session(source: &str) -> Session {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = source.into();
    Session::new(stored, Document::from_source(source).unwrap(), None, 1)
}
fn selection(range: Range<usize>) -> Selection {
    Selection {
        primary: Caret {
            index: range.end,
            ..Default::default()
        },
        secondary: Caret {
            index: range.start,
            ..Default::default()
        },
    }
}

#[test]
fn replacements_refuse_partial_roles_and_seams_without_any_publication() {
    for (source, range) in [
        ("/ Mot: Définition", 2..6),
        ("/ Mot: Définition", 3..4),
        ("#quote(block: true, attribution: [Auteur])[Texte]", 4..7),
        ("#quote(block: true, attribution: [Auteur])[Texte]", 5..6),
        ("#figure(image(\"images/parc.svg\"), caption: [Cap])", 0..1),
        (
            "#figure(image(\"images/parc.svg\"), caption: [Cap])\n\nSuite",
            2..7,
        ),
        (
            "Avant\n\n#quote(block: true, attribution: [Auteur])[Texte]",
            4..9,
        ),
    ] {
        let mut session = session(source);
        let before = session.stored_document();
        let guard = session.target_guard();
        let changed_at = session.changed_at;
        let target = session.selection_target(selection(range)).unwrap();
        assert_eq!(
            session.replace_text(target, "Remplacement"),
            Err(EditError::ProtectedBoundary),
            "{source}"
        );
        assert_eq!(session.stored_document(), before);
        assert_eq!(session.target_guard(), guard);
        assert_eq!(session.changed_at, changed_at);
        assert!(session.undo.is_empty() && session.redo.is_empty());
        assert!(!session.is_dirty());
        assert!(session.resolve_target(target).is_ok());
    }
}

#[test]
fn whole_blocks_and_select_all_delete_normally_and_undo_restores_resources_and_targets() {
    for source in [
        "/ Mot: Définition",
        "#quote(block: true, attribution: [Auteur])[Texte]",
        "#figure(image(\"images/parc.svg\"), caption: [Cap])",
        "= Titre\n\n/ Mot: Définition\n\n#quote(block: true)[Texte]\n\n#figure(image(\"images/parc.svg\"))",
    ] {
        let mut session = session(source);
        session.document.assets.insert(
            "images/parc.svg".into(),
            include_bytes!("../../../../fixtures/parc.svg")
                .as_slice()
                .into(),
        );
        let before = session.stored_document();
        let original = selection(0..session.document.view.len());
        let target = session.selection_target(original).unwrap();
        let after = session.replace_text(target, "").unwrap();
        assert_eq!(session.document.view.text, "");
        assert_eq!(session.document.view.blocks, [BlockKind::Paragraph]);
        assert!(session.document.view.images.is_empty());
        assert_eq!(session.document.assets, before.assets);
        assert_eq!(session.undo.len(), 1);
        assert_eq!(session.history(true, after), Some(original));
        assert_eq!(session.stored_document(), before);
        assert_eq!(
            session.selection_target(original).unwrap().selection,
            target.selection
        );
        session.history(false, original).unwrap();
        assert_eq!(session.document.source(), "");
    }
}

#[test]
fn clearing_annotations_does_not_remove_their_parent_and_paragraph_joins_remain_free() {
    for (source, range, kind) in [
        (
            "#figure(image(\"images/parc.svg\"), caption: [Cap])",
            1..4,
            BlockKind::Image(1),
        ),
        (
            "#quote(block: true, attribution: [Auteur])[Texte]",
            6..12,
            BlockKind::Quote,
        ),
        ("/ Mot: Définition", 4..14, BlockKind::Description),
    ] {
        let mut session = session(source);
        let target = session.selection_target(selection(range)).unwrap();
        session.replace_text(target, "").unwrap();
        assert_eq!(session.document.view.blocks[0], kind);
        assert_eq!(
            Document::from_source(session.document.source())
                .unwrap()
                .view,
            session.document.view
        );
    }
    let mut session = session("Été\n\nNoël");
    let target = session.selection_target(selection(2..6)).unwrap();
    session.replace_text(target, "_").unwrap();
    assert_eq!(session.document.view.text, "Ét_oël");
    assert!(session.document.source().contains("\\_"));
}

#[test]
fn a_failed_multi_replacement_rolls_back_even_the_valid_part_and_preserves_redo() {
    let mut session = session("/ Mot: Texte\n\nSuite");
    let at = session.document.view.len();
    let target = session.selection_target(selection(at..at)).unwrap();
    let after = session.replace_text(target, " !").unwrap();
    session.history(true, after).unwrap();
    let before = session.stored_document();
    let revision = session.revision;
    let ranges = [2..5, 12..14];
    assert_eq!(
        session.replace_ranges(Selection::default(), &ranges, "X"),
        Err(EditError::ProtectedBoundary)
    );
    assert_eq!(session.stored_document(), before);
    assert_eq!(session.revision, revision);
    assert_eq!(session.redo.len(), 1);
    assert_eq!(
        session.replace_ranges(Selection::default(), &[5..8, 2..5], "X"),
        Err(EditError::InvalidSelection)
    );
}

#[test]
fn stale_prepared_edits_never_rollback_an_intervening_commit() {
    let mut session = session("Texte");
    let guard = session.target_guard();
    let before = session.snapshot(Selection::default());
    let mut view = before.document.view.clone();
    view.insert_text("Préparé", 0);
    session.metadata.courrier_mut().objet = "Nouvel objet".into();
    session
        .record(before.clone(), Selection::default(), EditGroup::Discrete)
        .unwrap();
    let committed = session.stored_document();
    let revision = session.revision;
    assert_eq!(
        session.commit_projection(guard, before, view, Selection::default(), EditGroup::Body),
        Err(EditError::StaleDraft)
    );
    assert_eq!(session.stored_document(), committed);
    assert_eq!(session.revision, revision);
    assert_eq!(session.undo.len(), 1);
}

#[test]
fn malformed_drafts_restore_all_document_fields_before_reporting_failure() {
    let mut session = session("Texte");
    let before = session.snapshot(Selection::default());
    let stored = session.stored_document();
    session.document.view.styles.pop();
    session
        .document
        .assets
        .insert("images/new.svg".into(), b"new".as_slice().into());
    session.metadata.courrier_mut().objet = "Non publié".into();
    session.template.push_str("\n// Non publié");
    assert_eq!(
        session.record(before, Selection::default(), EditGroup::Discrete),
        Err(EditError::InvalidProjection)
    );
    assert_eq!(session.stored_document(), stored);
    assert_eq!(session.revision, 1);
    assert!(session.undo.is_empty());
}

#[test]
fn typing_context_is_not_a_document_edit_and_is_applied_to_the_next_insertion() {
    let mut session = session("Texte");
    let caret = selection(5..5);
    session
        .edit(caret, |view, range| view.toggle_mark(range, Mark::Bold))
        .unwrap();
    assert_eq!(session.revision, 1);
    assert!(!session.is_dirty());
    assert!(session.undo.is_empty());
    let target = session.selection_target(caret).unwrap();
    let after = session.replace_text(target, " été").unwrap();
    assert_eq!(after.range(), 9..9);
    assert!(
        session.document.view.styles[5..]
            .iter()
            .all(|style| style.bold)
    );
    assert_eq!(session.undo.len(), 1);
    session.history(true, after).unwrap();
    assert_eq!(session.document.source(), "Texte");
}

#[test]
fn single_line_roles_reject_pasted_structural_characters_without_losing_selected_text() {
    for (source, range) in [
        ("/ Mot: Texte", 0..3),
        ("#quote(block: true, attribution: [Auteur])[Texte]", 6..12),
        ("#figure(image(\"images/parc.svg\"), caption: [Cap])", 1..4),
    ] {
        for inserted in ["Deux\nlignes", "Avec\ttabulation", "Objet \u{fffc}"] {
            let mut session = session(source);
            let before = session.stored_document();
            let target = session.selection_target(selection(range.clone())).unwrap();
            assert_eq!(
                session.replace_text(target, inserted),
                Err(EditError::InvalidText)
            );
            assert_eq!(session.stored_document(), before);
            assert!(session.undo.is_empty());
        }
    }
}

#[test]
fn stale_targets_foreign_snapshots_and_bad_ranges_cannot_enter_the_commit_path() {
    let mut current = session("Un texte");
    let other = session("Autre document");
    let old_target = current.selection_target(selection(0..2)).unwrap();
    let before = current.snapshot(Selection::default());
    current.metadata.courrier_mut().objet = "Publié".into();
    current
        .record(before, Selection::default(), EditGroup::Discrete)
        .unwrap();
    let published = current.stored_document();
    let revision = current.revision;
    assert_eq!(
        current.replace_text(old_target, "X"),
        Err(EditError::Target(TargetError::StaleRevision))
    );
    assert_eq!(
        current.commit_projection(
            current.target_guard(),
            other.snapshot(Selection::default()),
            other.document.view.clone(),
            Selection::default(),
            EditGroup::Discrete
        ),
        Err(EditError::StaleDraft)
    );
    assert_eq!(
        current.replace_ranges(Selection::default(), std::slice::from_ref(&(100..101)), "X"),
        Err(EditError::InvalidSelection)
    );
    assert!(current.stored_document() == published);
    assert_eq!(current.revision, revision);
    assert_eq!(current.undo.len(), 1);
}

#[test]
fn object_insertion_cannot_split_a_caption_description_or_quote_implicitly() {
    for source in [
        "/ Mot: Texte",
        "#quote(block: true, attribution: [Auteur])[Texte]",
        "#figure(image(\"images/parc.svg\"), caption: [Cap])",
    ] {
        let mut session = session(source);
        let before = session.stored_document();
        let result = session.edit(selection(2..2), |view, range| {
            view.insert_image(
                range.start,
                crate::document::ImageBlock {
                    path: "images/autre.svg".into(),
                    alt: String::new(),
                },
            );
        });
        assert_eq!(result, Err(EditError::ProtectedBoundary));
        assert!(session.stored_document() == before);
        assert!(session.undo.is_empty());
    }
}

#[test]
fn replacing_a_whole_heading_keeps_its_kind_but_select_all_delete_leaves_a_paragraph() {
    let mut session = session("= *Titre*");
    let target = session.selection_target(selection(0..5)).unwrap();
    session.replace_text(target, "Autre").unwrap();
    assert_eq!(session.document.view.blocks, [BlockKind::Heading(1)]);
    assert!(session.document.view.styles.iter().all(|style| style.bold));
    let target = session.selection_target(selection(0..5)).unwrap();
    let after = session.replace_text(target, "").unwrap();
    assert_eq!(session.document.view.blocks, [BlockKind::Paragraph]);
    assert_eq!(session.document.source(), "");
    session.history(true, after).unwrap();
    assert_eq!(session.document.view.blocks, [BlockKind::Heading(1)]);
    assert_eq!(session.document.view.text, "Autre");
}
