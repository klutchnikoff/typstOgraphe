use super::*;
use typstographe_core::document_types::DocumentType;

fn engine() -> Engine {
    let mut stored = StoredDocument::blank(DocumentType::Courrier);
    stored.source = "Cheres collègues".into();
    Engine::from_stored(stored, None).unwrap()
}
fn submission(e: &Engine, id: &str) -> Map<String, Value> {
    json!({"document":{"id":"doc","guard":e.target_guard()},"request_id":id,"steps":[
        {"id":"text.insert","params":{"selector":{"select":"Cheres"},"text":"Chères"}}
    ]})
    .as_object()
    .unwrap()
    .clone()
}
fn dispatch(
    s: &mut Proposals,
    e: &Engine,
    c: &EditContext,
    name: &str,
    args: Map<String, Value>,
) -> Value {
    s.dispatch(e, c, "doc", name, args, false).unwrap()
}
#[test]
fn submission_is_private_retry_is_idempotent_and_human_acceptance_is_one_undo() {
    let mut e = engine();
    let c = e.create_edit_context(e.target_guard(), None).unwrap();
    let mut s = Proposals::default();
    let args = submission(&e, "one");
    let before = e.stored_document();
    let user = e.primary_context();
    let result = dispatch(&mut s, &e, &c, "proposals.submit", args.clone());
    assert_eq!(result["status"], "pending");
    assert_eq!(result["published"], false);
    assert_eq!(e.stored_document(), before);
    assert_eq!(e.primary_context(), user);
    assert_eq!(e.history().undo, 0);
    assert_eq!(
        dispatch(&mut s, &e, &c, "proposals.submit", args.clone()),
        result
    );
    assert!(
        s.dispatch(
            &e,
            &c,
            "doc",
            "proposals.submit",
            submission(&e, "two"),
            false
        )
        .is_err()
    );
    let id = result["proposal"].as_str().unwrap();
    s.decide(&mut e, &c, id, true, false).unwrap();
    assert_eq!(e.document().view.text, "Chères collègues");
    assert_eq!(e.history().undo, 1);
    let retry = dispatch(&mut s, &e, &c, "proposals.submit", args);
    assert_eq!(retry["status"], "accepted");
    assert_eq!(retry["published"], true);
    assert!(s.review().is_none());
    assert_eq!(s.records.len(), 1);
    e.run(Command::Undo(Empty {})).unwrap();
    assert_eq!(e.stored_document(), before);
    assert_eq!(s.status(id), Some(ProposalStatus::Accepted));
}
#[test]
fn busy_stale_cancel_and_reject_never_publish_and_status_survives_old_guards() {
    let mut e = engine();
    let c = e.create_edit_context(e.target_guard(), None).unwrap();
    let mut s = Proposals::default();
    let args = submission(&e, "one");
    assert!(
        s.dispatch(&e, &c, "doc", "proposals.submit", args.clone(), true)
            .is_err()
    );
    assert!(s.records.is_empty());
    let result = dispatch(&mut s, &e, &c, "proposals.submit", args);
    let id = result["proposal"].as_str().unwrap();
    assert!(s.decide(&mut e, &c, id, true, true).is_err());
    assert_eq!(e.history().undo, 0);
    e.run_call(Call {
        id: "text.insert".to_owned().try_into().unwrap(),
        params: json!({"text":"Un "}),
    })
    .unwrap();
    assert!(s.decide(&mut e, &c, id, true, false).is_err());
    let status = dispatch(
        &mut s,
        &e,
        &c,
        "proposals.get",
        json!({"proposal":id}).as_object().unwrap().clone(),
    );
    assert_eq!(status["status"], "stale");
    assert!(s.review().is_none());
    for (n, accept) in [("two", false), ("three", false)] {
        let r = dispatch(&mut s, &e, &c, "proposals.submit", submission(&e, n));
        let id = r["proposal"].as_str().unwrap();
        if n == "two" {
            s.decide(&mut e, &c, id, accept, false).unwrap();
            assert_eq!(s.status(id), Some(ProposalStatus::Rejected));
        } else {
            let r = dispatch(
                &mut s,
                &e,
                &c,
                "proposals.cancel",
                json!({"proposal":id}).as_object().unwrap().clone(),
            );
            assert_eq!(r["status"], "cancelled");
        }
    }
    assert_eq!(e.document().view.text, "Un Cheres collègues");
    assert_eq!(e.history().undo, 1);
}
#[test]
fn different_retry_payload_forbidden_steps_resources_and_late_failure_are_atomic() {
    let mut e = engine();
    let c = e.create_edit_context(e.target_guard(), None).unwrap();
    let mut s = Proposals::default();
    for command in [
        "document.save",
        "edit.undo",
        "document.batch",
        "recovery.restore",
    ] {
        let mut args = submission(&e, command);
        args["steps"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":command,"params":{}}));
        assert!(
            s.dispatch(&e, &c, "doc", "proposals.submit", args, false)
                .is_err()
        );
    }
    for step in [
        json!({"id":"insert.image","params":{"image":{"kind":"path","path":"missing.svg"}}}),
        json!({"id":"text.link","params":{"url":"javascript:bad"}}),
    ] {
        let mut args = submission(&e, "bad");
        args["steps"].as_array_mut().unwrap().push(step);
        assert!(
            s.dispatch(&e, &c, "doc", "proposals.submit", args, false)
                .is_err()
        );
    }
    assert!(s.records.is_empty());
    assert_eq!(e.history().undo, 0);
    let args = submission(&e, "same");
    dispatch(&mut s, &e, &c, "proposals.submit", args.clone());
    let mut different = args;
    different.insert("summary".into(), json!("different"));
    assert!(
        s.dispatch(&e, &c, "doc", "proposals.submit", different, false)
            .is_err()
    );
    assert_eq!(s.records.len(), 1);
    assert_eq!(e.document().view.text, "Cheres collègues");
}
#[test]
fn bounded_terminal_records_are_not_evicted_and_helpers_are_not_approval_tools() {
    let mut e = engine();
    let c = e.create_edit_context(e.target_guard(), None).unwrap();
    let mut s = Proposals::default();
    for i in 0..64 {
        let r = dispatch(
            &mut s,
            &e,
            &c,
            "proposals.submit",
            submission(&e, &i.to_string()),
        );
        dispatch(
            &mut s,
            &e,
            &c,
            "proposals.cancel",
            json!({"proposal":r["proposal"]})
                .as_object()
                .unwrap()
                .clone(),
        );
    }
    assert!(
        s.dispatch(
            &e,
            &c,
            "doc",
            "proposals.submit",
            submission(&e, "overflow"),
            false
        )
        .is_err()
    );
    assert_eq!(s.records.len(), 64);
    assert_eq!(e.history().undo, 0);
    assert_eq!(tools().len(), 3);
    for tool in tools() {
        assert!(!tool.name.contains("accept") && !tool.name.contains("approve"));
        assert_eq!(tool.input_schema["additionalProperties"], false);
    }
}
