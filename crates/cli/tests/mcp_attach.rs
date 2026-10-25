#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
//! Real stdio SDK client → CLI bridge → local socket → the owner's published Engine.
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::json;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use typstographe_core::{document_types::DocumentType, engine::*, storage::StoredDocument};
use typstographe_mcp::local::LocalSession;

struct Owner {
    invitation: PathBuf,
    stop: Arc<AtomicBool>,
    edit: mpsc::Sender<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Owner {
    fn start() -> Self {
        Self::with_discovery(None)
    }
    fn with_discovery(directory: Option<PathBuf>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let (ready, path) = mpsc::sync_channel(1);
        let (edit, pending_edits) = mpsc::channel::<mpsc::Sender<()>>();
        let thread = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut engine =
                    Engine::from_stored(StoredDocument::demo(DocumentType::Courrier), None)
                        .unwrap();
                let mut session = LocalSession::start(&mut engine, Arc::new(|| {})).unwrap();
                if let Some(directory) = directory {
                    session.advertise(&directory).unwrap();
                }
                ready.send(session.invitation_path().to_path_buf()).unwrap();
                while !stopped.load(Ordering::Acquire) {
                    // Same publication order as egui: native input before attached requests.
                    if let Ok(done) = pending_edits.try_recv() {
                        engine
                            .run(Command::InsertText(InsertTextParams {
                                target: TextTarget::Current {},
                                text: "Texte utilisateur publié. ".into(),
                                input_rules: InputRules::Literal,
                            }))
                            .unwrap();
                        done.send(()).unwrap();
                    }
                    if !session.poll(&mut engine) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                assert!(engine.file_path().is_none());
                assert!(!engine.document().source().contains("Unauthorized"));
            })
            .unwrap();
        Self {
            invitation: path.recv_timeout(Duration::from_secs(10)).unwrap(),
            stop,
            edit,
            thread: Some(thread),
        }
    }
    fn user_edits(&self) {
        let (done, wait) = mpsc::channel();
        self.edit.send(done).unwrap();
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[test]
fn stdio_bridge_reads_the_same_unsaved_document_while_its_owner_keeps_editing() {
    let owner = Owner::start();
    let dir = tempfile::tempdir().unwrap();
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            let client = start(
                dir.path(),
                ProtocolVersion::default(),
                &["--attach", owner.invitation.to_str().unwrap()],
            )
            .await;
            let initial = call(&client, "document.inspect", json!({})).await;
            assert_eq!(initial["outcome"]["open"], true);
            assert!(initial["outcome"]["path"].is_null());
            owner.user_edits();
            let stale = result(
                &client,
                "document.text",
                json!({"document":initial["document"],"target":{"kind":"all"}}),
                false,
            )
            .await
            .structured_content
            .unwrap();
            assert_eq!(stale["error"]["params"]["reason"], "stale_revision");
            let inspect = call(&client, "document.inspect", json!({})).await;
            let text = call(
                &client,
                "document.text",
                json!({"document":inspect["document"],"target":{"kind":"all"}}),
            )
            .await;
            assert!(
                text["outcome"]["text"]
                    .as_str()
                    .unwrap()
                    .starts_with("Texte utilisateur publié.")
            );
            let commands = call(
                &client,
                "commands.list",
                json!({"document":inspect["document"]}),
            )
            .await;
            assert!(
                commands["outcome"]["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|entry| {
                        entry["family"] == "query"
                            || entry["id"] == "text.select"
                            || entry["mcp_support"]["authorized"] == false
                    })
            );
            let navigation = call(
                &client,
                "text.select",
                json!({"document":inspect["document"],"target":{"kind":"all"}}),
            )
            .await;
            assert_eq!(navigation["outcome"]["changed"], false);
            assert_eq!(navigation["outcome"]["effects"], json!([]));
            for name in [
                "text.insert",
                "document.new",
                "document.open",
                "document.close",
                "document.save",
                "document.export",
                "document.batch",
            ] {
                result(
                    &client,
                    name,
                    json!({"document":inspect["document"],"text":"Unauthorized"}),
                    false,
                )
                .await;
            }
            let resources = client.list_all_resources().await.unwrap();
            assert_eq!(resources.len(), 5);
            client
                .read_resource(ReadResourceRequestParams::new("typstographe://document"))
                .await
                .unwrap();
            client.cancel().await.unwrap();
        })
        .await
        .unwrap();
    });
    let path = owner.invitation.clone();
    drop(owner);
    assert!(!path.exists());
    let stderr = std::fs::read_to_string(dir.path().join("stderr.log")).unwrap();
    assert!(stderr.is_empty(), "unexpected bridge stderr: {stderr}");
}

