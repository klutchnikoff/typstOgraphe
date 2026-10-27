//! Assistance locale : découverte et propositions, jamais d'édition directe.
use crate::{
    Bridge,
    session::{Session, Snapshot},
    settings::Settings,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, State};
use typstographe_core::content::{
    Node, catalogue,
    operations::{Call, Target},
};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub request_id: String,
    pub summary: String,
    pub revision: u64,
    pub status: String,
    pub before: Node,
    pub after: Node,
    pub stages: Vec<Node>,
    #[serde(skip_serializing)]
    pub settings: Settings,
}
#[derive(Default)]
pub struct AgentState {
    pub token: Option<String>,
    pub proposal: Option<Proposal>,
    pub seen: std::collections::BTreeMap<String, (String, Value)>,
    pub snapshot: Option<Snapshot>,
    pub busy: std::collections::BTreeSet<String>,
}
#[derive(Serialize, Deserialize)]
struct Invitation {
    address: String,
    token: String,
}
#[tauri::command]
pub fn publish_snapshot(state: State<Bridge>, snapshot: Snapshot) -> Result<(), String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    session.validate(&snapshot)?;
    state.agent.lock().map_err(|e| e.to_string())?.snapshot = Some(snapshot);
    Ok(())
}
fn proposal(state: &mut AgentState, revision: u64) -> Option<Proposal> {
    if let Some(p) = &mut state.proposal
        && p.status == "pending"
        && p.revision != revision
    {
        p.status = "stale".into()
    }
    state.proposal.clone()
}
#[tauri::command]
pub fn agent_state(state: State<Bridge>) -> Result<Value, String> {
    let revision = state.session.lock().map_err(|e| e.to_string())?.revision;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    Ok(json!({"connected":agent.token.is_some(),"proposal":proposal(&mut agent,revision)}))
}
#[tauri::command]
pub fn agent_revoke(state: State<Bridge>) -> Result<(), String> {
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    agent.token = None;
    agent.proposal = None;
    agent.seen.clear();
    Ok(())
}
#[tauri::command]
pub fn agent_reject(state: State<Bridge>, id: String) -> Result<(), String> {
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    let p = agent
        .proposal
        .as_mut()
        .filter(|p| p.id == id)
        .ok_or("Proposition absente.")?;
    if p.status == "pending" {
        p.status = "rejected".into()
    }
    Ok(())
}
#[tauri::command]
pub fn agent_input(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    composing: bool,
) -> Result<(), String> {
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    if composing {
        agent.busy.insert(window.label().into());
    } else {
        agent.busy.remove(window.label());
    }
    Ok(())
}
#[tauri::command]
pub async fn agent_accept(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: State<'_, Bridge>,
    id: String,
    update: Vec<u8>,
) -> Result<(), String> {
    let (candidate, assets) = {
        let session = state.session.lock().map_err(|e| e.to_string())?;
        let agent = state.agent.lock().map_err(|e| e.to_string())?;
        let p = agent
            .proposal
            .as_ref()
            .filter(|p| p.id == id && p.status == "pending" && p.revision == session.revision)
            .ok_or("Proposition périmée.")?;
        if !agent.busy.is_empty() {
            return Err("Terminez la composition IME avant de décider.".into());
        }
        (p.clone(), session.assets.clone())
    };
    let renderer = state.renderer.clone();
    let validation = candidate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        renderer.validate(
            &crate::schema::source(&validation.after, &assets)?,
            &assets,
            &validation.settings,
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    if !agent.busy.is_empty() {
        return Err("Composition IME en cours.".into());
    }
    let p = agent
        .proposal
        .as_mut()
        .filter(|p| p.id == id)
        .ok_or("Proposition absente.")?;
    if p.status != "pending" || p.revision != session.revision {
        p.status = "stale".into();
        return Err("Proposition périmée : relisez le document.".into());
    }
    if update.is_empty() {
        return Err("Aucune édition à accepter.".into());
    }
    let revision = session.relay(window.label(), &update)?;
    p.status = "accepted".into();
    app.emit(
        "web-editor:change",
        json!({"from":"agent","acceptedBy":window.label(),"revision":revision,"update":update}),
    )
    .map_err(|e| e.to_string())?;
    app.emit("agent:changed", ()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn agent_invite(app: tauri::AppHandle, state: State<Bridge>) -> Result<Value, String> {
    let root = crate::preferences::root(&app)?;
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let invitation = Invitation {
        address: listener.local_addr().unwrap().to_string(),
        token: uuid::Uuid::new_v4().simple().to_string(),
    };
    let file = root.join("invitation.json");
    let options = atomic_write_file::AtomicWriteFile::options();
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    };
    let mut out = options.open(&file).map_err(|e| e.to_string())?;
    out.write_all(&serde_json::to_vec(&invitation).unwrap())
        .map_err(|e| e.to_string())?;
    out.commit().map_err(|e| e.to_string())?;
    {
        let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
        agent.token = Some(invitation.token.clone());
        agent.proposal = None;
        agent.seen.clear();
    }
    let session = state.session.clone();
    let agent = state.agent.clone();
    let token = invitation.token.clone();
    let endpoint = invitation.address.clone();
    let binary = std::env::current_exe().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        loop {
            if agent
                .lock()
                .map(|a| a.token.as_ref() != Some(&token))
                .unwrap_or(true)
            {
                break;
            }
            match listener.accept() {
                Ok((stream, _)) => {
                    let session = session.clone();
                    let agent = agent.clone();
                    let app = app.clone();
                    std::thread::spawn(move || serve_connection(stream, session, agent, app));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(80))
                }
                Err(_) => break,
            }
        }
    });
    Ok(
        json!({"binary":binary,"arguments":["--mcp","--invitation",file],"endpoint":endpoint,"permissions":{"proposals":true,"filesystem":false,"approval":"whole-lot human acceptance only"}}),
    )
}
fn serve_connection(
    mut stream: TcpStream,
    session: Arc<Mutex<Session>>,
    agent: Arc<Mutex<AgentState>>,
    app: tauri::AppHandle,
) {
    // macOS peut transmettre O_NONBLOCK depuis le listener au socket accepté.
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let mut line = String::new();
    if BufReader::new((&stream).take(5 * 1024 * 1024 + 1))
        .read_line(&mut line)
        .is_err()
        || line.len() > 5 * 1024 * 1024
    {
        return;
    }
    let response = (|| {
        let request: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        let s = session.lock().map_err(|e| e.to_string())?;
        let mut a = agent.lock().map_err(|e| e.to_string())?;
        if a.token.is_none() || a.token.as_deref() != request["token"].as_str() {
            return Err("Invitation absente ou révoquée.".to_owned());
        }
        let result = dispatch(
            &s,
            &mut a,
            request["method"].as_str().unwrap_or(""),
            &request["params"],
        );
        if request["method"] == "proposals.submit" {
            let _ = app.emit("agent:changed", ());
        }
        result
    })();
    let value = match response {
        Ok(value) => json!({"result":value}),
        Err(error) => json!({"error":error}),
    };
    let _ = writeln!(stream, "{value}");
}
fn validate_step(step: &Call) -> Result<(), String> {
    static VALIDATORS: std::sync::OnceLock<
        std::collections::BTreeMap<String, jsonschema::Validator>,
    > = std::sync::OnceLock::new();
    let validators = VALIDATORS.get_or_init(|| {
        typstographe_core::content::operations::specifications()
            .into_iter()
            .map(|spec| {
                (
                    spec["id"].as_str().unwrap().into(),
                    jsonschema::validator_for(&spec["params_schema"])
                        .expect("shared command schema"),
                )
            })
            .collect()
    });
    validators
        .get(&step.id)
        .ok_or("Commande inconnue.")?
        .validate(&step.params)
        .map_err(|e| e.to_string())
}
fn dispatch(
    session: &Session,
    state: &mut AgentState,
    method: &str,
    params: &Value,
) -> Result<Value, String> {
    if method == "capabilities.list" {
        return Ok(
            json!({"entries":catalogue::ALL,"commands":typstographe_core::content::operations::specifications(),"target":{"path":"Indices of child nodes, from document.structure; refresh after edits.","from":"UTF-16 offset inside an inline text zone, never split a surrogate pair.","to":"Exclusive end; equal to from for insertion."},"editing":"Submit steps in proposals.submit. Never publish, save, export or approve directly."}),
        );
    }
    if method == "document_types.list" {
        return Ok(
            json!({"entries":crate::catalogue::content_catalogue()["types"].as_array().unwrap().iter().map(|t|json!({"id":t["id"],"label":t["label"],"canCreate":t["canCreate"],"fields":t["fields"],"structured_metadata":t["structured"],"editing":"Metadata is readable; attached proposals currently edit body content only."})).collect::<Vec<_>>()}),
        );
    }
    if method == "capabilities.describe" {
        let id = params["id"].as_str().ok_or("ID absent.")?;
        return typstographe_core::content::operations::specifications()
            .into_iter()
            .find(|entry| entry["id"] == id)
            .ok_or_else(|| "Capacité inconnue.".into());
    }
    if method == "proposals.get" || method == "proposals.cancel" {
        let mut p = proposal(state, session.revision)
            .filter(|p| {
                params
                    .get("proposal")
                    .and_then(Value::as_str)
                    .is_none_or(|id| p.id == id)
            })
            .ok_or("Proposition absente.")?;
        if method == "proposals.cancel" && p.status == "pending" {
            p.status = "cancelled".into();
            state.proposal = Some(p.clone())
        }
        return Ok(json!(p));
    }
    let snapshot = state
        .snapshot
        .as_ref()
        .filter(|s| s.revision == session.revision)
        .ok_or("Document en cours de saisie : relire après publication de l'instantané.")?
        .clone();
    match method {
        "document.inspect" | "document.structure" => Ok(
            json!({"revision":snapshot.revision,"document":snapshot.document,"metadata":snapshot.settings.as_ref().map(|s|&s.metadata),"permissions":{"proposals":true,"filesystem":false},"capabilities":catalogue::for_type(snapshot.settings.as_ref().unwrap_or(&session.settings).metadata.document_type()).collect::<Vec<_>>()}),
        ),
        "proposals.submit" => {
            if !state.busy.is_empty() {
                return Err("Composition IME en cours : aucune proposition différée.".into());
            }
            let request_id = params["request_id"]
                .as_str()
                .filter(|s| !s.is_empty() && s.len() <= 128)
                .ok_or("request_id attendu.")?;
            let fingerprint = {
                use sha2::{Digest, Sha256};
                Sha256::digest(serde_json::to_vec(params).map_err(|e| e.to_string())?)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            proposal(state, session.revision);
            if let Some((previous_fingerprint, previous)) = state.seen.get(request_id) {
                if previous_fingerprint != &fingerprint {
                    return Err("request_id déjà utilisé pour un lot différent.".into());
                }
                return Ok(state
                    .proposal
                    .as_ref()
                    .filter(|p| p.request_id == request_id)
                    .map(|p| json!(p))
                    .unwrap_or_else(|| previous.clone()));
            }
            if state.seen.len() >= 64 {
                return Err(
                    "Limite de propositions atteinte : reconnectez une nouvelle invitation.".into(),
                );
            }
            if params["revision"].as_u64() != Some(snapshot.revision) {
                return Err("Révision périmée.".into());
            }
            if proposal(state, session.revision).is_some_and(|p| p.status == "pending") {
                return Err("Une proposition attend déjà votre décision.".into());
            }
            let steps: Vec<Call> =
                serde_json::from_value(params["steps"].clone()).map_err(|e| e.to_string())?;
            if steps.is_empty() || steps.len() > 128 {
                return Err("Lot limité à 128 étapes.".into());
            }
            let mut after = snapshot.document.clone();
            let settings = snapshot
                .settings
                .clone()
                .unwrap_or_else(|| session.settings.clone());
            let mut context: Target = typstographe_core::content::operations::end_target(&after);
            let mut stages = vec![];
            let mut typing_marks = vec![];
            let mut stage_bytes = 0;
            for step in steps {
                validate_step(&step)?;
                typstographe_core::content::operations::apply(
                    &mut after,
                    &step,
                    &mut context,
                    &mut typing_marks,
                    settings.metadata.document_type(),
                )?;
                crate::schema::source(&after, &session.assets)?;
                stage_bytes += serde_json::to_vec(&after).map_err(|e| e.to_string())?.len();
                if stage_bytes > 8 * 1024 * 1024 {
                    return Err(
                        "Proposition trop volumineuse : utilisez des lots plus courts.".into(),
                    );
                }
                stages.push(after.clone())
            }
            let summary = params["summary"]
                .as_str()
                .unwrap_or("Proposition de rédaction");
            if summary.len() > 2048 {
                return Err("Résumé trop volumineux.".into());
            }
            let p = Proposal {
                id: uuid::Uuid::new_v4().to_string(),
                request_id: request_id.into(),
                summary: summary.into(),
                revision: snapshot.revision,
                status: "pending".into(),
                before: snapshot.document,
                after,
                stages,
                settings,
            };
            if let Some(previous) = &state.proposal
                && let Some((_, receipt)) = state.seen.get_mut(&previous.request_id)
            {
                *receipt = json!({"id":previous.id,"revision":previous.revision,"status":previous.status,"requestId":previous.request_id});
            }
            state.seen.insert(request_id.into(), (fingerprint,json!({"id":p.id,"revision":p.revision,"status":p.status,"requestId":p.request_id})));
            state.proposal = Some(p.clone());
            Ok(json!(p))
        }
        _ => Err("Outil inconnu ou non autorisé dans l'application.".into()),
    }
}

#[derive(Clone)]
struct McpClient {
    invitation: Arc<Invitation>,
}
impl McpClient {
    async fn request(&self, method: String, params: Value) -> Result<Value, String> {
        let invitation = self.invitation.clone();
        tokio::task::spawn_blocking(move || {
            let mut stream = TcpStream::connect(&invitation.address).map_err(|e| e.to_string())?;
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(30)))
                .map_err(|e| e.to_string())?;
            writeln!(
                stream,
                "{}",
                json!({"token":invitation.token,"method":method,"params":params})
            )
            .map_err(|e| e.to_string())?;
            let mut line = String::new();
            BufReader::new(stream.take(16 * 1024 * 1024))
                .read_line(&mut line)
                .map_err(|e| e.to_string())?;
            let value: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
            if let Some(error) = value["error"].as_str() {
                Err(error.into())
            } else {
                Ok(value["result"].clone())
            }
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
fn tools() -> Vec<Tool> {
    let definitions = [
        (
            "document_types.list",
            "List the application's document types; availability is explicit.",
            json!({}),
        ),
        (
            "document.inspect",
            "Read current published document, revision and capabilities.",
            json!({}),
        ),
        (
            "document.structure",
            "Read the structured tree. Build target.path from child indices; offsets are UTF-16 within a text zone.",
            json!({}),
        ),
        (
            "capabilities.list",
            "List shared capabilities and target rules.",
            json!({}),
        ),
        (
            "capabilities.describe",
            "Describe a shared capability.",
            json!({"id":{"type":"string"}}),
        ),
        (
            "proposals.submit",
            "Prepare a whole lot for explicit human review. Returns pending, never published. Return control to the human; do not poll.",
            json!({"revision":{"type":"integer"},"request_id":{"type":"string"},"summary":{"type":"string"},"steps":{"type":"array","items":{"type":"object","properties":{"id":{"type":"string"},"params":{"type":"object"}},"required":["id","params"],"additionalProperties":false}}}),
        ),
        (
            "proposals.get",
            "Read the status of a proposal.",
            json!({"proposal":{"type":"string"}}),
        ),
        (
            "proposals.cancel",
            "Withdraw your pending proposal; cannot undo acceptance.",
            json!({"proposal":{"type":"string"}}),
        ),
    ];
    definitions
        .into_iter()
        .map(|(name, description, properties)| {
            let mut schema =
                json!({"type":"object","properties":properties,"additionalProperties":false});
            if name == "proposals.submit" {
                schema["required"] = json!(["revision", "request_id", "steps"])
            } else if name == "capabilities.describe" {
                schema["required"] = json!(["id"])
            }
            Tool::new(
                name,
                description,
                Arc::new(schema.as_object().unwrap().clone()),
            )
        })
        .collect()
}
impl ServerHandler for McpClient {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(Implementation::new("typstographe-web",env!("CARGO_PKG_VERSION"))).with_instructions("You are attached to typstOgraphe's structured editor. Discover document.inspect and capabilities.list. Use child-index paths from document.structure and the latest revision. Propose literal content via proposals.submit, never raw Typst. Edits are pending until the human accepts the entire lot. No tool may approve, save, export, read preferences or move the human selection. Return control after a proposal. Any published edit makes it stale. On failure refresh rather than guessing. No skill or repository access is required.")
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: tools(),
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        tools().into_iter().find(|tool| tool.name == name)
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let method = request.name.into_owned();
        if self.get_tool(&method).is_none() {
            return Err(ErrorData::invalid_params("Unknown tool", None));
        }
        let result = match self
            .request(method, json!(request.arguments.unwrap_or_default()))
            .await
        {
            Ok(value) => CallToolResult::structured(value),
            Err(error) => CallToolResult::structured_error(json!({"error":error})),
        };
        Ok(result.into())
    }
}
pub fn serve(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let invitation: Invitation = serde_json::from_slice(&std::fs::read(path)?)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let server = McpClient {
            invitation: Arc::new(invitation),
        }
        .serve(rmcp::transport::stdio())
        .await?;
        server.waiting().await?;
        Ok(())
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Session, AgentState) {
        let root = tempfile::tempdir().unwrap();
        let mut session = Session::open(root.path().into()).unwrap();
        let bootstrap = session.join("human");
        let snapshot = Snapshot {
            revision: session.revision,
            document: bootstrap.fixture,
            settings: Some(session.settings.clone()),
        };
        (
            root,
            session,
            AgentState {
                snapshot: Some(snapshot),
                ..Default::default()
            },
        )
    }
    #[test]
    fn proposal_arguments_are_validated_against_the_shared_schemas() {
        for (id, params) in [
            ("text.bold", json!({"value":"guess"})),
            ("insert.table", json!({"rows":-2})),
            ("text.insert", json!({"text":"Hello","unexpected":true})),
            ("text.select", json!({"target":{"path":[-1]}})),
        ] {
            assert!(
                validate_step(&Call {
                    id: id.into(),
                    params
                })
                .is_err(),
                "{id}"
            );
        }
    }
    #[test]
    fn discovery_and_proposals_use_shared_contracts_without_human_approval_tools() {
        let (_root, session, mut state) = fixture();
        let entries = dispatch(&session, &mut state, "capabilities.list", &json!({})).unwrap();
        assert_eq!(
            entries["commands"],
            json!(typstographe_core::content::operations::specifications())
        );
        assert!(tools().iter().all(|t| !t.name.contains("accept")
            && !t.name.contains("save")
            && !t.name.contains("export")));
        let before = serde_json::to_value(&state.snapshot.as_ref().unwrap().document).unwrap();
        let params = json!({"request_id":"unique","revision":0,"steps":[{"id":"text.insert","params":{"text":"Essai administratif."}}]});
        let first = dispatch(&session, &mut state, "proposals.submit", &params).unwrap();
        assert_eq!(first["status"], "pending");
        assert_eq!(
            serde_json::to_value(&state.snapshot.as_ref().unwrap().document).unwrap(),
            before
        );
        assert_eq!(
            dispatch(&session, &mut state, "proposals.submit", &params).unwrap()["id"],
            first["id"]
        );
        let mut conflict = params.clone();
        conflict["steps"][0]["params"]["text"] = json!("Différent");
        assert!(dispatch(&session, &mut state, "proposals.submit", &conflict).is_err());
        dispatch(&session, &mut state, "proposals.cancel", &json!({})).unwrap();
        assert_eq!(
            dispatch(&session, &mut state, "proposals.submit", &params).unwrap()["status"],
            "cancelled"
        );
    }
    #[test]
    fn failed_lots_stale_revisions_and_ime_never_publish_partial_edits() {
        let (_root, mut session, mut state) = fixture();
        let bad = json!({"request_id":"bad","revision":0,"steps":[{"id":"text.insert","params":{"text":"First"}},{"id":"files.save","params":{}}]});
        assert!(dispatch(&session, &mut state, "proposals.submit", &bad).is_err());
        assert!(state.proposal.is_none());
        assert!(state.seen.is_empty());
        assert_eq!(session.revision, 0);
        state.busy.insert("human".into());
        let valid = json!({"request_id":"valid","revision":0,"steps":[{"id":"text.insert","params":{"text":"Only after approval"}}]});
        assert!(dispatch(&session, &mut state, "proposals.submit", &valid).is_err());
        state.busy.clear();
        dispatch(&session, &mut state, "proposals.submit", &valid).unwrap();
        session.revision += 1;
        assert_eq!(
            dispatch(&session, &mut state, "proposals.get", &json!({})).unwrap()["status"],
            "stale"
        );
        assert!(dispatch(&session, &mut state, "document.structure", &json!({})).is_err());
    }
}
