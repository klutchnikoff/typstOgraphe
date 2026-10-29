use super::*;
use crate::content::{
    commands::{FieldTarget, TextZone, TreeRoot, TreeTarget},
    contract::{Settings, TargetReason},
};
use crate::document_types::MetadataField;
use serde_json::json;
fn open() -> EditingSession {
    let state = State::new(
        serde_json::from_value(json!({"type":"doc","content":[
            {"type":"paragraph","content":[{"type":"text","text":"A😀é"}]},
            {"type":"paragraph","content":[{"type":"text","text":"Second"}]},
            {"type":"paragraph"}
        ]}))
        .unwrap(),
        Settings::default(),
    );
    EditingSession::open(
        Guard {
            document: "contexts".into(),
            revision: 0,
        },
        &SharedDocument::from_state(&state).unwrap().update(),
        Assets::default(),
    )
    .unwrap()
}
fn target(path: &[usize], from: u32, to: u32) -> TextTarget {
    TextTarget {
        zone: TextZone::Body {
            path: path.to_vec(),
        },
        from,
        to,
    }
}
fn ctx(s: &mut EditingSession) -> ContextId {
    s.open_context(&s.guard().clone()).unwrap()
}
fn select(s: &mut EditingSession, c: ContextId, t: TextTarget) {
    s.select(&s.guard().clone(), c, Some(t)).unwrap();
}
fn snapshot(s: &mut EditingSession, c: ContextId) -> ContextSnapshot {
    s.context(&s.guard().clone(), c).unwrap()
}
fn execute(s: &mut EditingSession, c: ContextId, commands: &[Command]) -> Publication {
    s.execute(&s.guard().clone(), c, commands).unwrap()
}
fn insert(s: &mut EditingSession, c: ContextId, text: &str) -> Publication {
    s.insert(&s.guard().clone(), c, text.into()).unwrap()
}
fn history(s: &mut EditingSession, c: ContextId, direction: Direction) -> Publication {
    s.history(&s.guard().clone(), c, direction).unwrap()
}
fn text(s: &EditingSession) -> String {
    s.state()
        .document
        .content
        .iter()
        .flat_map(|n| n.content.iter())
        .filter_map(|n| n.text.as_deref())
        .collect()
}
#[test]
fn contexts_selections_styles_and_lifetimes_never_modify_the_shared_document() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let bytes = s.update();
    let guard = s.guard().clone();
    select(&mut s, a, target(&[0], 3, 3));
    select(&mut s, b, target(&[1], 0, 3));
    s.typing_style(&guard, a, Style::Bold, Switch::On).unwrap();
    s.typing_style(&guard, a, Style::Italic, Switch::On)
        .unwrap();
    assert_eq!(
        snapshot(&mut s, a).typing_styles,
        vec![Style::Bold, Style::Italic]
    );
    assert!(snapshot(&mut s, b).typing_styles.is_empty());
    let before = snapshot(&mut s, a);
    assert!(matches!(
        s.select(&guard, a, Some(target(&[0], 2, 2))),
        Err(Error::InvalidTarget {
            reason: TargetReason::SplitSurrogate
        })
    ));
    assert_eq!(snapshot(&mut s, a), before);
    assert!(
        s.typing_style(
            &guard,
            a,
            Style::Link {
                url: "javascript:bad".into()
            },
            Switch::On
        )
        .is_err()
    );
    assert_eq!(snapshot(&mut s, a), before);
    assert_eq!(s.update(), bytes);
    assert_eq!(s.guard(), &guard);
    assert_eq!(before.undo_steps, 0);
    s.close_context(&guard, a).unwrap();
    assert!(matches!(s.context(&guard,a),Err(Error::EditingContext{reason}) if reason=="closed"));
    assert_ne!(ctx(&mut s), a);
    assert_eq!(
        snapshot(&mut s, b).selection,
        Some(target(&[1], 0, 3).into())
    );
    let foreign = Guard {
        document: "elsewhere".into(),
        revision: 0,
    };
    assert!(matches!(s.context(&foreign, b), Err(Error::WrongDocument)));
}
#[test]
fn typing_uses_only_its_own_styles_and_maps_the_other_participant_caret() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    select(&mut s, a, target(&[0], 1, 1));
    select(&mut s, b, target(&[0], 4, 4));
    s.typing_style(&s.guard().clone(), a, Style::Bold, Switch::On)
        .unwrap();
    insert(&mut s, a, "é日本");
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(target(&[0], 4, 4).into())
    );
    assert_eq!(
        snapshot(&mut s, b).selection,
        Some(target(&[0], 7, 7).into())
    );
    insert(&mut s, b, "B");
    assert!(s.state().document.content[0].content.iter().any(|n|n.text.as_deref()==Some("é日本") && n.marks.iter().any(|m|m.kind=="bold")));
    assert!(
        s.state().document.content[0].content.iter().any(|n| n
            .text
            .as_deref()
            .is_some_and(|t| t.ends_with('B'))
            && n.marks.is_empty())
    );
    history(&mut s, a, Direction::Undo);
    assert!(text(&s).contains('B'));
    assert!(!text(&s).contains("日本"));
    history(&mut s, a, Direction::Redo);
    assert!(text(&s).contains("日本"));
    assert!(text(&s).contains('B'));
    assert_eq!(snapshot(&mut s, b).undo_steps, 1);
}
#[test]
fn durable_yrs_replicas_preserve_redone_links_across_multiple_undo_and_redo_steps() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    select(&mut s, a, target(&[0], 1, 1));
    insert(&mut s, a, "x");
    execute(
        &mut s,
        a,
        &[Command::Delete {
            target: target(&[0], 1, 2),
        }],
    );
    select(&mut s, b, target(&[1], 6, 6));
    insert(&mut s, b, " distant");
    history(&mut s, a, Direction::Undo);
    assert!(text(&s).starts_with("Ax😀é"));
    history(&mut s, a, Direction::Undo);
    assert!(text(&s).starts_with("A😀é"));
    history(&mut s, a, Direction::Redo);
    assert!(text(&s).starts_with("Ax😀é"));
    history(&mut s, a, Direction::Redo);
    assert!(text(&s).starts_with("A😀é"));
    assert!(text(&s).ends_with("Second distant"));
}
#[test]
fn multizone_batch_has_one_undo_and_remote_changes_do_not_clear_redo() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let before = s.state().clone();
    execute(
        &mut s,
        a,
        &[
            Command::Bold {
                target: target(&[0], 0, 4),
                value: Switch::On,
            },
            Command::SetField {
                target: FieldTarget::Metadata {
                    field: MetadataField::LetterSubject,
                },
                value: json!("Objet changé"),
            },
        ],
    );
    assert_eq!(snapshot(&mut s, a).undo_steps, 1);
    history(&mut s, a, Direction::Undo);
    assert_eq!(s.state(), &before);
    select(&mut s, b, target(&[1], 6, 6));
    insert(&mut s, b, "B");
    assert_eq!(snapshot(&mut s, a).redo_steps, 1);
    history(&mut s, a, Direction::Redo);
    assert!(text(&s).ends_with("SecondB"));
    assert_ne!(s.state().settings.metadata, before.settings.metadata);
    let guard = s.guard().clone();
    let bytes = s.update();
    let invalid = [
        Command::Delete {
            target: target(&[1], 0, 1),
        },
        Command::Delete {
            target: target(&[0], 2, 2),
        },
    ];
    assert!(s.execute(&guard, a, &invalid).is_err());
    assert_eq!(s.update(), bytes);
    assert_eq!(s.guard(), &guard);
    assert_eq!(snapshot(&mut s, a).undo_steps, 1);
    assert!(s.execute(&before_guard(), a, &[]).is_err());
}
fn before_guard() -> Guard {
    Guard {
        document: "contexts".into(),
        revision: 0,
    }
}
#[test]
fn selection_follows_block_identity_and_never_retargets_a_replacement() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    select(&mut s, a, target(&[1], 1, 4));
    execute(
        &mut s,
        b,
        &[Command::InsertBlock {
            parent: TreeTarget {
                root: TreeRoot::Body,
                path: vec![],
            },
            index: 0,
            node: serde_json::from_value(
                json!({"type":"paragraph","content":[{"type":"text","text":"Before"}]}),
            )
            .unwrap(),
        }],
    );
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(target(&[2], 1, 4).into())
    );
    let mut same = s.state().document.content[2].clone();
    same.kind = "heading".into();
    same.attrs = Some(json!({"level":2}));
    execute(
        &mut s,
        b,
        &[Command::ReplaceBlock {
            target: TreeTarget {
                root: TreeRoot::Body,
                path: vec![2],
            },
            node: same,
        }],
    );
    let snap = snapshot(&mut s, a);
    assert!(snap.selection_lost && snap.selection.is_none());
    assert!(s.insert(&s.guard().clone(), a, "unsafe".into()).is_err());
}
#[test]
fn plain_metadata_never_receives_inherited_rich_styles() {
    let mut s = open();
    let a = ctx(&mut s);
    select(&mut s, a, target(&[0], 0, 0));
    s.typing_style(&s.guard().clone(), a, Style::Bold, Switch::On)
        .unwrap();
    s.select(
        &s.guard().clone(),
        a,
        Some(TextTarget {
            zone: TextZone::Metadata {
                field: MetadataField::LetterSubject,
            },
            from: 0,
            to: 0,
        }),
    )
    .unwrap();
    assert!(
        s.typing_style(&s.guard().clone(), a, Style::Italic, Switch::On)
            .is_err()
    );
    insert(&mut s, a, "Objet");
    assert_eq!(snapshot(&mut s, a).typing_styles, vec![Style::Bold]);
    history(&mut s, a, Direction::Undo);
    assert_eq!(snapshot(&mut s, a).undo_steps, 0);
}
#[test]
fn undo_cannot_hide_later_contributions_inside_an_inserted_container() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    execute(
        &mut s,
        a,
        &[Command::InsertTable {
            parent: TreeTarget {
                root: TreeRoot::Body,
                path: vec![],
            },
            index: 1,
            rows: 1,
            columns: 1,
            header: false,
        }],
    );
    select(&mut s, b, target(&[1, 0, 0, 0, 0], 0, 0));
    insert(&mut s, b, "Contribution de B");
    let guard = s.guard().clone();
    let bytes = s.update();
    assert!(
        matches!(s.history(&guard,a,Direction::Undo),Err(Error::EditingContext{reason}) if reason=="history_conflict")
    );
    assert_eq!(s.update(), bytes);
    assert_eq!(s.guard(), &guard);
    let draft = s.rejected_draft(&guard, a).unwrap().unwrap();
    assert_eq!(draft.base, guard);
    assert!(!draft.update.is_empty());
    assert!(snapshot(&mut s, a).pending);
    assert!(!snapshot(&mut s, b).pending);
    assert!(
        matches!(s.execute(&guard,a,&[]),Err(Error::EditingContext{reason}) if reason=="pending_draft")
    );
}
#[test]
fn empty_histories_and_noop_commands_do_not_create_revisions_or_undo_steps() {
    let mut s = open();
    let a = ctx(&mut s);
    let original = s.update();
    assert!(!history(&mut s, a, Direction::Undo).changed);
    assert!(!history(&mut s, a, Direction::Redo).changed);
    assert!(!execute(&mut s, a, &[]).changed);
    assert_eq!(s.update(), original);
    assert_eq!(s.guard().revision, 0);
    assert_eq!(snapshot(&mut s, a).undo_steps, 0);
}

