use super::{
    tests::{archive, engine},
    *,
};
use crate::content::editing::Direction;

#[test]
fn all_choices_are_pure_until_one_undoable_publication_in_both_contexts() {
    let before = archive(&["Old one", "anchor", "Old two", ""]);
    let after = archive(&["New one", "anchor", "New two", ""]);
    for source in [SourceSide::Before, SourceSide::After] {
        let (mut e, c) = engine(&before);
        let mut plan =
            Comparison::new(before.clone(), after.clone(), source, e.guard().clone()).unwrap();
        let initial = e.update();
        let prepared = plan
            .prepare_choices(&e, &[SourceSide::Before, SourceSide::After])
            .unwrap();
        assert_eq!(e.update(), initial);
        assert!(
            plan.report(e.guard())
                .groups
                .iter()
                .all(|g| g.decision == Decision::Pending)
        );
        plan.publish(&mut e, c, prepared).unwrap();
        assert_eq!(e.state().document.content[0], before.document.content[0]);
        assert_eq!(e.state().document.content[2], after.document.content[2]);
        assert_eq!(e.context(&e.guard().clone(), c).unwrap().undo_steps, 1);
        e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
        assert_eq!(e.state(), &before.state());
        e.history(&e.guard().clone(), c, Direction::Redo).unwrap();
        assert_eq!(e.state().document.content[2], after.document.content[2]);
    }
}
#[test]
fn composition_uses_the_complete_right_state_and_reverse_ranges_without_index_drift() {
    let before = archive(&["A", "anchor", "B", ""]);
    let after = archive(&["A", "extra", "anchor", "New B", ""]);
    let current = archive(&["Unrelated current", ""]);
    let (mut e, c) = engine(&current);
    let mut plan =
        Comparison::new(before, after.clone(), SourceSide::Before, e.guard().clone()).unwrap();
    let prepared = plan
        .prepare_choices(&e, &[SourceSide::Before, SourceSide::After])
        .unwrap();
    plan.publish(&mut e, c, prepared).unwrap();
    assert_eq!(
        e.state().document,
        archive(&["A", "anchor", "New B", ""]).document
    );
    e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
    assert_eq!(e.state(), &current.state());
    assert_eq!(plan.after().state(), after.state());
}
#[test]
fn invalid_or_stale_choices_never_publish_and_noops_do_not_add_history() {
    let before = archive(&["Old", ""]);
    let after = archive(&["New", ""]);
    let (mut e, c) = engine(&after);
    let mut plan = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    assert!(plan.prepare_choices(&e, &[]).is_err());
    assert!(plan.prepare_choices(&e, &[SourceSide::After; 2]).is_err());
    let prepared = plan.prepare_choices(&e, &[SourceSide::After]).unwrap();
    let duplicate = plan.prepare_choices(&e, &[SourceSide::After]).unwrap();
    assert!(!plan.publish(&mut e, c, prepared).unwrap().changed);
    assert_eq!(e.context(&e.guard().clone(), c).unwrap().undo_steps, 0);
    assert!(plan.publish(&mut e, c, duplicate).is_err());
}
