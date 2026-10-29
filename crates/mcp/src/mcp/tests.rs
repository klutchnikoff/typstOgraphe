use super::*;
fn run(h: &mut host::Host, id: &str, p: Value) -> Value {
    h.dispatch(id, p.as_object().unwrap().clone())
        .unwrap()
        .value
}
fn error(h: &mut host::Host, id: &str, p: Value) -> Value {
    api::error_value(
        &h.dispatch(id, p.as_object().unwrap().clone())
            .err()
            .expect("refused"),
    )
}
fn insertion() -> Value {
    json!({"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Bonjour 😀"}})
}
#[test]
fn every_advertised_schema_resolves_and_unknown_params_are_rejected() {
    for attached in [false, true] {
        let tools = catalogue::tools(attached);
        let names: Vec<_> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.windows(2).all(|w| w[0] < w[1]));
        for t in tools {
            jsonschema::validator_for(&json!(t.input_schema))
                .unwrap_or_else(|e| panic!("{}: {e}", t.name));
        }
    }
    assert!(validation::check(true, "commands.list", &json!({"extra":true})).is_err());
    assert!(
        validation::check(
            true,
            "proposals.submit",
            &json!({"document":{},"request_id":"x","steps":[]})
        )
        .is_err()
    );
    assert!(validation::check(true, "document.save", &json!({})).is_err());
}
#[test]
fn strict_references_atomic_batches_and_participant_history_share_the_owner() {
    let mut h = host::Host::new(vec![], vec![], false).unwrap();
    let r = run(&mut h, "document.new", json!({"type":"courrier"}));
    let doc = r["document"].clone();
    assert_eq!(
        error(
            &mut h,
            "document.batch",
            json!({"document":doc,"steps":[insertion(),{"id":"text.delete","params":{"target":{"zone":{"kind":"body","path":[50]},"from":0,"to":1}}}]})
        )["code"],
        "batch_failed"
    );
    assert_eq!(run(&mut h, "document.inspect", json!({}))["document"], doc);
    let changed = run(
        &mut h,
        "document.batch",
        json!({"document":doc,"steps":[insertion()]}),
    );
    assert_ne!(changed["document"], doc);
    assert_eq!(
        error(&mut h, "document.text", json!({"document":doc}))["code"],
        "stale_revision"
    );
    let undone = run(
        &mut h,
        "history.undo",
        json!({"document":changed["document"]}),
    );
    assert_eq!(
        run(
            &mut h,
            "document.text",
            json!({"document":undone["document"]})
        )["outcome"]["text"],
        ""
    );
}
#[test]
fn roots_read_only_and_save_failure_never_publish_a_lot() {
    let d = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let mut h = host::Host::new(vec![d.path().into()], vec![d.path().into()], false).unwrap();
    let r = run(&mut h, "document.new", json!({"type":"courrier"}));
    let doc = r["document"].clone();
    error(
        &mut h,
        "document.batch",
        json!({"document":doc,"steps":[insertion()],"save":{"kind":"as","path":outside.path().join("denied.typsto")}}),
    );
    assert_eq!(run(&mut h, "document.inspect", json!({}))["document"], doc);
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
    let path = d.path().join("new.typsto");
    run(
        &mut h,
        "document.batch",
        json!({"document":doc,"steps":[insertion()],"save":{"kind":"as","path":path}}),
    );
    let mut reader = host::Host::new(vec![d.path().into()], vec![], true).unwrap();
    let r = run(
        &mut reader,
        "document.open",
        json!({"path":path,"read_only":true}),
    );
    assert_eq!(
        error(
            &mut reader,
            "text.insert",
            json!({"document":r["document"],"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"No"})
        )["params"]["reason"],
        "mcp_read_only"
    );
    assert_eq!(
        run(
            &mut reader,
            "document.text",
            json!({"document":r["document"]})
        )["outcome"]["text"],
        "Bonjour 😀\n"
    );
}
#[cfg(unix)]
#[test]
fn symlink_escape_and_lock_sidecar_are_denied_before_file_access() {
    use std::os::unix::fs::symlink;
    let d = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let roots = roots::Roots::new(vec![d.path().into()], vec![d.path().into()]).unwrap();
    symlink(out.path(), d.path().join("escape")).unwrap();
    assert!(roots.write(&d.path().join("escape/file.typsto")).is_err());
    std::fs::write(d.path().join("project.typsto"), "sentinel").unwrap();
    symlink(
        out.path().join("victim"),
        d.path().join(".project.typsto.lock"),
    )
    .unwrap();
    assert!(
        roots
            .document_write(&d.path().join("project.typsto"))
            .is_err()
    );
    assert!(!out.path().join("victim").exists());
    assert!(roots.write(&d.path().join("../outside")).is_err());
}
#[tokio::test]
async fn cancellation_before_dispatch_drops_a_queued_request() {
    use std::time::Duration;
    let (sender, mut receiver) = mpsc::channel(16);
    let (client_io, server_io) = tokio::io::duplex(65536);
    let (server, client) = tokio::join!(
        Server { sender }.serve(server_io),
        ClientConfig::default().serve(client_io)
    );
    let server = server.unwrap();
    let client = client.unwrap();
    let peer = client.peer().clone();
    let pending = tokio::spawn(async move {
        peer.call_tool_once(CallToolRequestParams::new("document.inspect"))
            .await
    });
    let mut request = tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .unwrap()
        .unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(5), request.reply.closed())
        .await
        .unwrap();
    assert!(request.cancel.is_cancelled());
    client.cancel().await.unwrap();
    server.cancel().await.unwrap();
}