#[test]
fn cell_caption_and_annotation_contexts_keep_their_own_boundaries() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    execute(
        &mut s,
        a,
        &[Command::InsertTable {
            parent: TreeTarget {
                root: TreeRoot::Body,
                path: vec![],
            },
            index: 1,
            rows: 1,
            columns: 1,
            header: false,
        }],
    );
    select(&mut s, a, target(&[1, 0, 0, 0, 0], 0, 0));
    insert(&mut s, a, "Cellule");
    select(&mut s, b, target(&[1, 1], 0, 0));
    insert(&mut s, b, "Légende");
    execute(
        &mut s,
        b,
        &[Command::InsertTableRow {
            target: TreeTarget {
                root: TreeRoot::Body,
                path: vec![1],
            },
            index: 0,
        }],
    );
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(target(&[1, 0, 1, 0, 0], 7, 7).into())
    );
    assert_eq!(
        snapshot(&mut s, b).selection,
        Some(target(&[1, 1], 7, 7).into())
    );
    let paragraph=serde_json::from_value(json!({"type":"paragraph","content":[{"type":"text","text":"Avant"},{"type":"footnote","attrs":{"text":"Note"}},{"type":"text","text":"après"}]})).unwrap();
    execute(
        &mut s,
        b,
        &[Command::InsertBlock {
            parent: TreeTarget {
                root: TreeRoot::Body,
                path: vec![],
            },
            index: 0,
            node: paragraph,
        }],
    );
    let annotation = TextTarget {
        zone: TextZone::Annotation { path: vec![0, 1] },
        from: 0,
        to: 4,
    };
    select(&mut s, a, annotation);
    execute(
        &mut s,
        b,
        &[Command::Insert {
            target: TextTarget {
                zone: TextZone::Annotation { path: vec![0, 1] },
                from: 0,
                to: 4,
            },
            text: "Autre note".into(),
            marks: vec![],
        }],
    );
    assert!(snapshot(&mut s, a).selection_lost);
}
#[test]
fn record_contexts_rebase_rich_zones_but_invalid_history_stays_private() {
    use crate::document_types::{DocumentType, Metadata};
    let mut metadata = Metadata::blank(DocumentType::Article);
    if let Metadata::Article(m) = &mut metadata {
        m.localizations[0].summary = crate::content::editorial::RichText::plain("Base");
    }
    let settings = Settings {
        metadata,
        template: DocumentType::Article.default_model(),
        model_id: "article.model.working".into(),
    };
    let state = State::new(
        serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph"}]})).unwrap(),
        settings,
    );
    let mut s = EditingSession::open(
        before_guard(),
        &SharedDocument::from_state(&state).unwrap().update(),
        Assets::default(),
    )
    .unwrap();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let fields = serde_json::to_value(&s.state().settings.metadata).unwrap();
    let id = fields["fields"]["localizations"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let zone = TextZone::Record {
        collection: "localizations".into(),
        id,
        field: "abstract".into(),
        path: vec![0],
    };
    select(
        &mut s,
        a,
        TextTarget {
            zone: zone.clone(),
            from: 0,
            to: 0,
        },
    );
    s.typing_style(&s.guard().clone(), a, Style::Italic, Switch::On)
        .unwrap();
    insert(&mut s, a, "Résumé");
    select(
        &mut s,
        b,
        TextTarget {
            zone: zone.clone(),
            from: 0,
            to: 0,
        },
    );
    insert(&mut s, b, "Un ");
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(
            TextTarget {
                zone: zone.clone(),
                from: 9,
                to: 9
            }
            .into()
        )
    );
    history(&mut s, a, Direction::Undo);
    history(&mut s, a, Direction::Redo);
    execute(
        &mut s,
        a,
        &[
            Command::AddRecord {
                collection: "affiliations".into(),
                id: "lab".into(),
                values: serde_json::from_value(json!({"text":"Laboratoire"})).unwrap(),
                after: None,
            },
            Command::AddRecord {
                collection: "authors".into(),
                id: "alice".into(),
                values: serde_json::from_value(json!({"name":"Alice","affiliation-ids":["lab"]}))
                    .unwrap(),
                after: None,
            },
        ],
    );
    execute(
        &mut s,
        b,
        &[Command::AddRecord {
            collection: "authors".into(),
            id: "bob".into(),
            values: serde_json::from_value(json!({"name":"Bob","affiliation-ids":["lab"]}))
                .unwrap(),
            after: None,
        }],
    );
    let bytes = s.update();
    let guard = s.guard().clone();
    assert!(matches!(
        s.history(&guard, a, Direction::Undo),
        Err(Error::InvalidDocument { .. })
    ));
    assert_eq!(s.update(), bytes);
    assert_eq!(s.guard(), &guard);
    assert!(s.rejected_draft(&guard, a).unwrap().is_some());
    assert!(!snapshot(&mut s, b).pending);
}
#[test]
fn accepted_update_batches_have_one_owner_and_revision_overflow_retains_the_private_history_draft()
{
    use yrs::{Text, XmlFragment};
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let local = SharedDocument::from_update(&s.update()).unwrap();
    let vector = local.doc.transact().state_vector();
    let root = local.doc.get_or_insert_xml_fragment("default");
    let paragraph = root
        .get(&local.doc.transact(), 1)
        .unwrap()
        .into_xml_element()
        .unwrap();
    let text = paragraph
        .get(&local.doc.transact(), 0)
        .unwrap()
        .into_xml_text()
        .unwrap();
    text.insert(&mut local.doc.transact_mut(), 0, "Remote ");
    let update = local.doc.transact().encode_diff_v1(&vector);
    s.execute_updates(&s.guard().clone(), b, &[update]).unwrap();
    assert_eq!(snapshot(&mut s, b).undo_steps, 1);
    assert_eq!(snapshot(&mut s, a).undo_steps, 0);
    history(&mut s, b, Direction::Undo);
    assert_eq!(
        s.state().document.content[1].content[0].text.as_deref(),
        Some("Second")
    );
    // Open at the last valid revision to test failure after local history generation.
    let guard = Guard {
        document: "max".into(),
        revision: u64::MAX - 1,
    };
    let mut last = EditingSession::open(guard, &s.update(), Assets::default()).unwrap();
    let c = ctx(&mut last);
    select(&mut last, c, target(&[0], 0, 0));
    insert(&mut last, c, "x");
    let bytes = last.update();
    let guard = last.guard().clone();
    assert!(matches!(
        last.history(&guard, c, Direction::Undo),
        Err(Error::StateLimitExceeded)
    ));
    assert_eq!(last.update(), bytes);
    assert!(last.rejected_draft(&guard, c).unwrap().is_some());
}

