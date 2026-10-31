//! Shared frozen viewer and guarded human decisions. No filesystem publication.
use crate::{Bridge, agent::AgentState, session::Session};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{Emitter, State};
use typstographe_core::content::{
    comparison::{Comparison, Decision, PreparedDecision, SourceSide},
    contract::Guard,
};
#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Origin {
    Version {
        id: uuid::Uuid,
        #[serde(default)]
        against: Option<uuid::Uuid>,
    },
    Proposal {
        id: String,
    },
}
#[derive(Clone)]
pub struct Review {
    pub plan: Comparison,
    peer: Option<String>,
    origin: Origin,
    read_only: bool,
    before_name: Option<String>,
    after_name: Option<String>,
    created: std::time::Instant,
}
fn error(key: &str) -> String {
    typstographe_i18n::text(key)
}
fn check(s: &Session, peer: &str, r: &Review) -> Result<(), String> {
    s.check_peer(peer, &r.plan.guard().document)?;
    if r.peer.as_deref().is_some_and(|p| p != peer) {
        return Err(error("web-comparison-unavailable"));
    }
    Ok(())
}
fn payload(
    s: &Session,
    a: &AgentState,
    id: &str,
    r: &Review,
    content: bool,
) -> Result<Value, String> {
    let mut value = json!({"id":id,"report":r.plan.report(&s.guard())});
    value["report"]["read_only"] = json!(r.read_only);
    value["report"]["before_name"] = json!(r.before_name);
    value["report"]["after_name"] = json!(r.after_name);
    value["report"]["closed"] = json!(
        matches!(&r.origin, Origin::Proposal {id:proposal} if a.token.is_none() || !a.proposal.as_ref().is_some_and(|p|p.id==*proposal && p.review.as_deref()==Some(id) && p.status=="pending"))
    );
    if content {
        value["before"] = r.plan.before().view().map_err(crate::project::error)?;
        value["after"] = r.plan.after().view().map_err(crate::project::error)?;
    }
    Ok(value)
}
pub fn open(
    s: &mut Session,
    agent: &mut AgentState,
    peer: &str,
    guard: &Guard,
    origin: Origin,
) -> Result<Value, String> {
    s.check_peer(peer, &guard.document)?;
    guard.check(&s.guard()).map_err(crate::owner::error)?;
    if let Origin::Proposal { id } = &origin {
        let p = agent
            .proposal
            .as_ref()
            .filter(|p| p.id == *id)
            .ok_or_else(|| error("web-comparison-unavailable"))?;
        if let Some(review) = &p.review
            && let Some(r) = s.comparisons.get(review)
        {
            check(s, peer, r)?;
            return payload(s, agent, review, r, true);
        }
        if p.generation != s.generation {
            return Err(error("web-comparison-stale"));
        }
    }
    let current = s.stored_project()?.current().clone();
    let (mut before, mut after, source, owner) = match &origin {
        Origin::Version { id, against } => (
            s.project
                .get()
                .unwrap()
                .owner
                .version(*id)
                .map_err(crate::project::error)?,
            if let Some(against) = against {
                s.project
                    .get()
                    .unwrap()
                    .owner
                    .version(*against)
                    .map_err(crate::project::error)?
            } else {
                current
            },
            SourceSide::Before,
            Some(peer.into()),
        ),
        Origin::Proposal { .. } => {
            let p = agent.proposal.as_ref().unwrap();
            (p.base.clone(), p.candidate.clone(), SourceSide::After, None)
        }
    };
    let (mut before_name, mut after_name) = (None, None);
    if let Origin::Version { id, against } = &origin {
        let records = s.project.get().unwrap().owner.versions();
        let selected = records.iter().find(|v| v.id == *id).unwrap();
        before_name = Some(selected.details.name.clone());
        if let Some(other) = against.and_then(|id| records.iter().find(|v| v.id == id)) {
            after_name = Some(other.details.name.clone());
            if other.created_at < selected.created_at {
                std::mem::swap(&mut before, &mut after);
                std::mem::swap(&mut before_name, &mut after_name);
            }
        }
    }
    let expected = match &origin {
        Origin::Version { .. } => guard.clone(),
        Origin::Proposal { .. } => {
            let p = agent.proposal.as_ref().unwrap();
            Guard {
                document: p.generation.clone(),
                revision: p.revision,
            }
        }
    };
    let plan = Comparison::new(before, after, source, expected).map_err(crate::owner::error)?;
    // A delayed old opening/closing must not evict a newer viewer from the same window.
    let peers: Vec<_> = s
        .comparisons
        .iter()
        .filter(|(_, r)| match (&r.origin, &origin) {
            (Origin::Version { .. }, Origin::Version { .. }) => r.peer.as_deref() == Some(peer),
            (Origin::Proposal { .. }, Origin::Proposal { .. }) => true,
            _ => false,
        })
        .map(|(id, r)| (id.clone(), r.created))
        .collect();
    if peers.len() >= 8
        && let Some((oldest, _)) = peers.into_iter().min_by_key(|(_, created)| *created)
    {
        s.comparisons.remove(&oldest);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let read_only = matches!(
        &origin,
        Origin::Version {
            against: Some(_),
            ..
        }
    );
    let r = Review {
        plan,
        peer: owner,
        origin,
        read_only,
        before_name,
        after_name,
        created: std::time::Instant::now(),
    };
    if matches!(r.origin, Origin::Proposal { .. }) {
        agent.proposal.as_mut().unwrap().review = Some(id.clone());
    }
    let result = payload(s, agent, &id, &r, true)?;
    s.comparisons.insert(id, r);
    Ok(result)
}
pub fn read(s: &Session, a: &AgentState, peer: &str, id: &str) -> Result<Value, String> {
    let r = s
        .comparisons
        .get(id)
        .ok_or_else(|| error("web-comparison-unavailable"))?;
    check(s, peer, r)?;
    payload(s, a, id, r, false)
}
fn authorized(s: &Session, a: &AgentState, peer: &str, id: &str) -> Result<(), String> {
    let r = s
        .comparisons
        .get(id)
        .ok_or_else(|| error("web-comparison-unavailable"))?;
    check(s, peer, r)?;
    if r.read_only {
        return Err(error("web-comparison-read-only"));
    }
    if !a.busy.is_empty() {
        return Err(error("web-comparison-composing"));
    }
    if let Origin::Proposal { id: proposal } = &r.origin
        && (a.token.is_none()
            || !a.proposal.as_ref().is_some_and(|p| {
                p.id == *proposal && p.review.as_deref() == Some(id) && p.status == "pending"
            }))
    {
        return Err(error("web-comparison-unavailable"));
    }
    Ok(())
}
pub fn prepare(
    s: &mut Session,
    a: &AgentState,
    peer: &str,
    id: &str,
    ids: &[usize],
) -> Result<PreparedDecision, String> {
    authorized(s, a, peer, id)?;
    let plan = s.comparisons[id].plan.clone();
    s.ensure_owner()?.prepare_comparison(&plan, ids)
}
pub fn decide(
    s: &mut Session,
    a: &mut AgentState,
    peer: &str,
    id: &str,
    ids: &[usize],
    prepared: Option<PreparedDecision>,
) -> Result<
    (
        Value,
        Option<typstographe_core::content::shared::Publication>,
    ),
    String,
> {
    authorized(s, a, peer, id)?;
    let project = if let Some(p) = &prepared {
        Some(
            s.stored_project()?
                .with_current(p.archive().clone())
                .map_err(crate::project::error)?,
        )
    } else {
        None
    };
    let mut review = s.comparisons.remove(id).unwrap();
    let result = (|| {
        let publication = if let Some(prepared) = prepared {
            let pub_ = s
                .ensure_owner()?
                .publish_comparison(peer, &mut review.plan, prepared)?;
            s.record(&pub_);
            let saved = s.project.get().is_some_and(|c| c.saved_origin);
            s.project =
                std::sync::OnceLock::from(crate::project::Context::new(project.unwrap(), saved));
            Some(pub_)
        } else {
            review.plan.refuse(ids).map_err(crate::owner::error)?;
            None
        };
        if let Origin::Proposal { .. } = &review.origin {
            let groups = review.plan.report(&s.guard()).groups;
            let proposal = a.proposal.as_mut().unwrap();
            proposal.decisions = groups
                .iter()
                .map(|g| json!({"group":g.id,"decision":g.decision}))
                .collect();
            proposal.working_revision = Some(s.revision);
            if !groups.iter().any(|g| g.decision == Decision::Pending) {
                a.proposal.as_mut().unwrap().status =
                    if groups.iter().all(|g| g.decision == Decision::Applied) {
                        "accepted"
                    } else if groups.iter().all(|g| g.decision == Decision::Refused) {
                        "rejected"
                    } else {
                        "partially_accepted"
                    }
                    .into();
            }
        }
        Ok((payload(s, a, id, &review, false)?, publication))
    })();
    s.comparisons.insert(id.into(), review);
    result
}
#[tauri::command(async)]
pub fn comparison_open(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    guard: Guard,
    origin: Origin,
) -> Result<Value, String> {
    open(
        &mut *state.session.lock().map_err(|e| e.to_string())?,
        &mut *state.agent.lock().map_err(|e| e.to_string())?,
        window.label(),
        &guard,
        origin,
    )
}
#[tauri::command(async)]
pub fn comparison_read(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    id: String,
) -> Result<Value, String> {
    read(
        &*state.session.lock().map_err(|e| e.to_string())?,
        &*state.agent.lock().map_err(|e| e.to_string())?,
        window.label(),
        &id,
    )
}
pub fn close(s: &mut Session, peer: &str, id: &str) -> Result<(), String> {
    if let Some(r) = s.comparisons.get(id) {
        check(s, peer, r)?;
        if r.peer.is_some() {
            s.comparisons.remove(id);
        }
    }
    Ok(())
}
#[tauri::command(async)]
pub fn comparison_close(
    window: tauri::WebviewWindow,
    state: State<Bridge>,
    id: String,
) -> Result<(), String> {
    close(
        &mut *state.session.lock().map_err(|e| e.to_string())?,
        window.label(),
        &id,
    )
}
#[tauri::command]
pub async fn comparison_preview(
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    id: String,
    side: SourceSide,
) -> Result<crate::rendering::Preview, String> {
    let archive = {
        let s = state.session.lock().map_err(|e| e.to_string())?;
        let r = s
            .comparisons
            .get(&id)
            .ok_or_else(|| error("web-comparison-unavailable"))?;
        check(&s, window.label(), r)?;
        match side {
            SourceSide::Before => r.plan.before().clone(),
            SourceSide::After => r.plan.after().clone(),
        }
    };
    tauri::async_runtime::spawn_blocking(move || crate::versions::preview(archive))
        .await
        .map_err(|e| e.to_string())?
}
#[derive(Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Apply,
    Refuse,
}
#[tauri::command]
pub async fn comparison_decide(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: State<'_, Bridge>,
    id: String,
    groups: Vec<usize>,
    action: Action,
) -> Result<Value, String> {
    let session = state.session.clone();
    let agent = state.agent.clone();
    let renderer = state.renderer.clone();
    let peer = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let prepared = if matches!(action, Action::Apply) {
            let ready = prepare(
                &mut *session.lock().map_err(|e| e.to_string())?,
                &*agent.lock().map_err(|e| e.to_string())?,
                &peer, &id, &groups,
            )?;
            let archive = ready.archive();
            renderer.validate(
                &archive.validate().map_err(crate::project::error)?, &archive.assets,
                crate::rendering::RenderContext {
                    settings: &archive.state().settings,
                    dependencies: archive.dependencies.clone(),
                },
            )?;
            Some(ready)
        } else { None };
        let mut s = session.lock().map_err(|e| e.to_string())?;
        let mut a = agent.lock().map_err(|e| e.to_string())?;
        let (mut report, publication) = decide(&mut s, &mut a, &peer, &id, &groups, prepared)?;
        // The response also carries the confirmed delta: the accepting window
        // never depends on event delivery to install its publication.
        report["publication"] = serde_json::to_value(&publication).map_err(|e|e.to_string())?;
        if let Some(p) = publication {
            for resource in p.resources {
                let _ = app.emit("web-editor:asset", json!({
                    "generation": p.guard.document, "path": resource.path,
                    "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, resource.data),
                }));
            }
            let _ = app.emit("web-editor:change", json!({
                "from": "comparison", "acceptedBy": peer, "generation": p.guard.document,
                "revision": p.guard.revision, "update": p.update,
            }));
        }
        let _ = app.emit("comparison:changed", json!({"id": id}));
        let _ = app.emit("agent:changed", ());
        Ok(report)
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "comparisons_tests.rs"]
mod tests;
