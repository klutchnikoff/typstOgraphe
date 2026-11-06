//! Guarded desktop preview protocol; document/export formats are independent.
use crate::{
    Bridge,
    error::CommandError,
    rendering::{
        self,
        delta::{Reference, Scope},
    },
};
use serde::Deserialize;
use tauri::{State, WebviewWindow};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub scope: Scope,
    pub base: Option<String>,
}

pub fn bind(bridge: &Bridge, peer: &str, generation: &str) -> Result<String, CommandError> {
    bridge.session()?.check_peer(peer, generation)?;
    Ok(bridge.renderer.2.lock()?.bind(peer, generation))
}
pub fn invalidate(bridge: &Bridge, peer: &str, scope: &Scope) -> Result<(), CommandError> {
    bridge.session()?.check_peer(peer, &scope.generation)?;
    bridge.renderer.2.lock()?.invalidate(peer, scope)
}
pub fn render(
    bridge: &Bridge,
    peer: &str,
    snapshot: crate::session::Snapshot,
    request: Request,
) -> Result<Vec<u8>, CommandError> {
    let (source, assets, settings, dependencies, ticket) = {
        let session = bridge.session()?;
        session.check_peer(peer, &request.scope.generation)?;
        let ticket =
            bridge
                .renderer
                .2
                .lock()?
                .begin(peer, request.scope, request.base.as_deref())?;
        let (source, assets, settings, dependencies) = session.render_snapshot(&snapshot)?;
        (source, assets, settings, dependencies, ticket)
    };
    // Never wait on a session lock while holding the compiler. Contention can
    // conservatively abandon a preview; authoritative document edits keep priority.
    let current = |_stage| {
        ticket.current()
            && bridge.session.try_lock().is_ok_and(|session| {
                session.generation == ticket.scope.generation
                    && session.revision == snapshot.revision
            })
    };
    if !current(rendering::Stage::Start) {
        return Err("stale".into());
    }
    let map = typstographe_core::content::source_mapped_with_metadata(
        &snapshot.document,
        &assets,
        &settings.metadata,
    )?
    .1;
    let (bytes, record) = bridge.renderer.delta_frame(
        source,
        assets,
        snapshot.revision,
        rendering::RenderContext {
            settings: &settings,
            dependencies,
        },
        map,
        &ticket,
        &current,
    )?;
    {
        let session = bridge.session()?;
        session.check_peer(peer, &ticket.scope.generation)?;
        if session.revision != snapshot.revision {
            return Err("stale".into());
        }
        bridge
            .renderer
            .2
            .lock()?
            .commit(peer, &ticket, record.clone())?;
        // Full-render diagnostics keep their existing API. Production queries
        // below address the exact frame and window, including its generation.
        *bridge.renderer.1.lock()? = Some(record.sync.clone());
    }
    Ok(bytes)
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Forward {
        position: usize,
    },
    Backward {
        location: typstographe_core::navigation::PagePoint,
    },
    Selection {
        from: usize,
        to: usize,
    },
}
pub fn query(
    bridge: &Bridge,
    peer: &str,
    frame: &Reference,
    revision: u64,
    query: Query,
) -> Result<serde_json::Value, CommandError> {
    let sync = {
        let session = bridge.session()?;
        session.check_peer(peer, &frame.generation)?;
        if session.revision != revision {
            return Err("stale".into());
        }
        bridge.renderer.2.lock()?.sync(peer, frame, revision)?
    };
    // The retained immutable compilation matches the displayed frame, even if
    // a new one is being prepared concurrently. Frontend tickets guard replies.
    let value = match query {
        Query::Forward { position } => serde_json::to_value(sync.forward(position))?,
        Query::Backward { location } => {
            if location.point.iter().any(|n| !n.is_finite()) {
                return Err("stale".into());
            }
            serde_json::to_value(sync.backward(location))?
        }
        Query::Selection { from, to } => {
            serde_json::to_value(sync.selection_highlights_exact(from..to))?
        }
    };
    Ok(value)
}
#[tauri::command(async)]
pub fn preview_bind(
    window: WebviewWindow,
    state: State<Bridge>,
    generation: String,
) -> Result<String, CommandError> {
    bind(&state, window.label(), &generation)
}
#[tauri::command(async)]
pub fn preview_invalidate(
    window: WebviewWindow,
    state: State<Bridge>,
    scope: Scope,
) -> Result<(), CommandError> {
    invalidate(&state, window.label(), &scope)
}
#[tauri::command]
pub async fn preview_delta(
    window: WebviewWindow,
    state: State<'_, Bridge>,
    snapshot: crate::session::Snapshot,
    request: Request,
) -> Result<tauri::ipc::Response, CommandError> {
    let peer = window.label().to_string();
    let state = state.inner().clone();
    let bytes =
        tauri::async_runtime::spawn_blocking(move || render(&state, &peer, snapshot, request))
            .await??;
    Ok(tauri::ipc::Response::new(bytes))
}
#[tauri::command(async)]
pub fn preview_sync(
    window: WebviewWindow,
    state: State<Bridge>,
    frame: Reference,
    revision: u64,
    query: Query,
) -> Result<serde_json::Value, CommandError> {
    self::query(&state, window.label(), &frame, revision, query)
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[test]
    fn scoped_navigation_is_window_generation_and_frame_specific() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::session::Session::fresh(dir.path().into());
        session.join("first");
        session.join("second");
        let generation = session.generation.clone();
        let snapshot = crate::session::Snapshot { revision: session.revision, document: serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Texte de navigation"}]}]})).unwrap(), settings: None };
        let bridge = Bridge {
            session: Arc::new(Mutex::new(session)),
            renderer: Arc::new(rendering::Renderer::default()),
            #[cfg(feature = "recipe-driver")]
            diagnostic: None,
            agent: Arc::new(Mutex::new(crate::agent::AgentState::default())),
        };
        let epoch = bind(&bridge, "first", &generation).unwrap();
        let scope = Scope {
            generation: generation.clone(),
            epoch: epoch.clone(),
            version: 1,
        };
        let bytes = render(
            &bridge,
            "first",
            snapshot.clone(),
            Request {
                scope: scope.clone(),
                base: None,
            },
        )
        .unwrap();
        let length = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let header: serde_json::Value = serde_json::from_slice(&bytes[8..8 + length]).unwrap();
        let reference = Reference {
            generation: generation.clone(),
            epoch,
            id: header["preview"]["delta"]["id"].as_str().unwrap().into(),
        };
        let forward = || Query::Forward { position: 3 };
        assert!(
            !query(&bridge, "first", &reference, snapshot.revision, forward())
                .unwrap()
                .is_null()
        );
        assert!(query(&bridge, "second", &reference, snapshot.revision, forward()).is_err());
        assert!(
            query(
                &bridge,
                "first",
                &reference,
                snapshot.revision + 1,
                forward()
            )
            .is_err()
        );
        invalidate(
            &bridge,
            "first",
            &Scope {
                version: 2,
                ..scope.clone()
            },
        )
        .unwrap();
        // Cancelled request cannot replace or evict the displayed map.
        assert!(
            render(
                &bridge,
                "first",
                snapshot.clone(),
                Request {
                    scope,
                    base: Some(reference.id.clone())
                }
            )
            .is_err()
        );
        assert!(
            !query(&bridge, "first", &reference, snapshot.revision, forward())
                .unwrap()
                .is_null()
        );
        bridge.renderer.2.lock().unwrap().detach("first");
        assert_ne!(
            bind(&bridge, "first", &generation).unwrap(),
            reference.epoch
        );
        assert!(query(&bridge, "first", &reference, snapshot.revision, forward()).is_err());
    }
}
