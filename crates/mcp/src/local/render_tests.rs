use super::*;
use rmcp::{ServiceExt, model::*};
use serde_json::json;
use std::time::{Duration, Instant};
use typstographe_core::{document_types::DocumentType, engine::*, storage::StoredDocument};

fn fixture() -> (Engine, LocalSession) {
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.source = "Noël écrit un courrier.".into();
    stored.template = "#let courrier(..args, body) = body".into();
    let mut engine = Engine::from_stored(stored, None).unwrap();
    let session = LocalSession::start(&mut engine, Arc::new(|| {})).unwrap();
    (engine, session)
}
fn params(engine: &Engine, session: &LocalSession) -> Map<String, Value> {
    json!({"document":{"id":session.id,"guard":engine.target_guard()},"page":1})
        .as_object()
        .unwrap()
        .clone()
}
fn finish(session: &mut LocalSession, engine: &mut Engine) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while session.render_read.is_some() {
        assert!(
            Instant::now() < deadline,
            "background rendering did not finish"
        );
        session.poll_render_read(engine);
        std::thread::sleep(Duration::from_millis(2));
    }
}
#[test]
fn late_page_is_refused_after_typing_and_read_work_never_owns_the_live_engine() {
    for cached in [false, true] {
        let (mut engine, mut session) = fixture();
        if cached {
            engine
                .compile(GuardParams {
                    guard: engine.target_guard(),
                })
                .unwrap();
        }
        let (send, mut receive) = oneshot::channel();
        let arguments = params(&engine, &session);
        session.begin_render_read(
            &mut engine,
            "render.page".into(),
            arguments,
            send,
            CancellationToken::new(),
        );
        assert!(session.render_read.is_some());
        engine
            .run_call(Call {
                id: "text.insert".to_owned().try_into().unwrap(),
                params: json!({"text":"Texte humain. "}),
            })
            .unwrap();
        let before = engine.stored_document();
        finish(&mut session, &mut engine);
        assert_eq!(
            receive.try_recv().unwrap().err().unwrap().reason(),
            "stale_revision"
        );
        assert_eq!(engine.stored_document(), before);
        assert!(engine.render_info().is_none());
        assert_eq!(engine.history().undo, 1);
    }
}
#[test]
fn cancellation_retains_the_single_worker_slot_and_never_installs_a_late_cache() {
    let (mut engine, mut session) = fixture();
    let (send, _receive) = oneshot::channel();
    let (done, pending) = worker::channel();
    let cancel = CancellationToken::new();
    cancel.cancel();
    session.render_read = Some(PendingRead {
        phase: Phase::Compiling(pending),
        name: "render.compile".into(),
        arguments: Map::new(),
        guard: engine.target_guard(),
        reply: send,
        cancel,
    });
    session.poll_render_read(&mut engine);
    assert!(session.render_read.is_some());
    let (send, mut receive) = oneshot::channel();
    let arguments = params(&engine, &session);
    session.begin_render_read(
        &mut engine,
        "render.page".into(),
        arguments,
        send,
        CancellationToken::new(),
    );
    assert_eq!(
        receive.try_recv().unwrap().err().unwrap().reason(),
        "mcp_attached_busy"
    );
    done.send(Ok(engine
        .prepare_render(GuardParams {
            guard: engine.target_guard(),
        })
        .unwrap()
        .compile()))
        .unwrap_or_else(|_| panic!("worker slot lost"));
    session.poll_render_read(&mut engine);
    assert!(session.render_read.is_none());
    assert!(engine.render_info().is_none());
}
#[test]
fn real_sdk_gets_background_compile_geometry_and_png_while_owner_keeps_its_context() {
    let (mut engine, mut session) = fixture();
    let before = engine.primary_context();
    let stored = engine.stored_document();
    let endpoint = session.invitation.data.clone();
    let client = std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let connection = transport::connect(&endpoint).await.unwrap();
                let client = ClientConfig::default().serve(connection).await.unwrap();
                let call = async |name: &str, params: Value| {
                    client
                        .call_tool(
                            CallToolRequestParams::new(name.to_owned())
                                .with_arguments(params.as_object().unwrap().clone()),
                        )
                        .await
                        .unwrap()
                };
                let inspect = call("document.inspect", json!({}))
                    .await
                    .structured_content
                    .unwrap();
                let doc = inspect["document"].clone();
                let (compile, concurrent) = tokio::join!(
                    call("render.compile", json!({"document":doc})),
                    call(
                        "document.text",
                        json!({"document":doc,"target":{"kind":"all"}})
                    )
                );
                let compiled = compile.structured_content.unwrap();
                assert_eq!(compiled["outcome"]["diagnostics"], json!([]));
                assert_eq!(
                    concurrent.structured_content.unwrap()["outcome"]["text"],
                    "Noël écrit un courrier."
                );
                let geometry = call(
                    "render.selection_boxes",
                    json!({"document":doc,"target":{"kind":"all"}}),
                )
                .await;
                assert!(
                    !geometry.structured_content.unwrap()["outcome"]["boxes"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
                let page = call("render.page", json!({"document":doc,"page":1})).await;
                assert_ne!(page.is_error, Some(true));
                assert!(
                    page.content
                        .iter()
                        .any(|c| matches!(c, ContentBlock::Image(_)))
                );
                assert_eq!(
                    page.structured_content.unwrap()["permissions"]["human_approval_required"],
                    true
                );
                client.cancel().await.unwrap();
            });
    });
    let deadline = Instant::now() + Duration::from_secs(60);
    while !client.is_finished() {
        assert!(Instant::now() < deadline, "SDK did not finish");
        if !session.finished() {
            session.poll_with_input(&mut engine, true);
        } // Reads remain available during IME/modal.
        std::thread::sleep(Duration::from_millis(2));
    }
    client.join().unwrap();
    assert_eq!(engine.primary_context(), before);
    assert_eq!(engine.stored_document(), stored);
    assert_eq!(engine.history().undo, 0);
}

