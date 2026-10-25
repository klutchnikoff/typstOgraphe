//! Local attachment. IO threads exchange requests; only the UI thread borrows Engine.
mod endpoint;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
mod transport;

use crate::mcp::proposals::Proposals;
use crate::mcp::{Host, Operation, Request};
use std::{
    io,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use typstographe_core::engine::{EditContext, Engine, TargetGuard};

/// Explicit read-only invitation for one in-memory document, consumed by one client.
/// Dropping it revokes the channel and removes its private invitation file.
pub struct LocalSession {
    invitation: endpoint::Invitation,
    receiver: mpsc::Receiver<Request>,
    shutdown: CancellationToken,
    status: Arc<AtomicU8>,
    guard: TargetGuard,
    id: String,
    context: EditContext,
    proposals: Proposals,
}
impl LocalSession {
    pub fn start(engine: &mut Engine, wake: Arc<dyn Fn() + Send + Sync>) -> io::Result<Self> {
        if !engine.is_open() {
            return Err(io::Error::other("No document open"));
        }
        let invitation = endpoint::Invitation::new()?;
        let (sender, receiver) = mpsc::channel(16);
        let shutdown = CancellationToken::new();
        let status = Arc::new(AtomicU8::new(0));
        let (ready, started) = std::sync::mpsc::sync_channel(1);
        let endpoint = invitation.data.clone();
        let stop = shutdown.clone();
        let state = status.clone();
        std::thread::Builder::new()
            .name("mcp-attachment".into())
            .spawn(move || {
                let result = transport::listen(
                    endpoint,
                    sender,
                    stop.clone(),
                    state.clone(),
                    wake.clone(),
                    ready.clone(),
                );
                state.store(2, Ordering::Release);
                stop.cancel();
                wake();
                if let Err(error) = result {
                    let _ = ready.send(Err(error));
                }
            })?;
        started.recv().map_err(io::Error::other)??;
        // A context has no source, revision, history or focus effects. Its lifetime
        // is tied to this invitation, never to an individual incoming request.
        let context = engine
            .create_edit_context(engine.target_guard(), None)
            .map_err(io::Error::other)?;
        Ok(Self {
            invitation,
            receiver,
            shutdown,
            status,
            guard: engine.target_guard(),
            id: uuid::Uuid::new_v4().to_string(),
            context,
            proposals: Default::default(),
        })
    }
    pub fn invitation_path(&self) -> &Path {
        self.invitation.path()
    }
    pub fn connected(&self) -> bool {
        self.status.load(Ordering::Acquire) == 1
    }
    pub fn finished(&self) -> bool {
        self.status.load(Ordering::Acquire) == 2 || self.shutdown.is_cancelled()
    }
    /// Call after native text publication. Reads observe a single published revision.
    /// Returns false after disconnect/revoke or if the document was closed/replaced.
    pub fn poll(&mut self, engine: &mut Engine) -> bool {
        self.poll_with_input(engine, false)
    }
    pub fn review(&self) -> Option<&crate::ProposalReview> {
        self.proposals.review()
    }
    /// Native diagnostic/front-end preparation uses the same portable submission
    /// path as MCP. This never approves or publishes a candidate.
    pub fn prepare_proposal(
        &mut self,
        engine: &Engine,
        request_id: &str,
        steps: Vec<typstographe_core::engine::Call>,
    ) -> Result<serde_json::Value, typstographe_core::engine::CommandError> {
        if self.finished() {
            return Err(crate::mcp::proposals::error("Invitation ended"));
        }
        let params = serde_json::json!({"document":{"id":self.id,"guard":engine.target_guard()},"request_id":request_id,"steps":steps});
        self.proposals.dispatch(
            engine,
            &self.context,
            &self.id,
            "proposals.submit",
            params.as_object().expect("object").clone(),
            false,
        )
    }
    pub fn proposal_status(&self, id: &str) -> Option<crate::ProposalStatus> {
        self.proposals.status(id)
    }
    /// Trusted human-facing decision; never routed from incoming MCP operations.
    pub fn decide(
        &mut self,
        engine: &mut Engine,
        id: &str,
        accept: bool,
        busy: bool,
    ) -> Result<Option<typstographe_core::engine::Outcome>, typstographe_core::engine::CommandError>
    {
        if self.finished() || engine.target_guard().document_id != self.guard.document_id {
            return Err(crate::mcp::proposals::error("Invitation ended"));
        }
        self.proposals
            .decide(engine, &self.context, id, accept, busy)
    }
    pub fn poll_with_input(&mut self, engine: &mut Engine, busy: bool) -> bool {
        if self.finished()
            || !engine.is_open()
            || engine.target_guard().document_id != self.guard.document_id
        {
            self.shutdown.cancel();
            return false;
        }
        self.proposals.refresh(engine, &self.context);
        // Bounded work per frame; the transport also has bounded backpressure.
        for _ in 0..16 {
            let Ok(request) = self.receiver.try_recv() else {
                break;
            };
            if request.reply.is_closed() || self.shutdown.is_cancelled() {
                continue;
            }
            let result = Host::attached_proposals(
                engine,
                self.id.clone(),
                self.context.clone(),
                &mut self.proposals,
                busy,
            )
            .and_then(|mut host| match request.operation {
                Operation::Tool { name, arguments } => host.dispatch(&name, arguments),
                Operation::Resource { uri } => host.resource(&uri),
            });
            let _ = request.reply.send(result);
        }
        true
    }
}
impl Drop for LocalSession {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

/// Start the ordinary MCP stdio bridge using an invitation created by the application.
/// No document is opened and no autonomous Engine is constructed by the bridge.
pub fn bridge(path: &Path) -> io::Result<()> {
    let endpoint = endpoint::read(path)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(transport::bridge(endpoint));
    // Tokio stdin uses an uncancellable blocking read. When the application
    // revokes the socket, do not wait for the MCP client to close stdin too.
    // This runtime belongs exclusively to the short-lived CLI bridge process.
    runtime.shutdown_background();
    result
}
