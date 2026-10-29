//! One confirmed Rust owner and one private history per authenticated window.
use std::collections::HashMap;
use typstographe_core::{
    assets::Assets,
    content::{
        commands::Command,
        contract::{Error, Guard},
        editing::{ContextId, ContextSnapshot, Direction, EditingSession},
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
    ) -> Result<Publication, String> {
        let id = self.context(peer)?;
        let guard = self.engine.guard().clone();
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
