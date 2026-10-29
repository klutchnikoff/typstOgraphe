//! Replace an invitation only after its durable local descriptor is written.
use super::{AgentState, agent_peer};
use crate::session::Session;
use std::{io::Write, path::Path, sync::Mutex};
use typstographe_mcp::local::Invitation;
pub(super) fn install(
    session: &Mutex<Session>,
    agent: &Mutex<AgentState>,
    root: &Path,
    invitation: &Invitation,
) -> Result<(), String> {
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let options = atomic_write_file::AtomicWriteFile::options();
    #[cfg(unix)]
    let options = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = options;
        options.mode(0o600);
        options
    };
    let mut out = options
        .open(root.join("invitation.json"))
        .map_err(|e| e.to_string())?;
    out.write_all(&serde_json::to_vec(invitation).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    out.commit().map_err(|e| e.to_string())?;
    let mut session = session.lock().map_err(|e| e.to_string())?;
    let mut agent = agent.lock().map_err(|e| e.to_string())?;
    if let Some(owner) = &mut session.owner {
        owner.close(&agent_peer(&agent))?;
    }
    agent.token = Some(invitation.token.clone());
    agent.proposal = None;
    agent.seen.clear();
    Ok(())
}
