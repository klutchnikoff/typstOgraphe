#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
#![cfg_attr(coverage_nightly, coverage(off))]
mod mcp_support;
use mcp_support::*;
use rmcp::model::*;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
};
struct Owner {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    revision: Arc<AtomicUsize>,
    task: Option<thread::JoinHandle<()>>,
}
impl Owner {
    fn start(root: &Path, name: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let path = root.join(name);
        std::fs::write(&path,json!({"address":listener.local_addr().unwrap().to_string(),"token":"private-native-token-for-test"}).to_string()).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let revision = Arc::new(AtomicUsize::new(0));
        let halt = stop.clone();
        let rev = revision.clone();
        let task = thread::spawn(move || {
            while !halt.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                            .unwrap();
                        let mut line = String::new();
                        BufReader::new(&stream).read_line(&mut line).unwrap();
                        let p: Value = serde_json::from_str(&line).unwrap();
                        assert_eq!(p["token"], "private-native-token-for-test");
                        // Transport fixture only; actual native preparation/acceptance is tested against Rust and Yjs.
                        let r = json!({"version":1,"ok":true,"document":{"id":name_for_reply(),"guard":{"document":name_for_reply(),"revision":rev.load(Ordering::SeqCst)}},"outcome":{"method":p["method"],"params":p["params"],"unsaved":true}});
                        writeln!(stream, "{}", json!({"result":r})).unwrap();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(10))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            path,
            stop,
            revision,
            task: Some(task),
        }
    }
}
fn name_for_reply() -> &'static str {
    "native-document"
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let joined = self.task.take().unwrap().join();
        if !std::thread::panicking() {
            joined.unwrap();
        }
    }
}
#[test]
fn both_cli_and_native_invitation_formats_share_the_fixed_stdio_bridge() {
    runtime().block_on(async {
        let d=tempfile::tempdir().unwrap();let owner=Owner::start(d.path(),"invitation.json");
        let c=start(d.path(),ProtocolVersion::default(),&["--attach",owner.path.to_str().unwrap()]).await;
        let names=c.list_all_tools().await.unwrap();assert!(names.iter().any(|t|t.name=="proposals.submit"));assert!(!names.iter().any(|t|t.name=="text.insert"||t.name=="document.save"));
        let r=call(&c,"document.inspect",json!({})).await;assert_eq!(r["outcome"]["unsaved"],true);
        owner.revision.store(2,Ordering::SeqCst);let r=call(&c,"document.inspect",json!({})).await;assert_eq!(r["document"]["guard"]["revision"],2);
        let r=call(&c,"proposals.submit",json!({"document":r["document"],"request_id":"one","steps":[{"id":"text.insert","params":{"target":{"zone":{"kind":"body","path":[0]},"from":0,"to":0},"text":"Proposal"}}]})).await;
        assert_eq!(r["outcome"]["method"],"proposals.submit");assert_eq!(owner.revision.load(Ordering::SeqCst),2);
        c.cancel().await.unwrap();
    });
}
#[test]
fn invalid_invitations_and_mixed_permissions_fail_without_fallback() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("invitation.json");
    for value in [
        json!({"address":"203.0.113.1:80","token":"long-enough-token"}),
        json!({"address":"127.0.0.1:9","token":"short"}),
        json!({"address":"127.0.0.1:9","token":"long-enough-token","extra":true}),
    ] {
        std::fs::write(&p, value.to_string()).unwrap();
        assert_cmd::cargo::cargo_bin_cmd!("typstographe-cli")
            .args(["mcp", "--attach"])
            .arg(&p)
            .assert()
            .failure();
    }
    assert_cmd::cargo::cargo_bin_cmd!("typstographe-cli")
        .args(["mcp", "--attach-active", "--read-only"])
        .assert()
        .failure();
    assert_cmd::cargo::cargo_bin_cmd!("typstographe-cli")
        .args(["mcp", "--attach"])
        .arg(d.path().join("missing"))
        .assert()
        .failure();
}
#[test]
fn fixed_configuration_reads_a_new_invitation_on_explicit_reconnection() {
    runtime().block_on(async {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("config");
        std::fs::create_dir_all(&root).unwrap();
        #[cfg(target_os = "macos")]
        let config = d.path().join("Library/Application Support");
        #[cfg(not(target_os = "macos"))]
        let config = root.clone();
        let config = config.join("org.typstographe.web-editor-spike");
        std::fs::create_dir_all(&config).unwrap();
        let env = [
            ("HOME", d.path()),
            ("APPDATA", root.as_path()),
            ("XDG_CONFIG_HOME", root.as_path()),
        ];
        let one = Owner::start(&config, "invitation.json");
        let a = start_with_env(
            d.path(),
            ProtocolVersion::default(),
            &["--attach-active"],
            &env,
        )
        .await;
        let two = Owner::start(&config, "invitation.json");
        two.revision.store(7, Ordering::SeqCst);
        let b = start_with_env(
            d.path(),
            ProtocolVersion::default(),
            &["--attach-active"],
            &env,
        )
        .await;
        assert_eq!(
            call(&a, "document.inspect", json!({})).await["document"]["guard"]["revision"],
            0
        );
        assert_eq!(
            call(&b, "document.inspect", json!({})).await["document"]["guard"]["revision"],
            7
        );
        a.cancel().await.unwrap();
        b.cancel().await.unwrap();
        drop(one);
        drop(two);
    });
}