#[test]
fn delayed_geometry_captures_agent_current_before_later_navigation() {
    let (mut engine, mut session) = fixture();
    let (send, mut receive) = oneshot::channel();
    let arguments = json!({"document":{"id":session.id,"guard":engine.target_guard()}})
        .as_object()
        .unwrap()
        .clone();
    session.begin_render_read(
        &mut engine,
        "render.selection_boxes".into(),
        arguments,
        send,
        CancellationToken::new(),
    );
    engine
        .navigate_context(
            &session.context,
            SelectTextParams {
                target: TextTarget::All {},
                part: None,
            },
        )
        .unwrap();
    finish(&mut session, &mut engine);
    let response = receive.try_recv().unwrap().unwrap();
    assert_eq!(response.value["outcome"]["boxes"], json!([]));
    assert!(
        !engine
            .selection_boxes(SelectionBoxesParams {
                render: engine.render_info().unwrap().reference,
                target: engine
                    .editing_context(&session.context)
                    .unwrap()
                    .selection
                    .into()
            })
            .unwrap()
            .boxes
            .is_empty()
    );
}

#[test]
fn failed_background_compile_returns_diagnostics_without_synchronous_retry() {
    let mut stored = StoredDocument::demo(DocumentType::Courrier);
    stored.template = "#let courrier(..args, body) = missing-name".into();
    let mut engine = Engine::from_stored(stored, None).unwrap();
    let mut session = LocalSession::start(&mut engine, Arc::new(|| {})).unwrap();
    let (send, mut receive) = oneshot::channel();
    let arguments = json!({"document":{"id":session.id,"guard":engine.target_guard()}})
        .as_object()
        .unwrap()
        .clone();
    session.begin_render_read(
        &mut engine,
        "render.compile".into(),
        arguments,
        send,
        CancellationToken::new(),
    );
    finish(&mut session, &mut engine);
    let response = receive.try_recv().unwrap().unwrap();
    assert!(response.value["outcome"]["render"].is_null());
    assert_eq!(
        response.value["outcome"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(engine.render_info().is_none());
    assert_eq!(engine.history().undo, 0);
}

#[test]
fn deferred_calls_keep_strict_parameter_validation_and_missing_geometry_never_panics() {
    let (mut engine, mut session) = fixture();
    for (name, extra) in [
        ("render.compile", json!({"unknown":true})),
        ("render.compile", json!({"guard":engine.target_guard()})),
        (
            "render.selection_boxes",
            json!({"target":{"kind":"current","unknown":true}}),
        ),
        ("render.locate", json!({})),
    ] {
        let mut arguments = extra.as_object().unwrap().clone();
        arguments.insert(
            "document".into(),
            json!({"id":session.id,"guard":engine.target_guard()}),
        );
        let (send, mut receive) = oneshot::channel();
        session.begin_render_read(
            &mut engine,
            name.into(),
            arguments,
            send,
            CancellationToken::new(),
        );
        finish(&mut session, &mut engine);
        assert!(receive.try_recv().unwrap().is_err(), "{name}");
        assert_eq!(engine.history().undo, 0);
    }
}
