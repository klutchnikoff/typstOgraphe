use super::*;
use crate::engine::*;
fn review(source: &str, steps: Vec<(&str, serde_json::Value)>) -> ProposalReview {
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source = source.into();
    let mut e = Engine::from_stored(stored, None).unwrap();
    let context = e.create_edit_context(e.target_guard(), None).unwrap();
    let proposal = e
        .prepare_proposal(
            &context,
            BatchParams {
                guard: e.target_guard(),
                save: None,
                steps: steps
                    .into_iter()
                    .map(|(id, params)| Call {
                        id: id.to_owned().try_into().unwrap(),
                        params,
                    })
                    .collect(),
            },
        )
        .unwrap();
    ProposalReview {
        guard: e.target_guard(),
        id: "test".into(),
        summary: String::new(),
        before: e.stored_document(),
        after: proposal.document().clone(),
        original: e.document().clone(),
        candidate: proposal.candidate_document().clone(),
    }
}
#[test]
fn unicode_replacements_and_identical_text_formatting_are_visible() {
    let r = review(
        "にほん Été 👩‍💻",
        vec![(
            "text.insert",
            serde_json::json!({"target":{"kind":"all"},"text":"日本 Noël 👩‍💻"}),
        )],
    );
    let diff = Changes::new(&r);
    assert!(!diff.added.is_empty() && !diff.removed.is_empty());
    assert!(
        diff.added
            .iter()
            .all(|range| range.end <= r.candidate.view.len())
    );
    let r = review(
        "Un texte",
        vec![(
            "text.bold",
            serde_json::json!({"target":{"kind":"all"},"value":"on"}),
        )],
    );
    assert_eq!(r.original.view.text, r.candidate.view.text);
    let diff = Changes::new(&r);
    assert_eq!(diff.added, vec![0..8]);
    assert_eq!(diff.removed, vec![0..8]);
}
#[test]
fn link_destinations_and_empty_paragraph_structure_changes_are_not_hidden() {
    let r = review(
        "#link(\"https://example.org/old\")[Exemple]",
        vec![(
            "text.link",
            serde_json::json!({"target":{"kind":"all"},"url":"https://example.org/new"}),
        )],
    );
    let diff = Changes::new(&r);
    assert!(
        diff.rows
            .iter()
            .any(|row| row.before.ends_with("/old") && row.after.ends_with("/new"))
    );
    let r = review("Texte", vec![("block.heading_1", serde_json::json!({}))]);
    assert!(!Changes::new(&r).added.is_empty());
}
