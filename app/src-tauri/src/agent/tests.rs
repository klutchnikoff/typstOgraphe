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

fn pending_acceptance() -> (tempfile::TempDir, Mutex<Session>, Mutex<AgentState>, String) {
    let (root, mut s, mut a) = fixture();
    a.token = Some("local-invitation-token".into());
    let params = submit(&s, "review", json!([insertion("Proposition ")]));
    let response = dispatch(&mut s, &mut a, "proposals.submit", &params).unwrap();
    let id = response["outcome"]["id"].as_str().unwrap().to_owned();
    (root, Mutex::new(s), Mutex::new(a), id)
}
fn confirmed(session: &Mutex<Session>) -> Value {
    let mut s = session.lock().unwrap();
    let human = s.ensure_owner().unwrap().snapshot("human").unwrap();
    json!({"guard":s.guard(),"state":s.owner.as_ref().unwrap().state(),"updates":s.updates,
        "history":human,"resources":s.resources(),"clean":s.clean_revision})
}
#[test]
fn full_acceptance_validates_real_typst_and_second_click_cannot_reapply_or_relabel_the_lot() {
    let (_root, s, a, id) = pending_acceptance();
    let before = confirmed(&s);
    let renderer = crate::rendering::Renderer::default();
    let publication = accept(&s, &a, &id, "human", |p, assets| {
        assert_eq!(confirmed(&s), before); // Validation holds no document lock.
        renderer.validate(
            &crate::schema::source(&p.after, assets)?,
            assets,
            &p.settings,
        )
    })
    .unwrap();
    assert!(publication.changed);
    let accepted = confirmed(&s);
    assert_eq!(
        accepted["guard"]["revision"],
        before["guard"]["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(accepted["history"]["undo_steps"], 1);
    assert!(
        accept(&s, &a, &id, "human", |_, _| panic!(
            "a second click must not compile again"
        ))
        .is_err()
    );
    assert_eq!(
        a.lock().unwrap().proposal.as_ref().unwrap().status,
        "accepted"
    );
    assert_eq!(confirmed(&s), accepted);
    let mut s = s.lock().unwrap();
    s.history(
        "human",
        typstographe_core::content::editing::Direction::Undo,
    )
    .unwrap();
    assert_eq!(json!(s.owner.as_ref().unwrap().state()), before["state"]);
}
#[test]
fn compilation_failure_keeps_the_proposal_pending_and_all_confirmed_work_intact() {
    let (_root, s, a, id) = pending_acceptance();
    let before = confirmed(&s);
    let renderer = crate::rendering::Renderer::default();
    assert!(
        accept(&s, &a, &id, "human", |p, assets| {
            renderer.validate(
                "#panic(\"simulated compilation failure\")",
                assets,
                &p.settings,
            )
        })
        .is_err()
    );
    assert_eq!(confirmed(&s), before);
    assert_eq!(
        a.lock().unwrap().proposal.as_ref().unwrap().status,
        "pending"
    );
    accept(&s, &a, &id, "human", |_, _| Ok(())).unwrap();
    assert_eq!(confirmed(&s)["history"]["undo_steps"], 1);
}
#[test]
fn human_edit_during_compilation_is_preserved_and_only_the_proposal_becomes_stale() {
    let (_root, s, a, id) = pending_acceptance();
    let mut expected = Value::Null;
    assert!(
        accept(&s, &a, &id, "human", |_, _| {
            let mut current = s.lock().unwrap();
            let guard = current.guard();
            let publication = current
                .ensure_owner()
                .unwrap()
                .calls(
                    "human",
                    &guard,
                    &[serde_json::from_value(insertion("Saisie humaine ")).unwrap()],
                )
                .unwrap();
            current.record(&publication);
            drop(current);
            expected = confirmed(&s);
            Ok(())
        })
        .is_err()
    );
    assert_eq!(confirmed(&s), expected);
    assert_eq!(a.lock().unwrap().proposal.as_ref().unwrap().status, "stale");
    assert_eq!(expected["history"]["undo_steps"], 1);
}
#[test]
fn composition_refusal_revocation_and_closed_window_during_validation_never_publish() {
    for interruption in ["ime", "reject", "revoke", "leave"] {
        let (_root, s, a, id) = pending_acceptance();
        let before = confirmed(&s);
        let result = accept(&s, &a, &id, "human", |_, _| {
            match interruption {
                "ime" => {
                    a.lock().unwrap().busy.insert("human".into());
                }
                "reject" => reject(&mut a.lock().unwrap(), &id).unwrap(),
                "revoke" => revoke(&mut s.lock().unwrap(), &mut a.lock().unwrap()).unwrap(),
                "leave" => s.lock().unwrap().leave("human"),
                _ => unreachable!(),
            }
            Ok(())
        });
        assert!(result.is_err(), "{interruption}");
        // Leaving closes the private context, but must not alter the document.
        let after = confirmed(&s);
        assert_eq!(after["state"], before["state"], "{interruption}");
        assert_eq!(after["updates"], before["updates"], "{interruption}");
        assert_eq!(after["guard"], before["guard"], "{interruption}");
        assert_eq!(after["history"]["undo_steps"], 0);
        let a = a.lock().unwrap();
        match interruption {
            "ime" => assert_eq!(a.proposal.as_ref().unwrap().status, "pending"),
            "reject" => assert_eq!(a.proposal.as_ref().unwrap().status, "rejected"),
            "revoke" => {
                assert!(a.proposal.is_none());
                assert!(a.seen.is_empty());
                assert!(a.token.is_none());
            }
            _ => {}
        }
    }
}
#[test]
fn replacement_during_validation_cannot_apply_an_old_document_proposal() {
    let (_root, s, a, id) = pending_acceptance();
    let mut expected = Value::Null;
    assert!(
        accept(&s, &a, &id, "human", |_, _| {
            let mut current = s.lock().unwrap();
            let state = current.owner.as_ref().unwrap().state().clone();
            let update = current.owner.as_ref().unwrap().update_bytes();
            current.replace(state.document, state.settings, Assets::default(), update);
            drop(current);
            expected = confirmed(&s);
            Ok(())
        })
        .is_err()
    );
    assert_eq!(confirmed(&s), expected);
    assert_eq!(a.lock().unwrap().proposal.as_ref().unwrap().status, "stale");
}
#[test]
fn two_acceptances_that_compile_concurrently_publish_once_and_keep_the_accepted_status() {
    let (_root, s, a, id) = pending_acceptance();
    let before = confirmed(&s);
    let barrier = std::sync::Barrier::new(2);
    let notifications = std::sync::atomic::AtomicUsize::new(0);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                scope.spawn(|| {
                    super::approval::accept_with_notification(
                        &s,
                        &a,
                        &id,
                        "human",
                        |_, _| {
                            barrier.wait();
                            Ok(())
                        },
                        |_| {
                            // The publication cannot be overtaken by a new edit before notification.
                            assert!(s.try_lock().is_err());
                            notifications.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            Ok(())
                        },
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(notifications.load(std::sync::atomic::Ordering::Relaxed), 1);
    let after = confirmed(&s);
    assert_eq!(
        after["guard"]["revision"],
        before["guard"]["revision"].as_u64().unwrap() + 1
    );
    assert_eq!(after["history"]["undo_steps"], 1);
    assert_eq!(
        a.lock().unwrap().proposal.as_ref().unwrap().status,
        "accepted"
    );
}

fn rpc(session: &Mutex<Session>, agent: &Mutex<AgentState>, request: &str) -> (Value, usize) {
    use std::io::{BufRead, BufReader, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let (stream, _) = listener.accept().unwrap();
    let notifications = AtomicUsize::new(0);
    let response = std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            super::transport::connection(stream, session, agent, || {
                notifications.fetch_add(1, Ordering::Relaxed);
            })
        });
        writeln!(client, "{request}").unwrap();
        let mut line = String::new();
        BufReader::new(client).read_line(&mut line).unwrap();
        worker.join().unwrap();
        serde_json::from_str(&line).unwrap()
    });
    (response, notifications.load(Ordering::Relaxed))
}
#[test]
fn native_mcp_socket_authenticates_and_validates_requests_without_changing_human_work() {
    let (_root, s, a, _id) = pending_acceptance();
    let before = confirmed(&s);
    for request in [
        "not JSON".to_string(),
        json!({"token":"wrong","method":"document.inspect","params":{}}).to_string(),
        json!({"token":"local-invitation-token","method":"document.save","params":{}}).to_string(),
        json!({"token":"local-invitation-token","method":"context.set","params":{"invalid":true}})
            .to_string(),
    ] {
        let (reply, events) = rpc(&s, &a, &request);
        assert!(reply.get("error").is_some(), "{reply}");
        assert_eq!(events, 0);
        assert_eq!(confirmed(&s), before);
    }
    for (method, params) in [
        ("document.inspect", json!({})),
        ("resource.read", json!({"uri":"typstographe://document"})),
    ] {
        let (reply, events) = rpc(
            &s,
            &a,
            &json!({"token":"local-invitation-token","method":method,"params":params}).to_string(),
        );
        assert!(reply.get("error").is_none(), "{reply}");
        assert_eq!(events, 0);
        assert_eq!(confirmed(&s), before);
    }
}
#[test]
fn native_mcp_submission_remains_private_and_revocation_cancels_review_and_socket_access() {
    let (_root, session, mut agent) = fixture();
    agent.token = Some("invitation-token-for-network".into());
    let params = submit(
        &session,
        "network",
        json!([insertion("Private network text ")]),
    );
    let s = Mutex::new(session);
    let a = Mutex::new(agent);
    let before = confirmed(&s);
    let request =
        json!({"token":"invitation-token-for-network","method":"proposals.submit","params":params});
    let (reply, events) = rpc(&s, &a, &request.to_string());
    assert!(reply.get("error").is_none(), "{reply}");
    assert_eq!(events, 1);
    assert_eq!(confirmed(&s), before);
    let id = reply["result"]["outcome"]["id"].as_str().unwrap();
    revoke(&mut s.lock().unwrap(), &mut a.lock().unwrap()).unwrap();
    let (refused, events) = rpc(
        &s,
        &a,
        &json!({"token":"invitation-token-for-network","method":"document.inspect","params":{}})
            .to_string(),
    );
    assert!(refused.get("error").is_some());
    assert_eq!(events, 0);
    assert!(
        accept(&s, &a, id, "human", |_, _| panic!(
            "revoked proposal must not compile"
        ))
        .is_err()
    );
    assert_eq!(confirmed(&s), before);
}

#[test]
fn notification_failure_keeps_the_accepted_work_recoverable_without_reapplying_the_lot() {
    let (_root, s, a, id) = pending_acceptance();
    assert!(
        super::approval::accept_with_notification(
            &s,
            &a,
            &id,
            "human",
            |_, _| Ok(()),
            |_| Err("window notification unavailable".into()),
        )
        .is_err()
    );
    let accepted = confirmed(&s);
    assert_eq!(accepted["history"]["undo_steps"], 1);
    assert_eq!(
        a.lock().unwrap().proposal.as_ref().unwrap().status,
        "accepted"
    );
    assert!(accept(&s, &a, &id, "human", |_, _| panic!("already accepted")).is_err());
    assert_eq!(confirmed(&s), accepted);
    let mut current = s.lock().unwrap();
    let revision = current.revision;
    let owner = current.ensure_owner().unwrap();
    let snapshot = crate::session::Snapshot {
        document: owner.state().document.clone(),
        settings: Some(owner.state().settings.clone()),
        revision,
    };
    let update = owner.update_bytes();
    let path = current.save(&snapshot, update).unwrap();
    let saved: crate::session::Checkpoint =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(json!(saved.document), accepted["state"]["document"]);
    assert!(!saved.update.is_empty());
}

#[path = "workflow_tests.rs"]
mod workflow_tests;
#[test]
fn attached_requests_stay_scoped_to_this_document_and_known_resources() {
    let (_root, mut s, mut a) = fixture();
    let rev = s.revision;
    let mut foreign = json!({"document":reference(&s)});
    foreign["document"]["id"] = json!("another-document");
    assert_eq!(
        dispatch(&mut s, &mut a, "document.inspect", &foreign),
        Err("Foreign document reference".into())
    );
    let read = |uri: &str| json!({ "uri": uri });
    let guide = dispatch(
        &mut s,
        &mut a,
        "resource.read",
        &read("typstographe://guide"),
    )
    .unwrap();
    assert!(guide["outcome"]["instructions"].is_string());
    let types = dispatch(
        &mut s,
        &mut a,
        "resource.read",
        &read("typstographe://types"),
    )
    .unwrap();
    assert_eq!(types["outcome"], api::types());
    for uri in [
        "typstographe://commands",
        "typstographe://content-capabilities",
        "typstographe://document",
        "typstographe://models",
    ] {
        dispatch(&mut s, &mut a, "resource.read", &read(uri)).unwrap();
    }
    assert_eq!(
        dispatch(&mut s, &mut a, "resource.read", &read("file:///etc/passwd")),
        Err("Unknown resource".into())
    );
    assert_eq!(
        dispatch(
            &mut s,
            &mut a,
            "resource.read",
            &json!({"uri":"typstographe://guide","extra":1})
        ),
        Err("Expected resource URI".into())
    );
    let described = dispatch(
        &mut s,
        &mut a,
        "document_types.describe",
        &json!({"type":"article"}),
    );
    assert!(described.is_ok());
    assert!(
        dispatch(
            &mut s,
            &mut a,
            "capabilities.describe",
            &json!({"id":"text.insert"})
        )
        .is_ok()
    );
    assert!(
        dispatch(
            &mut s,
            &mut a,
            "capabilities.describe",
            &json!({"id":"nothing.here"})
        )
        .is_err()
    );
    a.busy.insert("main".into());
    let context = json!({"document":reference(&s),"selection":null});
    assert!(dispatch(&mut s, &mut a, "context.set", &context).is_err());
    a.busy.clear();
    let kept = submit(&s, "kept", json!([insertion("x")]));
    dispatch(&mut s, &mut a, "proposals.submit", &kept).unwrap();
    s.generation = "replaced-document".into();
    assert_eq!(
        dispatch(&mut s, &mut a, "document.inspect", &json!({})),
        Err("Document replaced; reconnect invitation".into())
    );
    assert_eq!(s.revision, rev);
}
#[test]
fn the_invitation_listener_serves_only_while_its_token_is_current() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let agent = Arc::new(Mutex::new(AgentState {
        token: Some("current".into()),
        ..Default::default()
    }));
    let (served, received) = std::sync::mpsc::channel();
    let loop_agent = agent.clone();
    let worker = std::thread::spawn(move || {
        accept_while_invited(&listener, &loop_agent, "current", |stream| {
            served.send(stream.peer_addr().is_ok()).unwrap();
        })
    });
    let _client = TcpStream::connect(address).unwrap();
    let timeout = std::time::Duration::from_secs(5);
    assert_eq!(received.recv_timeout(timeout), Ok(true));
    agent.lock().unwrap().token = Some("replacement".into());
    let deadline = std::time::Instant::now() + timeout;
    while !worker.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "a revoked listener kept accepting"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    worker.join().unwrap();
    assert!(received.try_recv().is_err());
}
