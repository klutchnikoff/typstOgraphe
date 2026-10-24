use super::*;
use crate::mcp::{Host, types::Reply};
use rmcp::{ServiceExt, model::*};
use serde_json::{Value, json};
use std::{
    sync::atomic::AtomicUsize,
    time::{Duration, Instant},
};
use typstographe_core::{document_types::DocumentType, engine::*, storage::StoredDocument};

fn engine() -> Engine {
    Engine::from_stored(StoredDocument::demo(DocumentType::Courrier), None).unwrap()
}
fn run(host: &mut Host<'_>, name: &str, params: Value) -> Result<Reply, CommandError> {
    host.dispatch(name, params.as_object().unwrap().clone())
}

#[test]
fn borrowed_reads_share_the_published_engine_and_refuse_every_command() {
    let mut engine = engine();
    let before = engine.query(Query::Inspect(Empty {})).unwrap();
    let source = engine.document().source().to_owned();
    let context = engine
        .create_edit_context(engine.target_guard(), None)
        .unwrap();
    let mut host = Host::attached(&mut engine, "test-attachment".into(), context.clone()).unwrap();
    let inspect = run(&mut host, "document.inspect", json!({})).unwrap().value;
    assert_eq!(inspect["outcome"], before);
    assert_eq!(inspect["permissions"]["session_mode"], "attached");
    let reference = inspect["document"].clone();
    let catalogue = run(&mut host, "commands.list", json!({"document":reference}))
        .unwrap()
        .value;
    for spec in typstographe_core::engine::catalogue()
        .iter()
        .filter(|s| s.family != Family::Query)
    {
        let error = run(&mut host, spec.id.as_str(), json!({"document":reference}))
            .err()
            .unwrap();
        assert!(matches!(
            error.reason(),
            "mcp_attached_read_only" | "mcp_file_access"
        ));
        let entry = catalogue["outcome"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == spec.id.as_str())
            .unwrap();
        assert_eq!(entry["mcp_support"]["authorized"], false);
    }
    assert_eq!(
        host.resource("typstographe://guide").unwrap().value["outcome"]["session"],
        "attached"
    );
    drop(host);
    assert_eq!(engine.document().source(), source);
    assert_eq!(engine.query(Query::Inspect(Empty {})).unwrap(), before);
}

#[test]
fn attachment_references_expire_when_user_edits_but_inspect_can_refresh() {
    let mut engine = engine();
    let context = engine
        .create_edit_context(engine.target_guard(), None)
        .unwrap();
    let reference = run(
        &mut Host::attached(&mut engine, "same".into(), context.clone()).unwrap(),
        "document.inspect",
        json!({}),
    )
    .unwrap()
    .value["document"]
        .clone();
    engine
        .run(Command::InsertText(InsertTextParams {
            text: "Ajout utilisateur. ".into(),
            target: TextTarget::Current {},
            input_rules: InputRules::Literal,
        }))
        .unwrap();
    let mut host = Host::attached(&mut engine, "same".into(), context.clone()).unwrap();
    assert_eq!(
        run(
            &mut host,
            "document.text",
            json!({"document":reference,"target":{"kind":"all"}})
        )
        .err()
        .unwrap()
        .reason(),
        "stale_revision"
    );
    let current = run(&mut host, "document.inspect", json!({})).unwrap().value;
    assert_ne!(current["document"], reference);
    let resource = host.resource("typstographe://document").unwrap().value;
    assert_eq!(resource["document"], current["document"]);
    assert!(
        resource["outcome"]["text"]["text"]
            .as_str()
            .unwrap()
            .starts_with("Ajout utilisateur.")
    );
}

#[test]
fn attached_reads_use_a_persistent_independent_context_and_never_the_user_selection() {
    let mut engine = engine();
    engine
        .run(Command::SelectText(SelectTextParams {
            target: TextTarget::All {},
            part: None,
        }))
        .unwrap();
    engine
        .run(Command::Bold(StyleParams {
            target: Default::default(),
            value: StyleValue::On,
        }))
        .unwrap();
    let user = engine.primary_context().unwrap();
    assert!(user.typing_style.bold);
    let context = engine
        .create_edit_context(engine.target_guard(), None)
        .unwrap();
    {
        let mut host = Host::attached(&mut engine, "context-test".into(), context.clone()).unwrap();
        let inspect = run(&mut host, "document.inspect", json!({})).unwrap().value;
        assert_ne!(inspect["outcome"]["selection"], json!(user.selection));
        assert_eq!(inspect["outcome"]["typing_style"]["bold"], false);
        let reference = &inspect["document"];
        let text = run(
            &mut host,
            "document.text",
            json!({"document":reference,"target":{"kind":"current"}}),
        )
        .unwrap()
        .value;
        assert_eq!(text["outcome"]["text"], "");
        let resource = host.resource("typstographe://document").unwrap().value;
        assert_eq!(
            resource["outcome"]["state"]["selection"],
            inspect["outcome"]["selection"]
        );
    }
    assert_eq!(engine.primary_context(), Some(user));
    engine.update_adapter_cursor(Selection::default()).unwrap();
    engine
        .run(Command::InsertText(InsertTextParams {
            target: Default::default(),
            text: "にほん ".into(),
            input_rules: InputRules::Literal,
        }))
        .unwrap();
    engine
        .run(Command::SelectText(SelectTextParams {
            target: TextTarget::All {},
            part: None,
        }))
        .unwrap();
    let retained = engine.editing_context(&context).unwrap();
    assert_eq!(engine.resolve_target(retained.selection).unwrap(), (4, 4));
    let primary = engine.primary_context().unwrap();
    let mut host = Host::attached(&mut engine, "context-test".into(), context.clone()).unwrap();
    let inspect = run(&mut host, "document.inspect", json!({})).unwrap().value;
    assert_eq!(inspect["outcome"]["selection"], json!(retained.selection));
    drop(host);
    assert_eq!(engine.primary_context(), Some(primary));
}

