//! Undoable publication of a human-composed comparison; saving remains explicit.
use super::*;
use typstographe_core::content::{
    comparison::{Decision, PreparedDecision},
    shared::Publication,
};

pub struct Preparation {
    plan: Comparison,
    guard: Guard,
    update: Vec<u8>,
    assets: typstographe_core::assets::Assets,
    choices: Vec<SourceSide>,
}
impl Preparation {
    pub fn prepare(self) -> Result<PreparedDecision, String> {
        let engine = typstographe_core::content::editing::EditingSession::open(
            self.guard,
            &self.update,
            self.assets,
        )
        .map_err(crate::owner::error)?;
        self.plan
            .prepare_choices(&engine, &self.choices)
            .map_err(|e| typstographe_i18n::content_error(&e))
    }
}
pub fn capture(
    s: &mut Session,
    a: &AgentState,
    peer: &str,
    id: &str,
    choices: &[SourceSide],
) -> Result<Preparation, String> {
    authorized(s, a, peer, id)?;
    let plan = s.comparisons[id].plan.clone();
    let guard = s.guard();
    plan.guard().check(&guard).map_err(crate::owner::error)?;
    let owner = s.ensure_owner()?;
    Ok(Preparation {
        plan,
        guard,
        update: owner.update_bytes(),
        assets: owner.assets().clone(),
        choices: choices.to_vec(),
    })
}
#[cfg(any(test, feature = "recipe-driver"))]
pub fn prepare(
    s: &mut Session,
    a: &AgentState,
    peer: &str,
    id: &str,
    choices: &[SourceSide],
) -> Result<PreparedDecision, String> {
    capture(s, a, peer, id, choices)?.prepare()
}
pub fn apply(
    s: &mut Session,
    a: &mut AgentState,
    peer: &str,
    id: &str,
    prepared: PreparedDecision,
) -> Result<(Value, Publication), String> {
    authorized(s, a, peer, id)?;
    // Keep choices and the frozen pair available if publication is refused.
    let mut review = s.comparisons[id].clone();
    s.ensure_owner()?
        .check_comparison(peer, &review.plan, &prepared)?;
    let publication = s
        .ensure_owner()?
        .publish_comparison(peer, &mut review.plan, prepared)?;
    s.record(&publication);
    review.finished = true;
    if let Origin::Proposal { .. } = &review.origin {
        let groups = review.plan.report(&s.guard()).groups;
        let proposal = a.proposal.as_mut().unwrap();
        proposal.decisions = groups
            .iter()
            .map(|g| json!({"group":g.id,"decision":g.decision}))
            .collect();
        proposal.working_revision = Some(s.revision);
        proposal.status = if groups.iter().all(|g| g.decision == Decision::Applied) {
            "accepted"
        } else if groups.iter().all(|g| g.decision == Decision::Refused) {
            "rejected"
        } else {
            "partially_accepted"
        }
        .into();
    }
    let result = payload(s, a, id, &review, false)?;
    s.comparisons.insert(id.into(), review);
    Ok((result, publication))
}
#[tauri::command]
pub async fn comparison_apply(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: State<'_, Bridge>,
    id: String,
    choices: Vec<SourceSide>,
) -> Result<Value, String> {
    let session = state.session.clone();
    let agent = state.agent.clone();
    let renderer = state.renderer.clone();
    let peer = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = capture(&mut *session.lock().map_err(|e| e.to_string())?, &*agent.lock().map_err(|e| e.to_string())?, &peer, &id, &choices)?;
        let ready = snapshot.prepare()?;
        let archive = ready.archive();
        renderer.validate(&archive.validate().map_err(crate::project::error)?, &archive.assets, crate::rendering::RenderContext { settings: &archive.state().settings, dependencies: archive.dependencies.clone() })?;
        let (mut report, publication) = apply(&mut *session.lock().map_err(|e| e.to_string())?, &mut *agent.lock().map_err(|e| e.to_string())?, &peer, &id, ready)?;
        report["publication"] = serde_json::to_value(&publication).map_err(|e| e.to_string())?;
        for resource in &publication.resources {
            let _ = app.emit("web-editor:asset", json!({"generation":publication.guard.document,"path":resource.path,"data":base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &resource.data)}));
        }
        let _ = app.emit("web-editor:change", json!({"from":"comparison","acceptedBy":peer,"generation":publication.guard.document,"revision":publication.guard.revision,"update":publication.update}));
        let _ = app.emit("comparison:changed", json!({"id":id}));
        let _ = app.emit("agent:changed", ());
        Ok(report)
    }).await.map_err(|e| e.to_string())?
}