#[test]
fn invalid_attachment_options_and_missing_invitations_do_not_enter_stdio() {
    let missing = tempfile::tempdir().unwrap().path().join("missing.json");
    let command = || std::process::Command::new(env!("CARGO_BIN_EXE_typstographe-cli"));
    let output = command()
        .args(["mcp", "--attach"])
        .arg(&missing)
        .arg("--read-only")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let output = command()
        .args(["mcp", "--attach"])
        .arg(&missing)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn revoking_the_owner_ends_stdio_even_when_the_agent_keeps_stdin_open() {
    let owner = Owner::start();
    let dir = tempfile::tempdir().unwrap();
    runtime().block_on(async {
        let client = start(
            dir.path(),
            ProtocolVersion::default(),
            &["--attach", owner.invitation.to_str().unwrap()],
        )
        .await;
        call(&client, "document.inspect", json!({})).await;
        // Send nothing else and keep the SDK transport alive. The bridge must
        // terminate from socket EOF, without relying on another stdin byte.
        drop(owner);
        tokio::time::timeout(Duration::from_secs(5), client.waiting())
            .await
            .expect("bridge waited for stdin instead of closing stdout")
            .unwrap();
    });
}

#[test]
fn fixed_stdio_configuration_connects_new_invitations_without_rebinding_an_existing_client() {
    use typstographe_mcp::local::discovery;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let first = Owner::with_discovery(Some(root.clone()));
    runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            let config = ["--attach-active"];
            let environment = [(discovery::DIRECTORY_ENV, root.as_path())];
            let client = start_with_env(
                temp.path(),
                ProtocolVersion::default(),
                &config,
                &environment,
            )
            .await;
            let initial = call(&client, "document.inspect", json!({})).await;
            assert!(discovery::resolve(&root).is_err());
            let second = Owner::with_discovery(Some(root.clone()));
            assert_ne!(first.invitation, second.invitation);
            let still_first = call(&client, "document.inspect", json!({})).await;
            assert_eq!(initial["document"], still_first["document"]);
            client.cancel().await.unwrap();
            drop(first);
            let reconnected = start_with_env(
                temp.path(),
                ProtocolVersion::default(),
                &config,
                &environment,
            )
            .await;
            let current = call(&reconnected, "document.inspect", json!({})).await;
            assert_ne!(initial["document"]["id"], current["document"]["id"]);
            assert!(discovery::resolve(&root).is_err());
            reconnected.cancel().await.unwrap();
            drop(second);
        })
        .await
        .unwrap();
    });
}

#[test]
fn active_attachment_never_falls_back_to_standalone_or_accepts_mixed_permissions() {
    use typstographe_mcp::local::discovery;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("discovery");
    let command = || {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_typstographe-cli"));
        cmd.args(["mcp", "--attach-active"])
            .env(discovery::DIRECTORY_ENV, &root);
        cmd
    };
    let output = command().output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("No invitation"));
    let relative = command()
        .env(discovery::DIRECTORY_ENV, "relative")
        .output()
        .unwrap();
    assert_eq!(relative.status.code(), Some(3));
    assert!(relative.stdout.is_empty());
    for args in [
        vec!["--read-only"],
        vec!["--read-root", "."],
        vec!["--write-root", "."],
        vec!["--attach", "anything.json"],
    ] {
        let output = command().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    let first = Owner::with_discovery(Some(root.clone()));
    let second = Owner::with_discovery(Some(root.clone()));
    let output = command().output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Several invitations"));
    drop((first, second));
}
