//! Local attachment. IO threads exchange requests; only the UI thread borrows Engine.
mod endpoint;
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
mod transport;

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
use typstographe_core::engine::{Engine, TargetGuard};

/// Explicit read-only invitation for one in-memory document, consumed by one client.
/// Dropping it revokes the channel and removes its private invitation file.
pub struct LocalSession {
    invitation: endpoint::Invitation,
    receiver: mpsc::Receiver<Request>,
    shutdown: CancellationToken,
    status: Arc<AtomicU8>,
    guard: TargetGuard,
    id: String,
}
impl LocalSession {
    pub fn start(engine: &Engine, wake: Arc<dyn Fn() + Send + Sync>) -> io::Result<Self> {
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
        Ok(Self {
            invitation,
            receiver,
            shutdown,
            status,
            guard: engine.target_guard(),
            id: uuid::Uuid::new_v4().to_string(),
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
        if self.finished()
            || !engine.is_open()
            || engine.target_guard().document_id != self.guard.document_id
        {
            self.shutdown.cancel();
            return false;
        }
        // Bounded work per frame; the transport also has bounded backpressure.
        for _ in 0..16 {
            let Ok(request) = self.receiver.try_recv() else {
                break;
            };
            if request.reply.is_closed() || self.shutdown.is_cancelled() {
                continue;
            }
            let result = Host::attached(engine, self.id.clone()).and_then(|mut host| match request
                .operation
            {
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
