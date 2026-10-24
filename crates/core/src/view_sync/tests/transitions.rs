use super::*;
use crate::engine::Effect;

#[test]
fn native_publication_and_selection_are_separate_and_last_intent_wins() {
    let mut engine = engine("Bonjour Noël.");
    let mut view = idle_view(&engine);
    let before = engine.stored_document();
    let history = engine.history();
    // La sélection seule ne publie aucune révision ; la dernière intention prévaut.
    let revision = engine.revision();
    for (anchor, head) in [(0, 7), (7, 11), (11, 7)] {
        assert!(view.apply(ViewEvent::SelectionChanged {
            selection: target(&engine, anchor, head),
            origin: ViewOrigin::User,
        }));
    }
    assert_eq!(view.selection(), Some(target(&engine, 11, 7)));
    assert!(view.preview_pending());
    assert_eq!(engine.revision(), revision);
    assert_eq!(engine.stored_document(), before);
    assert_eq!(engine.history(), history);
    let mut view = idle_view(&engine);
    let outcome = insert(&mut engine, "Suite ");
    let selection = outcome.selection.unwrap();
    assert!(view.apply(ViewEvent::DocumentPublished {
        selection,
        origin: ViewOrigin::User,
    }));
    assert!(!view.preview_pending());
    assert!(view.apply(ViewEvent::SelectionChanged {
        selection,
        origin: ViewOrigin::User,
    }));
    assert!(view.preview_pending());
}

#[test]
fn command_no_op_save_and_saved_edit_keep_the_current_navigation_policy() {
    let engine = engine("Texte.");
    let baseline = idle_view(&engine);
    for (changed, saved, expected) in [
        (false, false, true),
        (false, true, false),
        (true, false, true),
        (true, true, true),
    ] {
        let mut view = baseline.clone();
        let effects = if saved { vec![Effect::Saved] } else { vec![] };
        let outcome = Outcome {
            guard: engine.target_guard(),
            selection: view.selection(),
            changed,
            effects,
            value: None,
        };
        let event = ViewEvent::CommandCompleted {
            result: (&outcome).into(),
            origin: ViewOrigin::User,
        };
        assert!(view.apply(event.clone()));
        assert_eq!(view.preview_pending(), expected);
        assert!(!view.editor_pending());
        // Une sauvegarde ne retire pas une demande déjà présente.
        view.apply(ViewEvent::RevealRequested {
            guard: engine.target_guard(),
            destination: RevealDestination::Preview,
        });
        view.apply(event);
        assert!(view.preview_pending());
    }
}

#[test]
fn reveal_effects_and_explicit_ui_requests_are_kept_distinct_from_selection() {
    let engine = engine("Texte.");
    let mut view = idle_view(&engine);
    let outcome = Outcome {
        guard: engine.target_guard(),
        selection: view.selection(),
        changed: false,
        effects: vec![Effect::RevealSelection],
        value: None,
    };
    view.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(),
        origin: ViewOrigin::User,
    });
    assert!(view.editor_pending());
    assert_eq!(
        view.take_editor_action(&engine),
        Some(ViewAction::RevealInEditor {
            target: target(&engine, 0, 0),
        })
    );
    assert!(view.take_editor_action(&engine).is_none());
    let mut view = idle_view(&engine);
    for destination in [RevealDestination::Editor, RevealDestination::Both] {
        assert!(view.apply(ViewEvent::RevealRequested {
            guard: engine.target_guard(),
            destination,
        }));
        assert!(view.editor_pending());
        assert!(view.take_editor_action(&engine).is_some());
    }
    assert!(view.preview_pending());
}

#[test]
fn inverse_selection_cancels_the_immediate_return_but_installation_rearms_it() {
    let engine = engine("Texte.");
    let mut view = view(&engine);
    assert!(view.preview_pending());
    view.apply(ViewEvent::SelectionChanged {
        selection: target(&engine, 2, 2),
        origin: ViewOrigin::Preview,
    });
    assert!(!view.preview_pending());
    assert!(view.editor_pending());
    assert_eq!(view.selection(), Some(target(&engine, 2, 2)));
    view.apply(ViewEvent::RenderInstalled {
        render: render_reference(engine.target_guard()),
    });
    assert!(view.preview_pending());
}

#[test]
fn stale_foreign_and_inconsistent_events_never_replace_the_current_intent() {
    let mut engine = engine("Texte.");
    let mut view = view(&engine);
    let old = target(&engine, 0, 0);
    let outcome = insert(&mut engine, "Suite ");
    view.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(),
        origin: ViewOrigin::User,
    });
    let other = super::engine("Autre.");
    let foreign = target(&other, 0, 0);
    let mut inconsistent = ViewCommandResult::from(&outcome);
    inconsistent.selection = Some(old);
    let baseline = view.clone();
    for event in [
        ViewEvent::DocumentPublished {
            selection: old,
            origin: ViewOrigin::User,
        },
        ViewEvent::DocumentPublished {
            selection: foreign,
            origin: ViewOrigin::User,
        },
        ViewEvent::SelectionChanged {
            selection: old,
            origin: ViewOrigin::User,
        },
        ViewEvent::SelectionChanged {
            selection: foreign,
            origin: ViewOrigin::Preview,
        },
        ViewEvent::RenderInstalled {
            render: render_reference(old.guard),
        },
        ViewEvent::RenderFailed {
            guard: foreign.guard,
        },
        ViewEvent::RevealRequested {
            guard: old.guard,
            destination: RevealDestination::Both,
        },
        ViewEvent::CommandCompleted {
            result: inconsistent,
            origin: ViewOrigin::User,
        },
    ] {
        assert!(!view.apply(event));
        assert_eq!(view, baseline);
    }
    // Même révision, mais autre identité : seule une substitution explicite ouvre la vue.
    assert!(view.apply(ViewEvent::DocumentReplaced {
        selection: Some(foreign)
    }));
    assert_eq!(view.selection(), Some(foreign));
    assert!(view.render.is_none());
    assert!(view.editor_pending() && view.preview_pending());
}

