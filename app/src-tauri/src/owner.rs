//! One confirmed Rust owner and one private history per authenticated window.
use std::collections::HashMap;
use typstographe_core::{
    assets::Assets,
    content::{
        commands::{Command, Style},
        contract::{Error, Guard},
        editing::{ContextId, ContextSnapshot, Direction, EditingSession, Selection},
        shared::Publication,
    },
};

pub struct Owner {
    engine: EditingSession,
    peers: HashMap<String, ContextId>,
}
#[derive(serde::Serialize)]
pub struct Receipt {
    pub revision: u64,
    pub update: Vec<u8>,
    pub history: ContextSnapshot,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextRequest {
    pub generation: String,
    pub revision: u64,
    pub frontier: Vec<u8>,
    pub selection: Option<Selection>,
    pub typing_styles: Vec<Style>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryViewRequest {
    pub frontier: Vec<u8>,
    pub selection: Option<Selection>,
    pub typing_styles: Vec<Style>,
}
pub fn error(error: Error) -> String {
    serde_json::to_string(&error).expect("typed content error")
}
impl Owner {
    pub fn open(guard: Guard, update: &[u8], assets: Assets) -> Result<Self, String> {
        Ok(Self {
            engine: EditingSession::open(guard, update, assets).map_err(error)?,
            peers: HashMap::new(),
        })
    }
    fn context(&mut self, peer: &str) -> Result<ContextId, String> {
        if let Some(id) = self.peers.get(peer) {
            return Ok(*id);
        }
        let id = self
            .engine
            .open_context(&self.engine.guard().clone())
            .map_err(error)?;
        self.peers.insert(peer.into(), id);
        Ok(id)
    }
    pub fn state(&self) -> &typstographe_core::content::contract::State {
        self.engine.state()
    }
    pub fn prepare_comparison(
        &self,
        plan: &typstographe_core::content::comparison::Comparison,
        ids: &[usize],
    ) -> Result<typstographe_core::content::comparison::PreparedDecision, String> {
        plan.prepare(&self.engine, ids)
            .map_err(|e| typstographe_i18n::content_error(&e))
    }
    pub fn publish_comparison(
        &mut self,
        peer: &str,
        plan: &mut typstographe_core::content::comparison::Comparison,
        prepared: typstographe_core::content::comparison::PreparedDecision,
    ) -> Result<Publication, String> {
        let context = self.context(peer)?;
        plan.publish(&mut self.engine, context, prepared)
            .map_err(|e| typstographe_i18n::content_error(&e))
    }
    pub fn check_comparison(
        &mut self,
        peer: &str,
        plan: &typstographe_core::content::comparison::Comparison,
        prepared: &typstographe_core::content::comparison::PreparedDecision,
    ) -> Result<(), String> {
        plan.check_prepared(&self.engine, prepared)
            .map_err(|e| typstographe_i18n::content_error(&e))?;
        let context = self.context(peer)?;
        self.engine
            .check_publication_context(plan.guard(), context)
            .map_err(|e| typstographe_i18n::content_error(&e))
    }
    pub fn prepare_agent_calls(
        &self,
        guard: &Guard,
        calls: &[typstographe_core::content::api::Call],
    ) -> Result<typstographe_core::content::shared::PreparedUpdate, String> {
        self.engine
            .prepare_resolved_calls(guard, calls, |state, call| {
                let command = typstographe_core::content::api::resolve_call(state, call)?;
                if !typstographe_mcp::attached_command(&command) {
                    return Err(typstographe_core::content::api::invalid(
                        "attached_body_target",
                    ));
                }
                Ok(command)
            })
            .map_err(error)
    }
    pub fn calls(
        &mut self,
        peer: &str,
        guard: &Guard,
        calls: &[typstographe_core::content::api::Call],
    ) -> Result<Publication, String> {
        let id = self.context(peer)?;
        let prepared = self.engine.prepare_calls(guard, calls).map_err(error)?;
        self.engine
            .publish_prepared(guard, id, prepared)
            .map_err(error)
    }
    pub fn navigate(
        &mut self,
        peer: &str,
        guard: &Guard,
        selection: Option<Selection>,
        styles: Vec<Style>,
    ) -> Result<ContextSnapshot, String> {
        let id = self.context(peer)?;
        self.engine
            .set_context(guard, id, selection, styles)
            .map_err(error)
    }
    pub fn assets(&self) -> &Assets {
        self.engine.assets()
    }
    pub fn update_bytes(&self) -> Vec<u8> {
        self.engine.update()
    }
    pub fn snapshot(&mut self, peer: &str) -> Result<ContextSnapshot, String> {
        let id = self.context(peer)?;
        self.engine
            .context(&self.engine.guard().clone(), id)
            .map_err(error)
    }
    pub fn set_context(
        &mut self,
        peer: &str,
        request: ContextRequest,
    ) -> Result<ContextSnapshot, String> {
        let guard = Guard {
            document: request.generation,
            revision: request.revision,
        };
        self.engine
            .check_frontier(&guard, &request.frontier)
            .map_err(error)?;
        let id = self.context(peer)?;
        self.engine
            .set_context(&guard, id, request.selection, request.typing_styles)
            .map_err(error)
    }
    pub fn reset(&mut self, peer: &str) -> Result<ContextSnapshot, String> {
        if let Some(id) = self.peers.remove(peer) {
            self.engine
                .close_context(&self.engine.guard().clone(), id)
                .map_err(error)?;
        }
        self.snapshot(peer)
    }
    pub fn close(&mut self, peer: &str) -> Result<(), String> {
        if let Some(id) = self.peers.remove(peer) {
            self.engine
                .close_context(&self.engine.guard().clone(), id)
                .map_err(error)?;
        }
        Ok(())
    }
    pub fn draft(
        &mut self,
        peer: &str,
    ) -> Result<Option<typstographe_core::content::editing::RejectedDraft>, String> {
        let id = self.context(peer)?;
        self.engine
            .rejected_draft(self.engine.guard(), id)
            .map(|draft| draft.cloned())
            .map_err(error)
    }
    pub fn update(
        &mut self,
        peer: &str,
        update: &[u8],
        group: Option<u64>,
        capture: bool,
        view: Option<HistoryViewRequest>,
    ) -> Result<Publication, String> {
        let id = self.context(peer)?;
        let guard = self.engine.guard().clone();
        if let Some(view) = view
            && capture
            && self.engine.check_frontier(&guard, &view.frontier).is_ok()
        {
            self.engine
                .set_context(&guard, id, view.selection, view.typing_styles)
                .map_err(error)?;
        }
        if !capture {
            return self
                .engine
                .publish_updates(&guard, &[update.to_vec()])
                .map_err(error);
        }
        match group {
            Some(group) => self
                .engine
                .execute_grouped_update(&guard, id, update, group),
            None => self.engine.execute_updates(&guard, id, &[update.to_vec()]),
        }
        .map_err(error)
    }
    pub fn commands(
        &mut self,
        peer: &str,
        guard: &Guard,
        commands: &[Command],
    ) -> Result<Publication, String> {
        let id = self.context(peer)?;
        self.engine.execute(guard, id, commands).map_err(error)
    }
    pub fn history(&mut self, peer: &str, direction: Direction) -> Result<Publication, String> {
        let id = self.context(peer)?;
        self.engine
            .history(&self.engine.guard().clone(), id, direction)
            .map_err(error)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::session::Session;
    use typstographe_core::content::{
        commands::{TextTarget, TextZone},
        contract::{Settings, State},
        shared::{SharedDocument, SharedSession},
    };
    fn seed(session: &mut Session) {
        session.join("a");
        session.join("b");
        let state = State::new(serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Base"}]},{"type":"paragraph"}]})).unwrap(), Settings::default());
        session
            .relay("a", &SharedDocument::from_state(&state).unwrap().update())
            .unwrap();
    }
    fn addition(session: &Session, text: &str) -> Vec<u8> {
        let shared = SharedDocument::from_updates(&session.updates).unwrap();
        let mut replica =
            SharedSession::open(session.guard(), &shared.update(), session.assets.clone()).unwrap();
        let prepared = replica
            .prepare_commands(
                &session.guard(),
                &[Command::Insert {
                    target: TextTarget {
                        zone: TextZone::Body { path: vec![0] },
                        from: 0,
                        to: 0,
                    },
                    text: text.into(),
                    marks: vec![],
                }],
            )
            .unwrap();
        replica.publish(prepared).unwrap().update
    }
    fn view_request(session: &Session, offset: u32) -> ContextRequest {
        use typstographe_core::content::editing::TextPoint;
        let frontier = SharedDocument::from_updates(&session.updates)
            .unwrap()
            .frontier();
        let point = TextPoint {
            zone: TextZone::Body { path: vec![0] },
            offset,
        };
        ContextRequest {
            generation: session.generation.clone(),
            revision: session.revision,
            frontier,
            selection: Some(Selection::Text {
                anchor: point.clone(),
                head: point,
            }),
            typing_styles: vec![Style::Bold],
        }
    }
    #[test]
    fn view_updates_are_guarded_private_and_cannot_create_a_document_revision() {
        let temp = tempfile::tempdir().unwrap();
        let mut session = Session::fresh(temp.path().into());
        seed(&mut session);
        let bytes = session.updates.clone();
        let revision = session.revision;
        let request = view_request(&session, 2);
        let selected = session
            .ensure_owner()
            .unwrap()
            .set_context("a", request)
            .unwrap();
        assert_eq!(selected.typing_styles, vec![Style::Bold]);
        assert_eq!(selected.undo_steps, 0);
        assert!(
            session
                .ensure_owner()
                .unwrap()
                .snapshot("b")
                .unwrap()
                .selection
                .is_none()
        );
        assert_eq!(session.revision, revision);
        assert_eq!(session.updates, bytes);
        let mut stale = view_request(&session, 0);
        stale.revision -= 1;
        assert!(
            session
                .ensure_owner()
                .unwrap()
                .set_context("a", stale)
                .is_err()
        );
        let mut different = view_request(&session, 0);
        different.frontier = vec![0, 0];
        assert!(
            session
                .ensure_owner()
                .unwrap()
                .set_context("a", different)
                .is_err()
        );
        assert_eq!(
            session.ensure_owner().unwrap().snapshot("a").unwrap(),
            selected
        );
        assert_eq!(session.updates, bytes);
    }
    #[test]
    fn a_history_view_is_causally_checked_and_cannot_retarget_a_stale_path() {
        let temp = tempfile::tempdir().unwrap();
        let mut s = Session::fresh(temp.path().into());
        seed(&mut s);
        let before = view_request(&s, 2);
        let expected = before.selection.clone();
        let update = addition(&s, "Added");
        let view = HistoryViewRequest {
            frontier: before.frontier,
            selection: before.selection,
            typing_styles: before.typing_styles,
        };
        s.relay_view("a", &update, Some(1), true, Some(view))
            .unwrap();
        s.history("a", Direction::Undo).unwrap();
        let restored = s.ensure_owner().unwrap().snapshot("a").unwrap();
        assert_eq!(restored.selection, expected);
        assert_eq!(restored.typing_styles, vec![Style::Bold]);

        // A remote edit makes the browser's revision-relative path untrustworthy.
        // Its outdated view is ignored; text still goes through normal validation.
        let stale = view_request(&s, 0);
        let update = addition(&s, "Remote");
        s.relay_group("b", &update, Some(1), true).unwrap();
        let update = addition(&s, "Local");
        s.relay_view(
            "a",
            &update,
            Some(2),
            true,
            Some(HistoryViewRequest {
                frontier: stale.frontier,
                selection: Some(Selection::Node {
                    target: typstographe_core::content::commands::TreeTarget {
                        root: typstographe_core::content::commands::TreeRoot::Body,
                        path: vec![999],
                    },
                }),
                typing_styles: vec![],
            }),
        )
        .unwrap();
        assert!(!s.ensure_owner().unwrap().snapshot("a").unwrap().pending);
        s.history("a", Direction::Undo).unwrap();
        let restored = s.ensure_owner().unwrap().snapshot("a").unwrap();
        assert_eq!(restored.typing_styles, vec![Style::Bold]);
        assert!(!restored.selection_lost);
    }
    #[test]
    fn native_owner_confirms_typing_once_and_keeps_histories_separate() {
        let temp = tempfile::tempdir().unwrap();
        let mut session = Session::fresh(temp.path().into());
        seed(&mut session);
        assert!(session.connection("a", "old-document", false).is_err());
        assert!(session.offline.is_empty());
        assert_eq!(session.receipt("a", vec![]).unwrap().history.undo_steps, 0);
        for value in ["A", "B"] {
            let update = addition(&session, value);
            session.relay_group("a", &update, Some(1), true).unwrap();
        }
        assert_eq!(session.receipt("a", vec![]).unwrap().history.undo_steps, 1);
        assert_eq!(session.receipt("b", vec![]).unwrap().history.undo_steps, 0);
        let remote = addition(&session, "C");
        session.relay_group("b", &remote, Some(1), true).unwrap();
        session.history("a", Direction::Undo).unwrap();
        let doc = SharedDocument::from_updates(&session.updates)
            .unwrap()
            .state()
            .unwrap();
        assert_eq!(
            doc.document.content[0].content[0].text.as_deref(),
            Some("CBase")
        );
        let revision = session.revision;
        assert_eq!(
            session
                .relay_group("a", &session.updates[0].clone(), Some(2), true)
                .unwrap()
                .revision,
            revision
        );
        assert!(session.check_peer("a", "old-document").is_err());
        assert!(session.check_peer("stranger", &session.generation).is_err());
        let before = session.updates.clone();
        assert!(session.relay_group("a", &[1, 2, 3], Some(3), true).is_err());
        assert_eq!(session.updates, before);
        assert!(session.receipt("a", vec![]).unwrap().history.pending);
        assert!(!session.receipt("b", vec![]).unwrap().history.pending);
        session.ensure_owner().unwrap().reset("a").unwrap();
        assert!(!session.receipt("a", vec![]).unwrap().history.pending);
    }
    #[test]
    fn transport_compaction_preserves_durable_history_and_untracked_repairs() {
        let temp = tempfile::tempdir().unwrap();
        let mut session = Session::fresh(temp.path().into());
        seed(&mut session);
        for _ in 0..514 {
            let update = addition(&session, "a");
            session.relay_group("a", &update, Some(1), true).unwrap();
        }
        assert!(session.updates.len() < 512);
        assert_eq!(session.receipt("a", vec![]).unwrap().history.undo_steps, 1);
        let update = addition(&session, "repair");
        session.relay_group("a", &update, Some(1), false).unwrap();
        assert_eq!(session.receipt("a", vec![]).unwrap().history.undo_steps, 1);
        session.history("a", Direction::Undo).unwrap();
        let state = SharedDocument::from_updates(&session.updates)
            .unwrap()
            .state()
            .unwrap();
        assert_eq!(
            state.document.content[0].content[0].text.as_deref(),
            Some("repairBase")
        );
    }
}