#[test]
fn insertion_keeps_its_zone_when_publication_normalizes_surrounding_whitespace() {
    let state = State::new(
        serde_json::from_value(json!({"type":"doc","content":[
            {"type":"paragraph"},{"type":"paragraph"},
            {"type":"paragraph","content":[{"type":"text","text":"Cible"}]},{"type":"paragraph"}
        ]}))
        .unwrap(),
        Settings::default(),
    );
    let mut s = EditingSession::open(
        before_guard(),
        &SharedDocument::from_state(&state).unwrap().update(),
        Assets::default(),
    )
    .unwrap();
    let a = ctx(&mut s);
    select(&mut s, a, target(&[2], 0, 0));
    insert(&mut s, a, "Ma ");
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(target(&[0], 3, 3).into())
    );
    assert!(text(&s).starts_with("Ma Cible"));
}

fn update_for(s: &EditingSession, text: &str) -> Vec<u8> {
    let mut replica =
        SharedSession::open(s.guard().clone(), &s.update(), Assets::default()).unwrap();
    let prepared = replica
        .prepare_commands(
            s.guard(),
            &[Command::Insert {
                target: target(&[0], 0, 0),
                text: text.into(),
                marks: vec![],
            }],
        )
        .unwrap();
    replica.publish(prepared).unwrap().update
}
#[test]
fn host_groups_typing_but_remote_sync_and_commands_end_the_group() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    for value in ["one", "two"] {
        let update = update_for(&s, value);
        s.execute_grouped_update(&s.guard().clone(), a, &update, 7)
            .unwrap();
    }
    assert_eq!(snapshot(&mut s, a).undo_steps, 1);
    let before_remote = text(&s);
    execute(
        &mut s,
        b,
        &[Command::Insert {
            target: target(&[1], 0, 0),
            text: "remote".into(),
            marks: vec![],
        }],
    );
    let update = update_for(&s, "three");
    s.execute_grouped_update(&s.guard().clone(), a, &update, 7)
        .unwrap();
    assert_eq!(snapshot(&mut s, a).undo_steps, 2);
    history(&mut s, a, Direction::Undo);
    assert_eq!(text(&s), before_remote.replace("Second", "remoteSecond"));
    history(&mut s, a, Direction::Undo);
    assert_eq!(text(&s), "A😀éremoteSecond");
    history(&mut s, a, Direction::Redo);
    history(&mut s, a, Direction::Redo);
    assert!(text(&s).starts_with("threetwoone"));
}
#[test]
fn rejected_typing_preserves_bytes_and_blocks_only_its_context() {
    use yrs::{XmlElementPrelim, XmlFragment};
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let before = s.update();
    let replica = SharedDocument::from_update(&before).unwrap();
    let root = replica.doc.get_or_insert_xml_fragment("default");
    let vector = replica.doc.transact().state_vector();
    root.insert(
        &mut replica.doc.transact_mut(),
        0,
        XmlElementPrelim::empty("unsupported"),
    );
    let update = replica.doc.transact().encode_diff_v1(&vector);
    assert!(
        s.execute_grouped_update(&s.guard().clone(), a, &update, 1)
            .is_err()
    );
    assert_eq!(s.update(), before);
    assert!(snapshot(&mut s, a).pending);
    assert_eq!(
        s.rejected_draft(s.guard(), a).unwrap().unwrap().update,
        update
    );
    assert!(
        s.execute_grouped_update(&s.guard().clone(), a, &update, 2)
            .is_err()
    );
    select(&mut s, b, target(&[0], 0, 0));
    insert(&mut s, b, "Other");
    assert!(text(&s).starts_with("Other"));
    assert!(snapshot(&mut s, a).pending);
}

