use super::*;
use typstographe_core::{
    assets::Assets,
    content::{
        contract::{Settings, State},
        shared::SharedDocument,
    },
};
fn fixture() -> (tempfile::TempDir, Session, AgentState) {
    let root = tempfile::tempdir().unwrap();
    let mut session = Session::fresh(root.path().into());
    session.join("human");
    let state=State::new(serde_json::from_value(json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Texte"}]},{"type":"paragraph"}]})).unwrap(),Settings::default());
    session
        .relay_group(
            "human",
            &SharedDocument::from_state(&state).unwrap().update(),
            None,
            true,
        )
        .unwrap();
    (root, session, AgentState::default())
}
fn insertion(text: &str) -> Value {
    json!({"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":text}})
}
fn submit(s: &Session, id: &str, steps: Value) -> Value {
    json!({"document":reference(s),"request_id":id,"summary":"Révision","steps":steps})
}
#[test]
fn discovery_matches_the_shared_catalogue_and_cannot_approve_or_write_files() {
    let (_root, mut s, mut a) = fixture();
    let commands = dispatch(&mut s, &mut a, "commands.list", &json!({})).unwrap();
    assert!(
        commands["outcome"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == "record.add" && e["attached_proposal"] == false)
    );
    let tools = typstographe_mcp::tools(true);
    assert!(tools.iter().all(|t| !t.name.contains("accept")
        && !t.name.contains("save")
        && !t.name.contains("export")));
    let ref_ = json!({"document":reference(&s)});
    assert!(dispatch(&mut s, &mut a, "document.save", &ref_).is_err());
    let inspect = dispatch(&mut s, &mut a, "document.inspect", &json!({})).unwrap();
    assert_eq!(inspect["document"], reference(&s));
    assert_eq!(
        inspect["outcome"]["state"],
        json!(s.owner.as_ref().unwrap().state())
    );
    // Reads use the confirmed owner directly, not a deferred frontend snapshot.
    assert!(a.snapshot.is_none());
}
#[test]
fn accepted_proposal_uses_common_edits_one_revision_and_human_history_without_raw_delta() {
    let (_root, mut s, mut a) = fixture();
    let rev = s.revision;
    let before = s.owner.as_ref().unwrap().update_bytes();
    let p = submit(&s, "one", json!([insertion("Bonjour ")]));
    let pending = dispatch(&mut s, &mut a, "proposals.submit", &p).unwrap();
    let id = pending["outcome"]["id"].as_str().unwrap().to_owned();
    assert_eq!(s.revision, rev);
    assert_eq!(s.owner.as_ref().unwrap().update_bytes(), before);
    let pub_ = commit_accepted(&mut s, &mut a, &id, "human").unwrap();
    assert!(pub_.changed);
    assert_eq!(s.revision, rev + 1);
    assert_eq!(a.proposal.as_ref().unwrap().status, "accepted");
    assert_eq!(
        s.owner
            .as_mut()
            .unwrap()
            .snapshot("human")
            .unwrap()
            .undo_steps,
        1
    );
    let retry = dispatch(&mut s, &mut a, "proposals.submit", &p).unwrap();
    assert_eq!(retry["outcome"]["status"], "accepted");
    s.history(
        "human",
        typstographe_core::content::editing::Direction::Undo,
    )
    .unwrap();
    assert_eq!(
        s.owner.as_ref().unwrap().state().document.content[0].content[0]
            .text
            .as_deref(),
        Some("Texte")
    );
}
#[test]
fn stale_invalid_pending_and_ime_batches_never_publish() {
    let (_root, mut s, mut a) = fixture();
    let before = s.updates.clone();
    let p = submit(
        &s,
        "bad",
        json!([insertion("First"),{"id":"text.delete","params":{"target":{"zone":{"kind":"body","path":[99]},"from":0,"to":1}}}]),
    );
    assert!(dispatch(&mut s, &mut a, "proposals.submit", &p).is_err());
    assert!(a.proposal.is_none());
    assert_eq!(s.updates, before);
    a.busy.insert("human".into());
    let valid = submit(&s, "valid", json!([insertion("Pending ")]));
    assert!(dispatch(&mut s, &mut a, "proposals.submit", &valid).is_err());
    a.busy.clear();
    let pending = dispatch(&mut s, &mut a, "proposals.submit", &valid).unwrap();
    let id = pending["outcome"]["id"].as_str().unwrap().to_owned();
    let another = submit(&s, "another", json!([insertion("Other")]));
    assert!(dispatch(&mut s, &mut a, "proposals.submit", &another).is_err());
    let guard = s.guard();
    let calls = vec![serde_json::from_value(insertion("Human ")).unwrap()];
    let pub_ = s
        .owner
        .as_mut()
        .unwrap()
        .calls("human", &guard, &calls)
        .unwrap();
    s.record(&pub_);
    assert!(commit_accepted(&mut s, &mut a, &id, "human").is_err());
    assert_eq!(
        dispatch(&mut s, &mut a, "proposals.get", &json!({"proposal":id})).unwrap()["outcome"]["status"],
        "stale"
    );
}
#[test]
fn header_targets_and_header_selectors_cannot_bypass_body_only_review() {
    let (_root, mut s, mut a) = fixture();
    let guard = s.guard();
    let p=s.owner.as_mut().unwrap().calls("human",&guard,&[Call{id:"metadata.set".into(),params:json!({"target":{"kind":"metadata","field":"letter.subject"},"value":"ONLY HEADER"})}]).unwrap();
    s.record(&p);
    let before = s.updates.clone();
    for target in [
        json!({"zone":{"kind":"metadata","field":"letter.subject"},"from":0,"to":0}),
        json!({"zone":{"kind":"record","collection":"localizations","id":"x","field":"title","path":[0]},"from":0,"to":0}),
    ] {
        let p = submit(
            &s,
            "bad",
            json!([{"id":"text.insert","params":{"target":target,"text":"Hidden"}}]),
        );
        assert!(dispatch(&mut s, &mut a, "proposals.submit", &p).is_err());
    }
    let p = submit(
        &s,
        "hidden",
        json!([{"id":"metadata.set","params":{"target":{"kind":"metadata","field":"recipient.name"},"value":"Hidden"}}]),
    );
    assert!(dispatch(&mut s, &mut a, "proposals.submit", &p).is_err());
    let p = submit(
        &s,
        "selector",
        json!([{"id":"text.insert","params":{"selector":{"select":"ONLY HEADER"},"text":"Hidden"}}]),
    );
    assert!(dispatch(&mut s, &mut a, "proposals.submit", &p).is_err());
    assert_eq!(s.updates, before);
}
#[test]
fn private_agent_context_does_not_change_document_or_human_context() {
    let (_root, mut s, mut a) = fixture();
    let bytes = s.owner.as_ref().unwrap().update_bytes();
    let human = s.owner.as_mut().unwrap().snapshot("human").unwrap();
    let selection: typstographe_core::content::editing::Selection =
        serde_json::from_value::<typstographe_core::content::commands::TextTarget>(
            json!({"zone":{"kind":"body","path":[0]},"from":1,"to":3}),
        )
        .unwrap()
        .into();
    let params =
        json!({"document":reference(&s),"selection":selection,"typing_styles":[{"type":"italic"}]});
    dispatch(&mut s, &mut a, "context.set", &params).unwrap();
    assert_eq!(s.owner.as_ref().unwrap().update_bytes(), bytes);
    assert_eq!(s.owner.as_mut().unwrap().snapshot("human").unwrap(), human);
    let params = json!({"document":reference(&s)});
    assert_eq!(
        dispatch(&mut s, &mut a, "context.get", &params).unwrap()["outcome"]["typing_styles"],
        json!([{"type":"italic"}])
    );
}
#[test]
fn resources_and_schemas_are_current_and_old_flat_commands_are_not_advertised() {
    let (_root, mut s, mut a) = fixture();
    let data = dispatch(
        &mut s,
        &mut a,
        "resource.read",
        &json!({"uri":"typstographe://document"}),
    )
    .unwrap();
    assert_eq!(data["outcome"]["guard"], json!(s.guard()));
    for (id, params) in [
        ("text.bold", json!({"value":"guess"})),
        ("text.select", json!({})),
        (
            "text.insert",
            json!({"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"X","unexpected":true}),
        ),
    ] {
        assert!(
            validate_step(&Call {
                id: id.into(),
                params
            })
            .is_err()
        );
    }
    assert!(Assets::default().is_empty());
}
