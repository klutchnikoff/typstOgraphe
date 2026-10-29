use super::*;
#[test]
fn terminal_receipts_survive_retries_and_new_proposals_without_republishing() {
    for terminal in ["rejected", "cancelled", "accepted", "stale"] {
        let (_root, s, a, id) = pending_acceptance();
        let original = {
            let s = s.lock().unwrap();
            submit(&s, "review", json!([insertion("Proposition ")]))
        };
        match terminal {
            "rejected" => reject(&mut a.lock().unwrap(), &id).unwrap(),
            "cancelled" => {
                let mut s = s.lock().unwrap();
                let p = json!({"proposal":id});
                dispatch(&mut s, &mut a.lock().unwrap(), "proposals.cancel", &p).unwrap();
            }
            "accepted" => {
                accept(&s, &a, &id, "human", |_, _| Ok(())).unwrap();
            }
            "stale" => {
                let mut s = s.lock().unwrap();
                let guard = s.guard();
                let publication = s
                    .ensure_owner()
                    .unwrap()
                    .calls(
                        "human",
                        &guard,
                        &[serde_json::from_value(insertion("Texte humain ")).unwrap()],
                    )
                    .unwrap();
                s.record(&publication);
            }
            _ => unreachable!(),
        }
        let before = confirmed(&s);
        let mut session = s.lock().unwrap();
        let mut agent = a.lock().unwrap();
        let replay = dispatch(&mut session, &mut agent, "proposals.submit", &original).unwrap();
        assert_eq!(replay["outcome"]["id"], id);
        assert_eq!(replay["outcome"]["status"], terminal);
        let second = submit(&session, "second", json!([insertion("Autre lot ")]));
        let next = dispatch(&mut session, &mut agent, "proposals.submit", &second).unwrap();
        let old = dispatch(&mut session, &mut agent, "proposals.submit", &original).unwrap();
        assert_eq!(old["outcome"]["status"], terminal);
        assert_eq!(agent.proposal.as_ref().unwrap().id, next["outcome"]["id"]);
        let mut conflict = original.clone();
        conflict["steps"] = json!([insertion("Autre contenu")]);
        assert!(dispatch(&mut session, &mut agent, "proposals.submit", &conflict).is_err());
        drop(agent);
        drop(session);
        assert_eq!(confirmed(&s), before);
    }
}
#[test]
fn oversized_requests_do_not_occupy_review_or_change_confirmed_work() {
    let (_root, session, agent) = fixture();
    let s = Mutex::new(session);
    let a = Mutex::new(agent);
    let before = confirmed(&s);
    for (steps, summary) in [
        (vec![insertion("x"); 129], "Résumé".into()),
        (vec![insertion("x")], "x".repeat(2049)),
        (vec![], "Résumé".into()),
    ] {
        let mut session = s.lock().unwrap();
        let mut p = submit(&session, "too-large", json!(steps));
        p["summary"] = json!(summary);
        let mut agent = a.lock().unwrap();
        assert!(dispatch(&mut session, &mut agent, "proposals.submit", &p).is_err());
        assert!(agent.proposal.is_none());
        assert!(agent.seen.is_empty());
    }
    assert_eq!(confirmed(&s), before);
}
#[test]
fn failed_invitation_write_keeps_review_then_renewal_revokes_even_an_already_connected_socket() {
    use std::io::{BufRead, BufReader, Write};
    let (root, s, a, id) = pending_acceptance();
    let before = confirmed(&s);
    let invalid = root.path().join("not-a-directory");
    std::fs::write(&invalid, b"keep").unwrap();
    let invitation = Invitation {
        address: "127.0.0.1:12345".into(),
        token: "replacement-token-that-is-long-enough".into(),
    };
    assert!(super::super::invitation::install(&s, &a, &invalid, &invitation).is_err());
    assert_eq!(a.lock().unwrap().proposal.as_ref().unwrap().id, id);
    assert_eq!(
        a.lock().unwrap().token.as_deref(),
        Some("local-invitation-token")
    );
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    client
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let (stream, _) = listener.accept().unwrap();
    let directory = root.path().join("preferences");
    super::super::invitation::install(&s, &a, &directory, &invitation).unwrap();
    assert!(a.lock().unwrap().proposal.is_none());
    assert!(a.lock().unwrap().seen.is_empty());
    std::thread::scope(|scope| {
        let handler = scope.spawn(|| {
            super::super::transport::connection(stream, &s, &a, || panic!("no publication"))
        });
        writeln!(
            client,
            "{}",
            json!({"token":"local-invitation-token","method":"document.inspect","params":{}})
        )
        .unwrap();
        let mut reply = String::new();
        BufReader::new(client).read_line(&mut reply).unwrap();
        handler.join().unwrap();
        assert!(
            serde_json::from_str::<Value>(&reply)
                .unwrap()
                .get("error")
                .is_some()
        );
    });
    let saved: Invitation =
        serde_json::from_slice(&std::fs::read(directory.join("invitation.json")).unwrap()).unwrap();
    assert_eq!(saved.token, invitation.token);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(directory.join("invitation.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    let (reply, _) = rpc(
        &s,
        &a,
        &json!({"token":invitation.token,"method":"document.inspect","params":{}}).to_string(),
    );
    assert!(reply.get("result").is_some(), "{reply}");
    assert!(accept(&s, &a, &id, "human", |_, _| panic!("old proposal")).is_err());
    assert_eq!(confirmed(&s), before);
}
