use super::*;
use crate::mcp::types::Reply;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use typstographe_core::{
    document_types::DocumentType,
    engine::*,
    storage::{self, StoredDocument},
};

fn fixture(
    save: bool,
    export: bool,
) -> (
    tempfile::TempDir,
    Engine,
    LocalSession,
    mpsc::Sender<Request>,
) {
    let dir = tempfile::tempdir().unwrap();
    let mut engine =
        Engine::from_stored(StoredDocument::demo(DocumentType::Courrier), None).unwrap();
    engine
        .run(Command::SaveDocument(SaveDocumentParams {
            guard: engine.target_guard(),
            path: Some(dir.path().join("letter.typsto")),
        }))
        .unwrap();
    edit(&mut engine);
    let root = dir.path().join("exports");
    std::fs::create_dir(&root).unwrap();
    let mut session =
        LocalSession::start_with_files(&mut engine, Arc::new(|| {}), save, export.then_some(root))
            .unwrap();
    let (send, receive) = mpsc::channel(16);
    session.receiver = receive;
    (dir, engine, session, send)
}
fn edit(engine: &mut Engine) {
    engine
        .run(Command::InsertText(InsertTextParams {
            text: "Publié. ".into(),
            target: Default::default(),
            input_rules: InputRules::Literal,
        }))
        .unwrap();
}
fn reference(session: &LocalSession, engine: &Engine) -> Value {
    json!({"id":session.id,"guard":engine.target_guard()})
}
fn request(
    send: &mpsc::Sender<Request>,
    name: &str,
    arguments: Value,
) -> oneshot::Receiver<Result<Reply, CommandError>> {
    let (reply, receive) = oneshot::channel();
    send.try_send(Request {
        operation: Operation::Tool {
            name: name.into(),
            arguments: arguments.as_object().unwrap().clone(),
        },
        reply,
        cancel: CancellationToken::new(),
    })
    .unwrap();
    receive
}
fn finish(
    session: &mut LocalSession,
    engine: &mut Engine,
    mut receive: oneshot::Receiver<Result<Reply, CommandError>>,
    busy: bool,
) -> Result<Reply, CommandError> {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        assert!(session.poll_with_input(engine, busy));
        match receive.try_recv() {
            Ok(reply) => return reply,
            Err(oneshot::error::TryRecvError::Closed) => panic!("reply closed"),
            Err(_) => {}
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn export(
    send: &mpsc::Sender<Request>,
    session: &LocalSession,
    engine: &Engine,
    directory: PathBuf,
) -> oneshot::Receiver<Result<Reply, CommandError>> {
    request(
        send,
        "document.export",
        json!({"document":reference(session,engine),"directory":directory,"format":"both","page_images":false}),
    )
}

#[test]
fn save_published_content_under_existing_lock_not_a_pending_proposal_and_emit_once() {
    let (dir, mut engine, mut session, send) = fixture(true, true);
    engine
        .run(Command::SelectText(SelectTextParams {
            target: TextTarget::All {},
            part: None,
        }))
        .unwrap();
    session
        .prepare_proposal(
            &engine,
            "private",
            vec![Call {
                id: "text.insert".to_owned().try_into().unwrap(),
                params: json!({"text":"PRIVÉ "}),
            }],
        )
        .unwrap();
    let published = engine.stored_document();
    let user = engine.primary_context();
    let guard = engine.target_guard();
    let undo = engine.history().undo;
    let catalogue = request(
        &send,
        "commands.list",
        json!({"document":reference(&session,&engine)}),
    );
    let catalogue = finish(&mut session, &mut engine, catalogue, false).unwrap();
    for id in [
        "document.save",
        "document.export",
        "document.save_as",
        "document.open",
        "text.insert",
    ] {
        let entry = catalogue.value["outcome"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["id"] == id)
            .unwrap();
        assert_eq!(
            entry["mcp_support"]["authorized"],
            matches!(id, "document.save" | "document.export")
        );
    }
    let response = request(
        &send,
        "document.save",
        json!({"document":reference(&session,&engine)}),
    );
    let reply = finish(&mut session, &mut engine, response, false).unwrap();
    assert_eq!(
        reply.value["outcome"]["selection"],
        json!(engine.editing_context(&session.context).unwrap().selection)
    );
    assert_ne!(
        reply.value["outcome"]["selection"],
        json!(user.as_ref().unwrap().selection)
    );
    assert_eq!(reply.value["outcome"]["effects"], json!(["saved"]));
    assert_eq!(
        storage::read(&dir.path().join("letter.typsto")).unwrap(),
        published
    );
    assert_eq!(engine.target_guard(), guard);
    assert_eq!(engine.primary_context(), user);
    assert_eq!(engine.history().undo, undo);
    assert!(!engine.is_dirty());
    assert!(session.review().is_some());
    assert!(matches!(
        session.take_file_events().as_slice(),
        [FileEvent::Saved(_)]
    ));
    assert!(session.take_file_events().is_empty());
    // Neither the exact-file grant nor the export folder permits SaveAs or path redirection.
    for (id, extra) in [
        (
            "document.save",
            json!({"path":dir.path().join("other.typsto")}),
        ),
        (
            "document.save_as",
            json!({"path":dir.path().join("exports/other.typsto"),"overwrite":false}),
        ),
        (
            "document.batch",
            json!({"steps":[],"save":{"kind":"current"}}),
        ),
        ("document.close", json!({"discard_changes":true})),
    ] {
        let mut args = extra;
        args["document"] = reference(&session, &engine);
        let response = request(&send, id, args);
        assert!(finish(&mut session, &mut engine, response, false).is_err());
    }
    assert!(!dir.path().join("other.typsto").exists());
}

#[test]
fn save_grants_do_not_follow_save_as_and_busy_stale_or_external_conflicts_do_not_write() {
    let (dir, mut engine, mut session, send) = fixture(true, false);
    let before = std::fs::read(dir.path().join("letter.typsto")).unwrap();
    let response = request(
        &send,
        "document.save",
        json!({"document":reference(&session,&engine)}),
    );
    assert_eq!(
        finish(&mut session, &mut engine, response, true)
            .err()
            .unwrap()
            .reason(),
        "mcp_attached_busy"
    );
    let response = request(
        &send,
        "document.save",
        json!({"document":reference(&session,&engine)}),
    );
    edit(&mut engine);
    assert_eq!(
        finish(&mut session, &mut engine, response, false)
            .err()
            .unwrap()
            .reason(),
        "stale_revision"
    );
    assert_eq!(
        std::fs::read(dir.path().join("letter.typsto")).unwrap(),
        before
    );
    std::fs::write(dir.path().join("letter.typsto"), b"external").unwrap();
    let response = request(
        &send,
        "document.save",
        json!({"document":reference(&session,&engine)}),
    );
    assert!(finish(&mut session, &mut engine, response, false).is_err());
    assert_eq!(
        std::fs::read(dir.path().join("letter.typsto")).unwrap(),
        b"external"
    );
    engine
        .run(Command::SaveAs(SaveAsParams {
            guard: engine.target_guard(),
            path: dir.path().join("copy.typsto"),
            overwrite: false,
        }))
        .unwrap();
    assert!(session.file_permissions(&engine)["current_file"].is_null());
    let response = request(
        &send,
        "document.save",
        json!({"document":reference(&session,&engine)}),
    );
    assert!(finish(&mut session, &mut engine, response, false).is_err());
    assert!(session.take_file_events().is_empty());
}

#[test]
fn asynchronous_export_uses_only_the_published_snapshot_and_never_marks_it_saved() {
    let (dir, mut engine, mut session, send) = fixture(false, true);
    engine
        .run(Command::SelectText(SelectTextParams {
            target: TextTarget::All {},
            part: None,
        }))
        .unwrap();
    session
        .prepare_proposal(
            &engine,
            "private",
            vec![Call {
                id: "text.insert".to_owned().try_into().unwrap(),
                params: json!({"text":"PRIVÉ "}),
            }],
        )
        .unwrap();
    let published = engine.stored_document();
    let user = engine.primary_context();
    let directory = dir.path().join("exports/letter");
    let response = export(&send, &session, &engine, directory.clone());
    assert!(session.poll(&mut engine));
    assert!(!directory.exists()); // Compilation alone never creates output.
    let reply = finish(&mut session, &mut engine, response, false).unwrap();
    assert_eq!(
        reply.value["outcome"]["selection"],
        json!(engine.editing_context(&session.context).unwrap().selection)
    );
    assert_ne!(
        reply.value["outcome"]["selection"],
        json!(user.as_ref().unwrap().selection)
    );
    assert_eq!(
        reply.value["outcome"]["value"]["guard"],
        json!(engine.target_guard())
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("contenu.typ")).unwrap(),
        published.source
    );
    assert!(directory.join("courrier.pdf").is_file());
    assert!(engine.is_dirty());
    assert_eq!(engine.primary_context(), user);
    assert!(session.review().is_some());
    assert!(matches!(
        session.take_file_events().as_slice(),
        [FileEvent::Exported(_)]
    ));
    let response = export(&send, &session, &engine, dir.path().join("outside"));
    assert!(finish(&mut session, &mut engine, response, false).is_err());
    assert!(!dir.path().join("outside").exists());
}

#[test]
fn deferred_exports_revalidate_revision_interruption_cancellation_and_empty_destination() {
    let (dir, mut engine, mut session, send) = fixture(false, true);
    for scenario in ["stale", "busy", "cancelled", "nonempty"] {
        let directory = dir.path().join("exports").join(scenario);
        let response = export(&send, &session, &engine, directory.clone());
        assert!(session.poll(&mut engine)); // Request started; no write phase until a later poll.
        match scenario {
            "stale" => edit(&mut engine),
            "nonempty" => {
                std::fs::create_dir(&directory).unwrap();
                std::fs::write(directory.join("keep"), b"keep").unwrap();
            }
            "cancelled" => {
                drop(response);
                let deadline = Instant::now() + Duration::from_secs(25);
                while session.export.is_some() {
                    assert!(session.poll(&mut engine));
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(2));
                }
                assert!(session.export.is_none());
                assert!(!directory.exists());
                continue;
            }
            _ => {}
        }
        assert!(finish(&mut session, &mut engine, response, scenario == "busy").is_err());
        if scenario == "nonempty" {
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        } else {
            assert!(!directory.exists());
        }
    }
    assert!(session.take_file_events().is_empty());
}

