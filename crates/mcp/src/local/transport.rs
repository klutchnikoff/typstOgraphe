//! A token handshake precedes the unchanged MCP newline-delimited byte stream.
use super::endpoint::Endpoint;
use crate::mcp::{Request, Server};
use interprocess::local_socket::tokio::{Stream, prelude::*};
use rmcp::ServiceExt;
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;

pub(super) fn listen(
    endpoint: Endpoint,
    sender: mpsc::Sender<Request>,
    shutdown: CancellationToken,
    status: Arc<AtomicU8>,
    wake: Arc<dyn Fn() + Send + Sync>,
    ready: std::sync::mpsc::SyncSender<io::Result<()>>,
) -> io::Result<()> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let listener = endpoint.listener()?;
        ready.send(Ok(())).map_err(io::Error::other)?;
        loop {
            let mut stream = tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                result = listener.accept() => result?,
            };
            let accepted = tokio::select! {
                _ = shutdown.cancelled() => return Ok(()),
                result = tokio::time::timeout(Duration::from_secs(3), authenticate(&mut stream, &endpoint.token)) => matches!(result, Ok(Ok(()))),
            };
            if !accepted { continue; }
            if status.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_err() {
                let _ = stream.write_all(b"busy\n").await;
                continue;
            }
            stream.write_all(b"ok\n").await?;
            wake();
            let server = Server::attached(sender.clone(), wake.clone());
            let stop = shutdown.clone();
            let state = status.clone();
            let repaint = wake.clone();
            tokio::spawn(async move {
                // Hold the state through initialization too: a peer may never initialize MCP.
                let _lease = Lease { status: state, wake: repaint };
                tokio::select! {
                    _ = stop.cancelled() => {},
                    _ = async {
                        if let Ok(service) = server.serve(stream).await { let _ = service.waiting().await; }
                    } => {},
                }
            });
        }
    })
}
struct Lease {
    status: Arc<AtomicU8>,
    wake: Arc<dyn Fn() + Send + Sync>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.status.store(2, Ordering::Release);
        (self.wake)();
    }
}
async fn authenticate(stream: &mut Stream, token: &str) -> io::Result<()> {
    let mut bytes = [0; 65];
    stream.read_exact(&mut bytes).await?;
    // Fixed-size handshake; invalid peers never enter rmcp or the UI queue.
    if bytes[64] != b'\n' || &bytes[..64] != token.as_bytes() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Invalid attachment token",
        ));
    }
    Ok(())
}
pub(super) async fn connect(endpoint: &Endpoint) -> io::Result<Stream> {
    let mut stream = Stream::connect(endpoint.name()?).await?;
    stream.write_all(endpoint.token.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    let mut response = [0; 3];
    stream.read_exact(&mut response).await?;
    if &response != b"ok\n" {
        return Err(io::Error::other("Invitation already consumed"));
    }
    Ok(stream)
}
pub(super) async fn bridge(endpoint: Endpoint) -> io::Result<()> {
    let stream = tokio::time::timeout(Duration::from_secs(5), connect(&endpoint))
        .await
        .map_err(io::Error::other)??;
    let (mut read, mut write) = tokio::io::split(stream);
    let mut input = tokio::io::stdin();
    let mut output = tokio::io::stdout();
    // EOF ends this attachment. Never reopen, reconnect, or replay a request.
    tokio::select! {
        result = tokio::io::copy(&mut input, &mut write) => { result?; },
        result = tokio::io::copy(&mut read, &mut output) => { result?; },
    }
    Ok(())
}