#[test]
fn agent_cursor_is_not_a_user_selection_and_publication_does_not_request_navigation() {
    let mut engine = engine("Texte.");
    let mut view = idle_view(&engine);
    let baseline = view.clone();
    assert!(!view.apply(ViewEvent::SelectionChanged {
        selection: target(&engine, 2, 4),
        origin: ViewOrigin::Agent,
    }));
    assert_eq!(view, baseline);
    let outcome = insert(&mut engine, "Suite ");
    let user_position = target(&engine, 6, 6);
    let mut result = ViewCommandResult::from(&outcome);
    result.selection = Some(user_position);
    result.reveal_selection = true;
    view.apply(ViewEvent::CommandCompleted {
        result,
        origin: ViewOrigin::Agent,
    });
    assert_eq!(view.selection(), Some(user_position));
    assert!(!view.editor_pending() && !view.preview_pending());
    view.apply(ViewEvent::RenderInstalled {
        render: render_reference(engine.target_guard()),
    });
    assert!(!view.preview_pending());
    // Le suivi volontaire de l'utilisateur reste possible après la publication.
    view.apply(ViewEvent::SelectionChanged {
        selection: user_position,
        origin: ViewOrigin::User,
    });
    assert!(view.preview_pending());
}

#[test]
fn events_and_actions_round_trip_as_data_without_platform_types() {
    let engine = engine("Texte.");
    let selection = target(&engine, 0, 2);
    let event = ViewEvent::SelectionChanged {
        selection,
        origin: ViewOrigin::User,
    };
    assert_eq!(
        serde_json::from_value::<ViewEvent>(serde_json::to_value(&event).unwrap()).unwrap(),
        event
    );
    let action = ViewAction::RevealInEditor { target: selection };
    assert_eq!(
        serde_json::from_value::<ViewAction>(serde_json::to_value(&action).unwrap()).unwrap(),
        action
    );
    let mut value = serde_json::to_value(event).unwrap();
    value["focus"] = json!(true);
    assert!(serde_json::from_value::<ViewEvent>(value).is_err());
}

#[test]
fn document_replacement_effect_and_close_reset_old_requests_and_render_errors() {
    let first = engine("Premier.");
    let second = engine("Second.");
    let mut view = view(&first);
    view.apply(ViewEvent::RenderFailed {
        guard: first.target_guard(),
    });
    let outcome = Outcome {
        guard: second.target_guard(),
        selection: Some(target(&second, 0, 0)),
        changed: true,
        effects: vec![Effect::DocumentReplaced, Effect::Render],
        value: None,
    };
    assert!(view.apply(ViewEvent::CommandCompleted {
        result: (&outcome).into(),
        origin: ViewOrigin::User,
    }));
    assert!(!view.render_failed);
    assert_eq!(view.selection(), outcome.selection);
    assert!(view.editor_pending() && view.preview_pending());
    let close = Outcome {
        selection: None,
        ..outcome
    };
    assert!(view.apply(ViewEvent::CommandCompleted {
        result: (&close).into(),
        origin: ViewOrigin::User,
    }));
    assert!(view.selection().is_none());
    assert!(!view.apply(ViewEvent::DocumentPublished {
        selection: target(&second, 0, 0),
        origin: ViewOrigin::User,
    }));
    let missing_selection = ViewCommandResult {
        document_replaced: false,
        ..(&close).into()
    };
    assert!(!view.apply(ViewEvent::CommandCompleted {
        result: missing_selection,
        origin: ViewOrigin::User,
    }));
}

#[test]
fn agent_publication_keeps_an_existing_user_navigation_request() {
    let mut engine = engine("Texte.");
    let mut view = view(&engine);
    let outcome = insert(&mut engine, "Suite ");
    assert!(view.apply(ViewEvent::DocumentPublished {
        selection: target(&engine, 6, 6),
        origin: ViewOrigin::Agent,
    }));
    assert!(view.preview_pending() && view.editor_pending());
    // Une cible de commande incohérente ne doit pas modifier ces demandes.
    let baseline = view.clone();
    let mut result = ViewCommandResult::from(&outcome);
    result.guard.revision += 1;
    assert!(!view.apply(ViewEvent::CommandCompleted {
        result,
        origin: ViewOrigin::User
    }));
    assert_eq!(view, baseline);
}
