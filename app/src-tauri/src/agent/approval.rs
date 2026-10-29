//! Whole-lot human acceptance. Compilation never holds the live document lock.
use super::{AgentState, Proposal, commit_accepted};
use crate::session::Session;
use std::sync::Mutex;
use typstographe_core::{assets::Assets, content::shared::Publication};

pub(crate) fn accept(
    session: &Mutex<Session>,
    agent: &Mutex<AgentState>,
    id: &str,
    peer: &str,
    validate: impl FnOnce(&Proposal, &Assets) -> Result<(), String>,
) -> Result<Publication, String> {
    accept_with_notification(session, agent, id, peer, validate, |_| Ok(()))
}

pub(super) fn accept_with_notification(
    session: &Mutex<Session>,
    agent: &Mutex<AgentState>,
    id: &str,
    peer: &str,
    validate: impl FnOnce(&Proposal, &Assets) -> Result<(), String>,
    publish: impl FnOnce(&Publication) -> Result<(), String>,
) -> Result<Publication, String> {
    let (candidate, assets) = {
        let session = session.lock().map_err(|e| e.to_string())?;
        let agent = agent.lock().map_err(|e| e.to_string())?;
        session.check_peer(peer, &session.generation)?;
        let candidate = agent
            .proposal
            .as_ref()
            .filter(|p| {
                p.id == id
                    && p.status == "pending"
                    && p.revision == session.revision
                    && p.generation == session.generation
            })
            .ok_or("Proposition périmée.")?;
        if !agent.busy.is_empty() {
            return Err("Terminez la composition IME avant de décider.".into());
        }
        (candidate.clone(), session.assets.clone())
    };
    validate(&candidate, &assets)?;
    let mut session = session.lock().map_err(|e| e.to_string())?;
    let mut agent = agent.lock().map_err(|e| e.to_string())?;
    if !agent.busy.is_empty() {
        return Err("Composition IME en cours.".into());
    }
    let proposal = agent
        .proposal
        .as_mut()
        .filter(|p| p.id == id)
        .ok_or("Proposition absente.")?;
    // A competing acceptance/refusal is final; do not relabel it as stale.
    if proposal.status != "pending" {
        return Err("Proposition déjà traitée.".into());
    }
    if proposal.revision != session.revision || proposal.generation != session.generation {
        proposal.status = "stale".into();
        return Err("Proposition périmée : relisez le document.".into());
    }
    let publication = commit_accepted(&mut session, &mut agent, id, peer)?;
    // Keep publication and its notification ordered against concurrent edits.
    publish(&publication)?;
    Ok(publication)
}