#[test]
fn typing_after_the_write_decision_keeps_the_export_snapshot_and_a_fresh_outer_reference() {
    let (dir, mut engine, mut session, send) = fixture(false, true);
    let snapshot = engine.stored_document();
    let guard = engine.target_guard();
    let directory = dir.path().join("exports/snapshot");
    let response = export(&send, &session, &engine, directory.clone());
    assert!(session.poll(&mut engine));
    let mut job = session.export.take().unwrap();
    let Phase::Compiling(receive) = job.phase else {
        panic!("compilation expected");
    };
    let compiled = receive
        .recv_timeout(Duration::from_secs(25))
        .unwrap()
        .unwrap();
    let (send, receive) = worker::channel();
    send.send(Ok(compiled)).unwrap();
    job.phase = Phase::Compiling(receive);
    session.export = Some(job);
    assert!(session.poll(&mut engine)); // Revalidation authorizes writing this immutable snapshot.
    assert!(matches!(
        session.export.as_ref().unwrap().phase,
        Phase::Writing(_)
    ));
    edit(&mut engine);
    let current = engine.target_guard();
    assert_ne!(current, guard);
    let reply = finish(&mut session, &mut engine, response, false).unwrap();
    assert_eq!(reply.value["document"]["guard"], json!(current));
    assert_eq!(reply.value["outcome"]["guard"], json!(guard));
    assert_eq!(reply.value["outcome"]["value"]["guard"], json!(guard));
    assert_eq!(
        std::fs::read_to_string(directory.join("contenu.typ")).unwrap(),
        snapshot.source
    );
    assert!(engine.is_dirty());
}

