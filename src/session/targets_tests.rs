use super::*;
use crate::document_types::DocumentType;

fn session() -> Session {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Été\n\nNoël".into();
    let document = Document::from_source(&stored.source).unwrap();
    Session::new(stored, document, None, 1)
}

fn selection() -> Selection {
    Selection {
        primary: Caret {
            index: 1,
            prefer_next_row: true,
        },
        secondary: Caret {
            index: 7,
            prefer_next_row: false,
        },
    }
}

#[test]
fn stale_targets_are_rejected_after_edits_and_even_after_undo_restores_their_zones() {
    let mut session = session();
    let selection = selection();
    let target = session.selection_target(selection).unwrap();
    assert_eq!(session.resolve_target(target), Ok((7, 1)));
    let before = session.snapshot(selection);
    session
        .document
        .view
        .insert_text(" !", session.document.view.len());
    session.remember(before, selection, false).unwrap();
    assert_eq!(
        session.resolve_target(target),
        Err(TargetError::StaleRevision)
    );
    assert_eq!(session.history(true, Selection::default()), Some(selection));
    assert_eq!(
        session.resolve_target(target),
        Err(TargetError::StaleRevision)
    );
    let refreshed = session.selection_target(selection).unwrap();
    assert_eq!(refreshed.selection, target.selection);
    assert_ne!(refreshed.guard.revision, target.guard.revision);
    assert_eq!(session.resolve_target(refreshed), Ok((7, 1)));
}

#[test]
fn another_session_is_rejected_even_if_it_has_the_same_content_and_revision() {
    let first = session();
    let stored = first.stored_document();
    // Même une copie en mémoire des blocs ne partage pas l’identité de session.
    let second = Session::new(stored, first.document.clone(), None, first.revision);
    let target = first.selection_target(selection()).unwrap();
    assert_ne!(first.target_guard(), second.target_guard());
    assert_eq!(
        second.resolve_target(target),
        Err(TargetError::ForeignDocument)
    );
    let reopened = Session::new(
        first.stored_document(),
        Document::from_source(first.document.source()).unwrap(),
        None,
        1,
    );
    assert_eq!(
        reopened.resolve_target(target),
        Err(TargetError::ForeignDocument)
    );
}

#[test]
fn selection_reads_do_not_invalidate_guards_but_metadata_edits_do() {
    let mut session = session();
    let target = session.selection_target(selection()).unwrap();
    assert!(session.selection_target(Selection::default()).is_ok());
    assert_eq!(session.target_guard(), target.guard);
    let before = session.snapshot(selection());
    session.metadata.courrier_mut().objet = "Nouvel objet".into();
    session
        .record(before, selection(), EditGroup::Metadata(1))
        .unwrap();
    assert_eq!(
        session.resolve_target(target),
        Err(TargetError::StaleRevision)
    );
    assert_eq!(
        session.history(true, Selection::default()),
        Some(selection())
    );
    assert_eq!(
        session.selection_target(selection()).unwrap().selection,
        target.selection
    );
}

#[test]
fn no_op_keeps_a_previously_read_target_valid_and_invalid_external_positions_fail() {
    let mut session = session();
    let target = session.selection_target(selection()).unwrap();
    let before = session.snapshot(selection());
    session.remember(before, selection(), false).unwrap();
    assert_eq!(session.resolve_target(target), Ok((7, 1)));
    let bad = Selection {
        primary: Caret {
            index: 1000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        session.selection_target(bad),
        Err(TargetError::InvalidPosition)
    );
    assert_eq!(session.revision, 1);
    assert!(session.undo.is_empty());
}
