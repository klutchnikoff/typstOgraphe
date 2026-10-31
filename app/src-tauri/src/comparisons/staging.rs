//! A historical composition must be reviewed against the captured current document.
use super::*;
use typstographe_core::content::{archive::Archive, comparison::PreparedDecision};

pub struct Preparation {
    parent: String,
    guard: Guard,
    current: Archive,
    snapshot: commit::Preparation,
}
pub struct PreparedReview {
    parent: String,
    decision: PreparedDecision,
    plan: Comparison,
}
impl Preparation {
    pub fn prepare(self) -> Result<PreparedReview, String> {
        let decision = self.snapshot.prepare()?;
        let plan = Comparison::new(
            self.current,
            decision.archive().clone(),
            SourceSide::After,
            self.guard,
        )
        .map_err(crate::owner::error)?;
        Ok(PreparedReview {
            parent: self.parent,
            decision,
            plan,
        })
    }
}
impl PreparedReview {
    pub fn archive(&self) -> &Archive {
        self.decision.archive()
    }
}
pub fn capture(
    s: &mut Session,
    a: &AgentState,
    peer: &str,
    id: &str,
    choices: &[SourceSide],
) -> Result<Preparation, String> {
    let snapshot = commit::capture(s, a, peer, id, choices)?;
    if s.comparisons[id].context != Context::Historical {
        return Err(error("web-comparison-unavailable"));
    }
    Ok(Preparation {
        parent: id.into(),
        guard: s.guard(),
        current: s.stored_project()?.current().clone(),
        snapshot,
    })
}
pub fn finish(
    s: &mut Session,
    a: &AgentState,
    peer: &str,
    prepared: PreparedReview,
) -> Result<Value, String> {
    authorized(s, a, peer, &prepared.parent)?;
    let parent = s.comparisons[&prepared.parent].clone();
    if parent.context != Context::Historical {
        return Err(error("web-comparison-unavailable"));
    }
    s.ensure_owner()?
        .check_comparison(peer, &parent.plan, &prepared.decision)?;
    let review = Review {
        plan: prepared.plan,
        context: Context::Composition,
        parent: Some(prepared.parent.clone()),
        peer: Some(peer.into()),
        origin: parent.origin,
        finished: false,
        before_name: None,
        after_name: None,
        created: std::time::Instant::now(),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let result = payload(s, a, &id, &review, true)?;
    // Only the most recent preparation for this parent may be accepted.
    s.comparisons
        .retain(|_, r| r.parent.as_deref() != Some(&prepared.parent));
    s.comparisons.insert(id, review);
    Ok(result)
}
#[tauri::command]
pub async fn comparison_prepare(
    window: tauri::WebviewWindow,
    state: State<'_, Bridge>,
    id: String,
    choices: Vec<SourceSide>,
) -> Result<Value, String> {
    let session = state.session.clone();
    let agent = state.agent.clone();
    let renderer = state.renderer.clone();
    let peer = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = capture(
            &mut *session.lock().map_err(|e| e.to_string())?,
            &*agent.lock().map_err(|e| e.to_string())?,
            &peer,
            &id,
            &choices,
        )?;
        let ready = snapshot.prepare()?;
        let archive = ready.archive();
        renderer.validate(
            &archive.validate().map_err(crate::project::error)?,
            &archive.assets,
            crate::rendering::RenderContext {
                settings: &archive.state().settings,
                dependencies: archive.dependencies.clone(),
            },
        )?;
        finish(
            &mut *session.lock().map_err(|e| e.to_string())?,
            &*agent.lock().map_err(|e| e.to_string())?,
            &peer,
            ready,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