#[test]
fn cancelling_a_running_compile_keeps_its_slot_until_the_worker_finishes() {
    let (dir, mut engine, mut session, send) = fixture(false, true);
    let directory = dir.path().join("exports/cancelled");
    let response = export(&send, &session, &engine, directory.clone());
    assert!(session.poll(&mut engine));
    let job = session.export.as_mut().unwrap();
    let (release, receive) = worker::channel();
    let Phase::Compiling(original) = std::mem::replace(&mut job.phase, Phase::Compiling(receive))
    else {
        panic!("compilation expected");
    };
    drop(response);
    assert!(session.poll(&mut engine));
    assert!(session.export.is_some());
    let second = export(&send, &session, &engine, dir.path().join("exports/second"));
    assert_eq!(
        finish(&mut session, &mut engine, second, false)
            .err()
            .unwrap()
            .reason(),
        "mcp_attached_busy"
    );
    let compiled = original.recv_timeout(Duration::from_secs(25)).unwrap();
    assert!(release.send(compiled).is_ok());
    assert!(session.poll(&mut engine));
    assert!(session.export.is_none());
    assert!(!directory.exists());
    assert!(!dir.path().join("exports/second").exists());
}

#[test]
fn file_permissions_are_opt_in_and_require_an_existing_writable_binding_for_save() {
    let (dir, mut engine, mut session, send) = fixture(false, false);
    let response = request(
        &send,
        "document.save",
        json!({"document":reference(&session,&engine)}),
    );
    assert!(finish(&mut session, &mut engine, response, false).is_err());
    let response = export(&send, &session, &engine, dir.path().join("exports/no"));
    assert!(finish(&mut session, &mut engine, response, false).is_err());
    let mut unsaved =
        Engine::from_stored(StoredDocument::demo(DocumentType::Courrier), None).unwrap();
    assert!(LocalSession::start_with_files(&mut unsaved, Arc::new(|| {}), true, None).is_err());
    let mut readonly = Engine::empty();
    readonly
        .run(Command::OpenDocument(OpenDocumentParams {
            guard: readonly.target_guard(),
            path: dir.path().join("letter.typsto"),
            read_only: true,
            discard_changes: false,
        }))
        .unwrap();
    assert!(LocalSession::start_with_files(&mut readonly, Arc::new(|| {}), true, None).is_err());
    let session = LocalSession::start_with_files(
        &mut readonly,
        Arc::new(|| {}),
        false,
        Some(dir.path().join("exports")),
    )
    .unwrap();
    assert!(session.file_permissions(&readonly)["current_file"].is_null());
}