#[test]
fn invitation_is_private_and_removed_on_revoke_or_document_replacement() {
    let mut engine = engine();
    let mut session = LocalSession::start(&mut engine, Arc::new(|| {})).unwrap();
    let path = session.invitation_path().to_path_buf();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    // A revision change keeps the grant; an identity change revokes it.
    engine
        .run(Command::InsertText(InsertTextParams {
            text: "X".into(),
            target: TextTarget::Current {},
            input_rules: InputRules::Literal,
        }))
        .unwrap();
    assert!(session.poll(&mut engine));
    let mut replacement = Engine::empty();
    assert!(!session.poll(&mut replacement));
    assert!(session.finished());
    drop(session);
    assert!(!path.exists());
}

#[test]
fn real_local_mcp_connection_reads_resources_rejects_writes_and_is_single_use() {
    let mut engine = engine();
    let selection = engine.selection();
    let guard = engine.target_guard();
    let wakes = Arc::new(AtomicUsize::new(0));
    let counter = wakes.clone();
    let mut session = LocalSession::start(
        &mut engine,
        Arc::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
        }),
    )
    .unwrap();
    let endpoint = session.invitation.data.clone();
    let client = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut bad = endpoint.clone();
                bad.token = "0".repeat(64);
                assert!(transport::connect(&bad).await.is_err());
                let connection = transport::connect(&endpoint).await.unwrap();
                let client = ClientConfig::default().serve(connection).await.unwrap();
                let info = client.peer_info().unwrap();
                assert!(
                    info.instructions
                        .as_ref()
                        .unwrap()
                        .contains("explicitly attached")
                );
                assert_eq!(client.list_all_tools().await.unwrap().len(), 67);
                let inspect = client
                    .call_tool(CallToolRequestParams::new("document.inspect"))
                    .await
                    .unwrap()
                    .structured_content
                    .unwrap();
                assert_eq!(inspect["outcome"]["open"], true);
                assert_eq!(inspect["permissions"]["read_only"], true);
                let resource = client
                    .read_resource(ReadResourceRequestParams::new("typstographe://document"))
                    .await
                    .unwrap();
                assert_eq!(resource.contents.len(), 1);
                let denied = client
                    .call_tool(
                        CallToolRequestParams::new("text.insert").with_arguments(
                            json!({"document":inspect["document"],"text":"Unauthorized"})
                                .as_object()
                                .unwrap()
                                .clone(),
                        ),
                    )
                    .await
                    .unwrap();
                assert_eq!(denied.is_error, Some(true));
                assert!(transport::connect(&endpoint).await.is_err());
                client.cancel().await.unwrap();
                assert!(transport::connect(&endpoint).await.is_err());
            });
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    while !client.is_finished() {
        assert!(Instant::now() < deadline, "local MCP did not complete");
        if !session.finished() {
            assert!(session.poll(&mut engine));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    client.join().unwrap();
    assert_eq!(engine.selection(), selection);
    assert_eq!(engine.target_guard(), guard);
    assert!(wakes.load(Ordering::Relaxed) > 0);
}

#[test]
fn malformed_and_oversized_invitations_are_refused_before_connecting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    for data in [
        "{}".into(),
        "x".repeat(4097),
        r#"{"version":2,"address":"unused","token":"short"}"#.into(),
    ] {
        std::fs::write(&path, data).unwrap();
        assert!(endpoint::read(&path).is_err());
    }
}

#[test]
fn revoking_a_live_connection_abandons_its_queued_read_and_closes_the_transport() {
    let mut engine = engine();
    let guard = engine.target_guard();
    let wakes = Arc::new(AtomicUsize::new(0));
    let count = wakes.clone();
    let session = LocalSession::start(
        &mut engine,
        Arc::new(move || {
            count.fetch_add(1, Ordering::Release);
        }),
    )
    .unwrap();
    let path = session.invitation_path().to_path_buf();
    let endpoint = session.invitation.data.clone();
    let client = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let stream = transport::connect(&endpoint).await.unwrap();
                let client = ClientConfig::default().serve(stream).await.unwrap();
                let result = tokio::time::timeout(
                    Duration::from_secs(5),
                    client.call_tool(CallToolRequestParams::new("document.inspect")),
                )
                .await
                .unwrap();
                assert!(result.is_err());
            });
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while wakes.load(Ordering::Acquire) < 2 {
        assert!(Instant::now() < deadline, "request was not queued");
        std::thread::sleep(Duration::from_millis(2));
    }
    // No poll: the request must not be dispatched after this explicit revoke.
    drop(session);
    client.join().unwrap();
    assert!(!path.exists());
    assert_eq!(engine.target_guard(), guard);
}