fn view(
    s: &mut EditingSession,
    c: ContextId,
    selection: Selection,
    styles: Vec<Style>,
) -> ContextSnapshot {
    s.set_context(&s.guard().clone(), c, Some(selection), styles)
        .unwrap()
}
fn point(path: &[usize], offset: u32) -> TextPoint {
    TextPoint {
        zone: TextZone::Body {
            path: path.to_vec(),
        },
        offset,
    }
}
#[test]
fn directed_multizone_selection_is_private_and_rebases_without_becoming_a_flat_range() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let selection = Selection::Text {
        anchor: point(&[1], 4),
        head: point(&[0], 3),
    };
    let bytes = s.update();
    view(&mut s, a, selection.clone(), vec![Style::Italic]);
    assert_eq!(snapshot(&mut s, a).selection, Some(selection));
    assert_eq!(bytes, s.update());
    s.typing_style(&s.guard().clone(), a, Style::Bold, Switch::On)
        .unwrap();
    assert_eq!(
        snapshot(&mut s, a).typing_styles,
        vec![Style::Italic, Style::Bold]
    );
    s.typing_style(&s.guard().clone(), a, Style::Bold, Switch::Off)
        .unwrap();
    assert!(snapshot(&mut s, b).selection.is_none());
    // Inserting at a multizone selection has not been defined: no silent flattening.
    assert!(s.insert(&s.guard().clone(), a, "X".into()).is_err());
    select(&mut s, b, target(&[0], 0, 0));
    insert(&mut s, b, "é日本");
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(Selection::Text {
            anchor: point(&[1], 4),
            head: point(&[0], 6),
        })
    );
    assert_eq!(snapshot(&mut s, a).undo_steps, 0);
    assert_eq!(snapshot(&mut s, a).typing_styles, vec![Style::Italic]);
}
#[test]
fn replacing_view_state_is_atomic_and_preserves_redo_and_backward_direction() {
    let mut s = open();
    let a = ctx(&mut s);
    select(&mut s, a, target(&[0], 0, 0));
    insert(&mut s, a, "X");
    history(&mut s, a, Direction::Undo);
    let selection = Selection::Text {
        anchor: point(&[0], 4),
        head: point(&[0], 1),
    };
    let before = view(&mut s, a, selection.clone(), vec![Style::Bold]);
    assert_eq!(before.redo_steps, 1);
    let guard = s.guard().clone();
    assert!(
        s.set_context(&guard, a, Some(Selection::from(target(&[0], 2, 2))), vec![])
            .is_err()
    );
    assert_eq!(snapshot(&mut s, a), before);
    assert!(
        s.set_context(
            &guard,
            a,
            Some(selection.clone()),
            vec![Style::Link {
                url: "javascript:x".into()
            }]
        )
        .is_err()
    );
    assert_eq!(snapshot(&mut s, a), before);
    assert_eq!(snapshot(&mut s, a).selection, Some(selection));
    history(&mut s, a, Direction::Redo);
    assert!(text(&s).starts_with('X'));
}
#[test]
fn structural_selection_tracks_identity_and_is_lost_on_deletion() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    let tree = |index| TreeTarget {
        root: TreeRoot::Body,
        path: vec![index],
    };
    view(&mut s, a, Selection::Node { target: tree(1) }, vec![]);
    execute(&mut s, b, &[Command::RemoveBlock { target: tree(0) }]);
    assert_eq!(
        snapshot(&mut s, a).selection,
        Some(Selection::Node { target: tree(0) })
    );
    execute(&mut s, b, &[Command::RemoveBlock { target: tree(0) }]);
    let snap = snapshot(&mut s, a);
    assert!(snap.selection_lost && snap.selection.is_none());
    assert!(snap.typing_styles.is_empty());
}
#[test]
fn frontier_detects_missing_deletions_even_with_an_unchanged_state_vector() {
    use yrs::updates::encoder::Encode;
    let mut s = open();
    let a = ctx(&mut s);
    let doc = SharedDocument::from_update(&s.update()).unwrap();
    let before = doc.doc.transact().snapshot().encode_v1();
    s.check_frontier(s.guard(), &before).unwrap();
    execute(
        &mut s,
        a,
        &[Command::Delete {
            target: target(&[0], 0, 1),
        }],
    );
    assert!(matches!(
        s.check_frontier(s.guard(), &before),
        Err(Error::InconsistentState { .. })
    ));
    let now = SharedDocument::from_update(&s.update()).unwrap();
    s.check_frontier(s.guard(), &now.doc.transact().snapshot().encode_v1())
        .unwrap();
}
#[test]
fn cell_and_all_selections_are_structural_and_reject_cross_table_ranges() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    execute(
        &mut s,
        b,
        &[Command::InsertTable {
            parent: TreeTarget {
                root: TreeRoot::Body,
                path: vec![],
            },
            index: 1,
            rows: 2,
            columns: 2,
            header: true,
        }],
    );
    let cell = |r, c| TreeTarget {
        root: TreeRoot::Body,
        path: vec![1, 0, r, c],
    };
    let cells = Selection::Cells {
        anchor: cell(1, 1),
        head: cell(0, 0),
    };
    let before = view(&mut s, a, cells.clone(), vec![]);
    assert_eq!(before.selection, Some(cells.clone()));
    assert!(
        s.set_context(&s.guard().clone(), a, Some(cells), vec![Style::Bold])
            .is_err()
    );
    assert_eq!(snapshot(&mut s, a), before);
    assert!(
        s.set_context(
            &s.guard().clone(),
            a,
            Some(Selection::Cells {
                anchor: cell(0, 0),
                head: TreeTarget {
                    root: TreeRoot::Body,
                    path: vec![0]
                },
            }),
            vec![]
        )
        .is_err()
    );
    assert_eq!(snapshot(&mut s, a), before);
    assert_eq!(
        view(
            &mut s,
            a,
            Selection::All {
                root: TreeRoot::Body
            },
            vec![]
        )
        .selection,
        Some(Selection::All {
            root: TreeRoot::Body
        })
    );
}
#[test]
fn a_stale_view_and_plain_field_styles_cannot_partially_replace_private_state() {
    let mut s = open();
    let a = ctx(&mut s);
    let b = ctx(&mut s);
    view(&mut s, a, target(&[0], 1, 1).into(), vec![Style::Bold]);
    let old = s.guard().clone();
    select(&mut s, b, target(&[1], 0, 0));
    insert(&mut s, b, "Autre ");
    let before = snapshot(&mut s, a);
    assert!(s.set_context(&old, a, None, vec![]).is_err());
    let plain = TextTarget {
        zone: TextZone::Metadata {
            field: MetadataField::LetterSubject,
        },
        from: 0,
        to: 0,
    };
    assert!(
        s.set_context(
            &s.guard().clone(),
            a,
            Some(plain.clone().into()),
            vec![Style::Italic]
        )
        .is_err()
    );
    assert_eq!(snapshot(&mut s, a), before);
    let after = view(&mut s, a, plain.into(), vec![]);
    assert!(after.typing_styles.is_empty());
    assert_eq!(after.undo_steps, before.undo_steps);
}