#[test]
fn real_rmcp_client_saves_and_exports_with_concurrent_reads_on_the_ui_owned_engine() {
    use rmcp::{ServiceExt, model::*};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let mut engine =
        Engine::from_stored(StoredDocument::demo(DocumentType::Courrier), None).unwrap();
    engine
        .run(Command::SaveDocument(SaveDocumentParams {
            guard: engine.target_guard(),
            path: Some(path.clone()),
        }))
        .unwrap();
    edit(&mut engine);
    let published = engine.stored_document();
    let mut session = LocalSession::start_with_files(
        &mut engine,
        Arc::new(|| {}),
        true,
        Some(dir.path().to_path_buf()),
    )
    .unwrap();
    let endpoint = session.invitation.data.clone();
    let directory = dir.path().join("export");
    let destination = directory.clone();
    let client = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let stream = transport::connect(&endpoint).await.unwrap();
            let client = ClientConfig::default().serve(stream).await.unwrap();
            let inspect = client.call_tool(CallToolRequestParams::new("document.inspect")).await.unwrap().structured_content.unwrap();
            assert_eq!(inspect["permissions"]["attached_files"]["current_file"],json!(path.canonicalize().unwrap()));
            let save = client.call_tool(CallToolRequestParams::new("document.save").with_arguments(json!({"document":inspect["document"]}).as_object().unwrap().clone())).await.unwrap();
            assert_ne!(save.is_error,Some(true));
            let args = json!({"document":inspect["document"],"directory":destination,"format":"pdf","page_images":false}).as_object().unwrap().clone();
            let (export,read) = tokio::join!(
                client.call_tool(CallToolRequestParams::new("document.export").with_arguments(args)),
                client.call_tool(CallToolRequestParams::new("document.inspect"))
            );
            assert_ne!(read.unwrap().is_error,Some(true));
            let export = export.unwrap();
            assert_ne!(export.is_error,Some(true));
            assert_eq!(export.structured_content.unwrap()["outcome"]["value"]["format"],"pdf");
            client.cancel().await.unwrap();
        });
    });
    let deadline = Instant::now() + Duration::from_secs(25);
    while !client.is_finished() {
        assert!(Instant::now() < deadline);
        if !session.finished() {
            assert!(session.poll(&mut engine));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    client.join().unwrap();
    assert_eq!(
        storage::read(engine.file_path().unwrap()).unwrap(),
        published
    );
    assert!(directory.join("courrier.pdf").is_file());
    assert!(!engine.is_dirty());
}

#[cfg(unix)]
#[test]
fn deferred_export_does_not_follow_a_replaced_root() {
    let (dir, mut engine, mut session, send) = fixture(false, true);
    let root = dir.path().join("exports");
    let response = export(&send, &session, &engine, root.join("letter"));
    assert!(session.poll(&mut engine));
    std::fs::rename(&root, dir.path().join("old")).unwrap();
    let outside = dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, &root).unwrap();
    assert!(finish(&mut session, &mut engine, response, false).is_err());
    assert_eq!(std::fs::read_dir(outside).unwrap().count(), 0);
}
