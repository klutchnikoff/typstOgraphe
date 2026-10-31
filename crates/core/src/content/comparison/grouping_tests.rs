use super::{
    tests::{archive, engine},
    *,
};
use crate::content::{
    commands::{Command, TextTarget, TextZone},
    editing::Direction,
};
use serde_json::json;

fn insert(path: &[usize]) -> Command {
    Command::Insert {
        target: TextTarget {
            zone: TextZone::Body { path: path.into() },
            from: 0,
            to: 0,
        },
        text: "New ".into(),
        marks: vec![],
    }
}
fn revisions(mut before: Archive, commands: &[Command]) -> (Archive, Archive) {
    let (mut e, c) = engine(&before);
    before.yjs = e.update();
    e.execute(&e.guard().clone(), c, commands).unwrap();
    let mut after = before.clone();
    let state = e.state();
    after.document = state.document.clone();
    after.metadata = state.settings.metadata.clone();
    after.template = state.settings.template.clone();
    after.model_id = state.settings.model_id.clone();
    after.yjs = e.update();
    (before, after)
}
#[test]
fn neighboring_paragraphs_can_be_applied_and_refused_independently_in_both_directions() {
    let (before, after) = revisions(
        archive(&["First", "Second", ""]),
        &[insert(&[0]), insert(&[1])],
    );
    for source in [SourceSide::Before, SourceSide::After] {
        let (original, incoming) = match source {
            SourceSide::Before => (&after, &before),
            SourceSide::After => (&before, &after),
        };
        let (mut e, c) = engine(original);
        let mut p =
            Comparison::new(before.clone(), after.clone(), source, e.guard().clone()).unwrap();
        assert_eq!(p.groups.len(), 2);
        assert_eq!(
            (p.groups[1].before.clone(), p.groups[1].after.clone()),
            (1..2, 1..2)
        );
        assert!(
            p.groups
                .iter()
                .all(|g| matches!(g.section, Section::Body) && !g.linked)
        );
        let ready = p.prepare(&e, &[1]).unwrap();
        p.publish(&mut e, c, ready).unwrap();
        p.refuse(&[0]).unwrap();
        assert_eq!(e.state().document.content[0], original.document.content[0]);
        assert_eq!(e.state().document.content[1], incoming.document.content[1]);
        e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
        assert_eq!(e.state(), &original.state());
        e.history(&e.guard().clone(), c, Direction::Redo).unwrap();
        assert_eq!(e.state().document.content[1], incoming.document.content[1]);
        assert_eq!(p.before().state(), before.state());
        assert_eq!(p.after().state(), after.state());
    }
}
#[test]
fn neighboring_paragraph_decisions_have_separate_undo_steps_in_either_order() {
    let (before, after) = revisions(
        archive(&["First", "Second", ""]),
        &[insert(&[0]), insert(&[1])],
    );
    for order in [[0, 1], [1, 0]] {
        let (mut e, c) = engine(&before);
        let mut p = Comparison::new(
            before.clone(),
            after.clone(),
            SourceSide::After,
            e.guard().clone(),
        )
        .unwrap();
        for id in order {
            let ready = p.prepare(&e, &[id]).unwrap();
            p.publish(&mut e, c, ready).unwrap();
        }
        assert_eq!(e.state(), &after.state());
        assert_eq!(e.context(&e.guard().clone(), c).unwrap().undo_steps, 2);
        e.history(&e.guard().clone(), c, Direction::Undo).unwrap();
        assert_eq!(
            e.state().document.content[order[0]],
            after.document.content[order[0]]
        );
        assert_eq!(
            e.state().document.content[order[1]],
            before.document.content[order[1]]
        );
    }
}
#[test]
fn widgets_stay_whole_even_when_their_neighbor_can_be_decided_separately() {
    let mut before = archive(&[""]);
    before.document = serde_json::from_value(json!({"type":"doc","content":[
        {"type":"quote","content":[{"type":"paragraph","content":[{"type":"text","text":"Quote"}]},{"type":"quoteAttribution","content":[{"type":"text","text":"Source"}]}]},
        {"type":"bulletList","content":[{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"First"}]}]},{"type":"listItem","content":[{"type":"paragraph","content":[{"type":"text","text":"Second"}]}]}]},
        {"type":"paragraph"}
    ]})).unwrap();
    let (before, after) = revisions(
        before,
        &[
            insert(&[0, 0]),
            insert(&[0, 1]),
            insert(&[1, 0, 0]),
            insert(&[1, 1, 0]),
        ],
    );
    let (mut e, c) = engine(&before);
    let mut p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.groups.len(), 2);
    assert!(
        p.groups
            .iter()
            .all(|g| g.linked && g.before.len() == 1 && g.after.len() == 1)
    );
    let ready = p.prepare(&e, &[0]).unwrap();
    p.publish(&mut e, c, ready).unwrap();
    assert_eq!(e.state().document.content[0], after.document.content[0]);
    assert_eq!(e.state().document.content[1], before.document.content[1]);
}
#[test]
fn joins_splits_and_recreated_boundaries_cannot_be_split_by_paragraph_count() {
    use crate::content::commands::{TreeRoot, TreeTarget};
    let target = |index| TreeTarget {
        root: TreeRoot::Body,
        path: vec![index],
    };
    let commands = [
        Command::JoinBlocks {
            left: target(0),
            right: target(1),
        },
        Command::SplitBlock {
            target: TextTarget {
                zone: TextZone::Body { path: vec![1] },
                from: 3,
                to: 3,
            },
        },
    ];
    for commands in [&commands[..1], &commands[..]] {
        let (before, after) = revisions(archive(&["First", "Second", "Third", ""]), commands);
        let (e, _) = engine(&before);
        let p = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
        assert_eq!(p.groups.len(), 1);
        assert!(p.groups[0].linked);
    }
    let (before, mut after) = revisions(
        archive(&["First", "Second", ""]),
        &[insert(&[0]), insert(&[1])],
    );
    after.yjs = crate::content::shared::SharedDocument::from_state(&after.state())
        .unwrap()
        .update();
    let (e, _) = engine(&before);
    let p = Comparison::new(
        before.clone(),
        after.clone(),
        SourceSide::After,
        e.guard().clone(),
    )
    .unwrap();
    assert_eq!(p.groups.len(), 1);
    after.yjs.clear();
    let p = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    assert_eq!(p.groups.len(), 1);
}
#[test]
fn settings_dependencies_override_preserved_paragraph_boundaries() {
    use crate::content::commands::{FieldTarget, OptionName};
    let (before, after) = revisions(
        archive(&["First", "Second", ""]),
        &[
            insert(&[0]),
            insert(&[1]),
            Command::SetField {
                target: FieldTarget::Option {
                    name: OptionName::Landscape,
                },
                value: json!(true),
            },
        ],
    );
    let (e, _) = engine(&before);
    let p = Comparison::new(before, after, SourceSide::After, e.guard().clone()).unwrap();
    assert_eq!(p.groups.len(), 1);
    assert!(matches!(p.groups[0].section, Section::Document));
}
