//! Assistance locale : découverte et propositions, jamais d'édition directe.
mod approval;
mod invitation;
mod transport;
use crate::{Bridge, session::Session, settings::Settings};
#[cfg(any(test, feature = "recipe-driver"))]
pub(crate) use approval::accept;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
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
    pub base: typstographe_core::content::archive::Archive,
    #[serde(skip_serializing)]
    pub candidate: typstographe_core::content::archive::Archive,
    #[serde(skip_serializing)]
    pub review: Option<String>,
    pub decisions: Vec<Value>,
    pub working_revision: Option<u64>,
}
#[derive(Default)]
pub struct AgentState {
    pub token: Option<String>,
    pub proposal: Option<Proposal>,
    pub seen: std::collections::BTreeMap<String, (String, Value)>,
    pub busy: std::collections::BTreeSet<String>,
}
fn proposal(state: &mut AgentState, session: &Session) -> Option<Proposal> {
    if let Some(p) = &mut state.proposal
        && p.status == "pending"
        && p.review
            .as_ref()
            .and_then(|id| session.comparisons.get(id))
            .map_or(p.revision != session.revision, |r| {
                r.plan.guard() != &session.guard()
            })
    {
        p.status = "stale".into()
    }
    state.proposal.clone()
}
#[tauri::command]
pub fn agent_state(state: State<Bridge>) -> Result<Value, String> {
    let session = state.session.lock().map_err(|e| e.to_string())?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    Ok(json!({"connected":agent.token.is_some(),"proposal":proposal(&mut agent,&session)}))
}
#[tauri::command]
pub fn agent_revoke(app: tauri::AppHandle, state: State<Bridge>) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    revoke(&mut session, &mut agent)?;
    let _ = app.emit("agent:changed", ());
    Ok(())
}
fn revoke(session: &mut Session, agent: &mut AgentState) -> Result<(), String> {
    if let Some(owner) = &mut session.owner {
        owner.close(&agent_peer(agent))?;
    }
    agent.token = None;
    agent.proposal = None;
    agent.seen.clear();
    Ok(())
}
#[tauri::command]
pub fn agent_reject(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: State<Bridge>,
    id: String,
) -> Result<(), String> {
    let mut session = state.session.lock().map_err(|e| e.to_string())?;
    let mut agent = state.agent.lock().map_err(|e| e.to_string())?;
    session.check_peer(window.label(), &session.generation)?;
    let review = agent
        .proposal
        .as_ref()
        .filter(|p| p.id == id)
        .and_then(|p| p.review.clone());
    if let Some(review) = review {
        let groups = session
            .comparisons
            .get(&review)
            .ok_or_else(|| typstographe_i18n::text("web-comparison-unavailable"))?
            .plan
            .report(&session.guard())
            .groups
            .into_iter()
            .filter(|g| g.decision == typstographe_core::content::comparison::Decision::Pending)
            .map(|g| g.id)
            .collect::<Vec<_>>();
        if !groups.is_empty() {
            session
                .comparisons
                .get_mut(&review)
                .unwrap()
                .plan
                .refuse(&groups)
                .map_err(crate::owner::error)?;
        }
        reject(&mut agent, &id)?;
        let _ = app.emit("comparison:changed", json!({"id":review}));
    } else {
        reject(&mut agent, &id)?;
    }
    let _ = app.emit("agent:changed", ());
    Ok(())
}
fn reject(agent: &mut AgentState, id: &str) -> Result<(), String> {
    let p = agent
        .proposal
        .as_mut()
        .filter(|p| p.id == id)
        .ok_or("Proposition absente.")?;
    if p.status == "pending" {
        p.status = "rejected".into();
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
    let session = state.session.clone();
    let agent = state.agent.clone();
    let renderer = state.renderer.clone();
    let peer = window.label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        approval::accept_with_notification(
            &session,
            &agent,
            &id,
            &peer,
            |candidate, assets| {
                let context = session.lock().map_err(|e| e.to_string())?.render_context(&candidate.settings);
                renderer.validate(
                    &crate::schema::source(&candidate.after, assets)?,
                    assets,
                    context,
                )
            },
            |publication| {
                app.emit(
                    "web-editor:change",
                    json!({"from":"agent","acceptedBy":peer,
                        "generation":publication.guard.document,"revision":publication.guard.revision,"update":publication.update}),
                ).map_err(|e| e.to_string())?;
                app.emit("agent:changed", ()).map_err(|e| e.to_string())
            },
        ).map(|_| ())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn agent_invite(app: tauri::AppHandle, state: State<Bridge>) -> Result<Value, String> {
    let root = crate::preferences::root(&app)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let invitation = Invitation {
        address: listener.local_addr().unwrap().to_string(),
        token: uuid::Uuid::new_v4().simple().to_string(),
    };
    let file = root.join("invitation.json");
    invitation::install(&state.session, &state.agent, &root, &invitation)?;
    let session = state.session.clone();
    let agent = state.agent.clone();
    let token = invitation.token.clone();
    let endpoint = invitation.address.clone();
    let binary = std::env::current_exe().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let connections = agent.clone();
        accept_while_invited(&listener, &connections, &token, |stream| {
            let session = session.clone();
            let agent = agent.clone();
            let app = app.clone();
            std::thread::spawn(move || serve_connection(stream, session, agent, app));
        })
    });
    Ok(
        json!({"binary":binary,"arguments":["--mcp","--invitation",file],"endpoint":endpoint,"permissions":{"proposals":true,"filesystem":false,"approval":"guarded human decisions only"}}),
    )
}
/// Stops accepting connections as soon as this invitation is revoked or replaced.
fn accept_while_invited(
    listener: &TcpListener,
    agent: &Mutex<AgentState>,
    token: &str,
    mut serve: impl FnMut(TcpStream),
) {
    listener.set_nonblocking(true).ok();
    loop {
        if agent
            .lock()
            .map(|a| a.token.as_deref() != Some(token))
            .unwrap_or(true)
        {
            break;
        }
        match listener.accept() {
            Ok((stream, _)) => serve(stream),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(std::time::Duration::from_millis(80))
            }
            Err(_) => break,
        }
    }
}
fn serve_connection(
    stream: TcpStream,
    session: Arc<Mutex<Session>>,
    agent: Arc<Mutex<AgentState>>,
    app: tauri::AppHandle,
) {
    transport::connection(stream, &session, &agent, || {
        let _ = app.emit("agent:changed", ());
    });
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
    if p.review.is_some() {
        return Err(typstographe_i18n::text("web-comparison-use-review"));
    }
    let guard = session.guard();
    let mut plan = typstographe_core::content::comparison::Comparison::new(
        p.base.clone(),
        p.candidate.clone(),
        typstographe_core::content::comparison::SourceSide::After,
        guard.clone(),
    )
    .map_err(crate::owner::error)?;
    let ids = plan
        .report(&guard)
        .groups
        .iter()
        .map(|g| g.id)
        .collect::<Vec<_>>();
    let publication = if ids.is_empty() {
        session.ensure_owner()?.calls(peer, &guard, &[])?
    } else {
        let prepared = session.ensure_owner()?.prepare_comparison(&plan, &ids)?;
        session
            .ensure_owner()?
            .publish_comparison(peer, &mut plan, prepared)?
    };
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
        let mut p = proposal(state, session)
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
        if !state.busy.is_empty() || proposal(state, session).is_some_and(|p| p.status == "pending")
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
        proposal(state, session);
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
        let base = session.stored_project()?.current().clone();
        let assets = typstographe_core::content::retained_assets(
            &after.document,
            &after.settings.metadata,
            &after.settings.template,
            prepared.assets(&session.assets),
            &base.dependencies,
        )?;
        let candidate = crate::project::archive(
            after.clone(),
            prepared.checkpoint(),
            assets,
            base.dependencies.clone(),
        );
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
            base,
            candidate,
            review: None,
            decisions: vec![],
            working_revision: None,
        };
        if serde_json::to_vec(&p).map_err(|e| e.to_string())?.len() > 8 * 1024 * 1024 {
            return Err("Proposal exceeds review size limit".into());
        }
        if let Some(previous) = &state.proposal
            && let Some((_, receipt)) = state.seen.get_mut(&previous.request_id)
        {
            *receipt = json!({"id":previous.id,"revision":previous.revision,"status":previous.status,"requestId":previous.request_id,"decisions":previous.decisions,"workingRevision":previous.working_revision});
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
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
