//! Assistance locale : découverte et propositions, jamais d'édition directe.
use crate::{
    Bridge,
    session::{Session, Snapshot},
    settings::Settings,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, State};
use typstographe_core::content::{
    Node,
    api::{self, Call},
    catalogue,
    contract::{Guard, State as ContentState},
};
use typstographe_mcp::local::Invitation;
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub request_id: String,
    pub summary: String,
    pub revision: u64,
    pub generation: String,
    pub status: String,
    pub before: Node,
    pub after: Node,
    #[serde(skip_serializing)]
    pub settings: Settings,
    #[serde(skip_serializing)]
    pub steps: Vec<Call>,
}
#[derive(Default)]
pub struct AgentState {
    pub token: Option<String>,
    pub proposal: Option<Proposal>,
    pub seen: std::collections::BTreeMap<String, (String, Value)>,
    pub snapshot: Option<Snapshot>,
    pub busy: std::collections::BTreeSet<String>,
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
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    if let Some(owner) = &mut session.owner {
        owner.close(&agent_peer(&agent))?;
    }
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
    let publication = commit_accepted(&mut session, &mut agent, &id, window.label())?;
    let revision = publication.guard.revision;
    let update = publication.update;
    app.emit(
        "web-editor:change",
        json!({"from":"agent","acceptedBy":window.label(),"generation":session.generation,"revision":revision,"update":update}),
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
        let mut session = state.session.lock().map_err(|e| e.to_string())?;
        let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
        if let Some(owner) = &mut session.owner {
            owner.close(&agent_peer(&agent))?;
        }
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
        authorize(
            &*agent.lock().map_err(|e| e.to_string())?,
            request["token"].as_str(),
        )?;
        // Schema compilation can be cold/expensive; do it before taking the live document lock.
        validate_request(request["method"].as_str().unwrap_or(""), &request["params"])?;
        let mut s = session.lock().map_err(|e| e.to_string())?;
        let mut a = agent.lock().map_err(|e| e.to_string())?;
        authorize(&a, request["token"].as_str())?; // Revocation may have happened meanwhile.
        let result = dispatch(
            &mut s,
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
        Err(error) => {
            let value = serde_json::from_str::<Value>(&error)
                .unwrap_or_else(|_| json!({"code":"unavailable","message":error}));
            json!({"error":value})
        }
    };
    let _ = writeln!(stream, "{value}");
}
fn validate_step(step: &Call) -> Result<(), String> {
    if !typstographe_mcp::attached_step(step) {
        return Err("Capability unavailable in attached body proposals".into());
    }
    api::check_params(&step.id, &step.params)
        .map_err(|e| serde_json::to_string(&api::error_value(&e)).unwrap())
}
fn reference(session: &Session) -> Value {
    json!({"id":session.generation,"guard":session.guard()})
}
fn check_reference(session: &Session, params: &Value) -> Result<(), String> {
    if params["document"]["id"] != session.generation {
        return Err("Foreign document reference".into());
    }
    let guard: Guard =
        serde_json::from_value(params["document"]["guard"].clone()).map_err(|e| e.to_string())?;
    guard
        .check(&session.guard())
        .map_err(|e| serde_json::to_string(&e).unwrap())
}
fn agent_peer(state: &AgentState) -> String {
    format!("agent:{}", state.token.as_deref().unwrap_or("diagnostic"))
}
pub(crate) fn commit_accepted(
    session: &mut Session,
    state: &mut AgentState,
    id: &str,
    peer: &str,
) -> Result<typstographe_core::content::shared::Publication, String> {
    session.check_peer(peer, &session.generation)?;
    if !state.busy.is_empty() {
        return Err("Composition IME en cours.".into());
    }
    let p = state
        .proposal
        .as_mut()
        .filter(|p| {
            p.id == id
                && p.status == "pending"
                && p.revision == session.revision
                && p.generation == session.generation
        })
        .ok_or("Proposition périmée.")?;
    let guard = session.guard();
    let publication = session.ensure_owner()?.calls(peer, &guard, &p.steps)?;
    session.record(&publication);
    p.status = "accepted".into();
    Ok(publication)
}
fn validate_request(method: &str, params: &Value) -> Result<(), String> {
    if method != "resource.read" {
        typstographe_mcp::validate_request(true, method, params)
            .map_err(|e| serde_json::to_string(&api::error_value(&e)).unwrap())?;
    } else if params
        .as_object()
        .is_none_or(|p| p.len() != 1 || !p.get("uri").is_some_and(Value::is_string))
    {
        return Err("Expected resource URI".into());
    }
    Ok(())
}
fn authorize(state: &AgentState, token: Option<&str>) -> Result<(), String> {
    if state.token.is_none() || state.token.as_deref() != token {
        Err("Invitation absente ou révoquée.".into())
    } else {
        Ok(())
    }
}
pub(crate) fn dispatch(
    session: &mut Session,
    state: &mut AgentState,
    method: &str,
    params: &Value,
) -> Result<Value, String> {
    validate_request(method, params)?;
    if state
        .proposal
        .as_ref()
        .is_some_and(|p| p.generation != session.generation)
    {
        return Err("Document replaced; reconnect invitation".into());
    }
    if method != "proposals.submit" && params.get("document").is_some() {
        check_reference(session, params)?;
    }
    let outcome = dispatch_outcome(session, state, method, params)?;
    Ok(
        json!({"version":1,"ok":true,"document":reference(session),"outcome":outcome,"permissions":{"proposals":true,"filesystem":false,"direct_edit_enabled":false,"human_approval_required":true}}),
    )
}
fn dispatch_outcome(
    session: &mut Session,
    state: &mut AgentState,
    method: &str,
    params: &Value,
) -> Result<Value, String> {
    if method == "proposals.get" || method == "proposals.cancel" {
        let mut p = proposal(state, session.revision)
            .filter(|p| params["proposal"] == p.id)
            .ok_or("Proposition absente.")?;
        if method == "proposals.cancel" && p.status == "pending" {
            p.status = "cancelled".into();
            state.proposal = Some(p.clone())
        }
        return Ok(json!(p));
    }
    if method == "document_types.list" {
        return Ok(api::types());
    }
    if method == "document_types.describe" {
        let kind = serde_json::from_value(params["type"].clone()).map_err(|e| e.to_string())?;
        return Ok(api::type_description(kind));
    }
    if method == "commands.list" {
        let mut c = api::catalogue();
        for entry in c["entries"].as_array_mut().unwrap() {
            entry["attached_direct"] = json!(typstographe_mcp::attached_support(
                entry["id"].as_str().unwrap()
            ));
            entry["attached_proposal"] = json!(
                entry["batch_step"] == true
                    && typstographe_mcp::attached_id(entry["id"].as_str().unwrap())
            );
        }
        return Ok(c);
    }
    if method == "capabilities.describe" {
        return api::describe(params["id"].as_str().ok_or("ID absent")?)
            .ok_or_else(|| "Unknown capability".into());
    }
    if method == "resource.read" {
        let uri = params["uri"].as_str().unwrap_or("");
        return match uri {
            "typstographe://guide" => {
                Ok(json!({"instructions":typstographe_mcp::instructions(true)}))
            }
            "typstographe://types" => Ok(api::types()),
            "typstographe://commands" => {
                dispatch_outcome(session, state, "commands.list", &json!({}))
            }
            "typstographe://content-capabilities" => {
                Ok(json!({"entries":catalogue::ALL,"commands":api::catalogue()}))
            }
            "typstographe://document" => {
                dispatch_outcome(session, state, "document.inspect", &json!({}))
            }
            "typstographe://models" => {
                let p = json!({"document":reference(session)});
                dispatch_outcome(session, state, "models.list", &p)
            }
            _ => Err("Unknown resource".into()),
        };
    }
    if method != "document.inspect"
        && method != "proposals.submit"
        && method != "spelling.suggestions"
    {
        check_reference(session, params)?
    }
    let confirmed: ContentState = session.ensure_owner()?.state().clone();
    if method == "document.inspect" {
        let peer = agent_peer(state);
        let context = session.ensure_owner()?.snapshot(&peer)?;
        return Ok(
            json!({"guard":session.guard(),"state":confirmed,"document_type":confirmed.settings.metadata.document_type(),"context":context}),
        );
    }
    if method == "context.get" {
        return session
            .ensure_owner()?
            .snapshot(&agent_peer(state))
            .map(|s| json!(s));
    }
    if method == "context.set" {
        if !state.busy.is_empty()
            || proposal(state, session.revision).is_some_and(|p| p.status == "pending")
        {
            return Err("Editing context is busy".into());
        }
        let selection =
            serde_json::from_value(params["selection"].clone()).map_err(|e| e.to_string())?;
        let styles =
            serde_json::from_value(params["typing_styles"].clone()).map_err(|e| e.to_string())?;
        let guard = session.guard();
        return session
            .ensure_owner()?
            .navigate(&agent_peer(state), &guard, selection, styles)
            .map(|s| json!(s));
    }
    if method == "proposals.submit" {
        let request_id = params["request_id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 128)
            .ok_or("request_id attendu")?;
        let fingerprint = {
            use sha2::{Digest, Sha256};
            Sha256::digest(serde_json::to_vec(params).map_err(|e| e.to_string())?)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        proposal(state, session.revision);
        if let Some((previous, response)) = state.seen.get(request_id) {
            if previous != &fingerprint {
                return Err("request_id déjà utilisé pour un lot différent.".into());
            }
            return Ok(state
                .proposal
                .as_ref()
                .filter(|p| p.request_id == request_id)
                .map(|p| json!(p))
                .unwrap_or_else(|| response.clone()));
        }
        if !state.busy.is_empty() {
            return Err("Composition IME en cours : aucune proposition différée.".into());
        }
        check_reference(session, params)?;
        if state.seen.len() >= 64
            || state
                .proposal
                .as_ref()
                .is_some_and(|p| p.status == "pending")
        {
            return Err("Limite de propositions atteinte ou lot en attente.".into());
        }
        let steps: Vec<Call> =
            serde_json::from_value(params["steps"].clone()).map_err(|e| e.to_string())?;
        if steps.is_empty() || steps.len() > 128 {
            return Err("Lot limité à 128 étapes.".into());
        }
        for step in &steps {
            validate_step(step)?;
        }
        let guard = session.guard();
        let prepared = session
            .ensure_owner()?
            .prepare_agent_calls(&guard, &steps)?;
        let after = prepared.state().clone();
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
            revision: session.revision,
            generation: session.generation.clone(),
            status: "pending".into(),
            before: confirmed.document,
            after: after.document.clone(),
            settings: after.settings,
            steps,
        };
        if serde_json::to_vec(&p).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
            return Err("Proposal exceeds review size limit".into());
        }
        if let Some(previous) = &state.proposal
            && let Some((_, receipt)) = state.seen.get_mut(&previous.request_id)
        {
            *receipt = json!({"id":previous.id,"revision":previous.revision,"status":previous.status,"requestId":previous.request_id});
        }
        state.seen.insert(
            request_id.into(),
            (
                fingerprint,
                json!({"id":p.id,"revision":p.revision,"status":p.status,"requestId":p.request_id}),
            ),
        );
        state.proposal = Some(p.clone());
        return Ok(json!(p));
    }
    if !typstographe_mcp::attached_support(method) {
        return Err("Tool unavailable in attached mode".into());
    }
    let mut p = params.clone();
    p.as_object_mut()
        .ok_or("Expected parameters")?
        .remove("document");
    api::check_params(method, &p)
        .map_err(|e| serde_json::to_string(&api::error_value(&e)).unwrap())?;
    api::query_state(&confirmed, &session.assets, method, &p)
        .map_err(|e| serde_json::to_string(&e).unwrap())
}

pub fn serve(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    typstographe_mcp::local::bridge(&path)
}

#[cfg(test)]
mod tests;
