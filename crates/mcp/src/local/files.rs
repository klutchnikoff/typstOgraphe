//! Deferred export: compile privately, then revalidate at the UI publication point.
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "file_tests.rs"]
mod tests;
use super::*;
use crate::mcp::{file_grant::FileGrant, types::Reply};
use serde_json::{Map, Value};
use std::{path::PathBuf, sync::mpsc as worker};
use tokio::sync::oneshot;
use typstographe_core::engine::{CommandError, CompiledExport, ErrorCode, Outcome};

pub enum FileEvent {
    Saved(Outcome),
    Exported(Value),
}
enum Phase {
    Compiling(worker::Receiver<Result<CompiledExport, CommandError>>),
    Writing(worker::Receiver<Result<Value, CommandError>>),
}
pub(super) struct PendingExport {
    phase: Phase,
    reply: oneshot::Sender<Result<Reply, CommandError>>,
    response: Reply,
    guard: TargetGuard,
    directory: PathBuf,
    cancel: CancellationToken,
}
fn failure(error: impl std::fmt::Display) -> CommandError {
    CommandError::detail(ErrorCode::Io, "mcp_file_access", error)
}
fn background<T: Send + 'static>(
    name: &str,
    wake: Arc<dyn Fn() + Send + Sync>,
    work: impl FnOnce() -> Result<T, CommandError> + Send + 'static,
) -> Result<worker::Receiver<Result<T, CommandError>>, CommandError> {
    let (send, receive) = worker::channel();
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let _ = send.send(work());
            wake();
        })
        .map_err(failure)?;
    Ok(receive)
}
impl LocalSession {
    pub fn file_permissions(&self, engine: &Engine) -> Value {
        self.files.describe(engine)
    }
    pub fn take_file_events(&mut self) -> Vec<FileEvent> {
        std::mem::take(&mut self.file_events)
    }
    pub(super) fn begin_file_export(
        &mut self,
        engine: &mut Engine,
        arguments: Map<String, Value>,
        reply: oneshot::Sender<Result<Reply, CommandError>>,
        cancel: CancellationToken,
        busy: bool,
    ) {
        let result = (|| {
            if self.export.is_some() {
                return Err(CommandError::new(
                    ErrorCode::Unavailable,
                    "mcp_attached_busy",
                ));
            }
            let host = Host::attached_proposals(
                engine,
                self.id.clone(),
                self.context.clone(),
                &mut self.proposals,
                busy,
            )?
            .with_files(self.files.clone());
            let (task, response, directory) = host.begin_export(arguments)?;
            let guard = engine.target_guard();
            let phase = Phase::Compiling(background(
                "mcp-export-compile",
                self.wake.clone(),
                move || task.compile(),
            )?);
            Ok((phase, response, guard, directory))
        })();
        match result {
            Ok((phase, response, guard, directory)) => {
                self.export = Some(PendingExport {
                    phase,
                    reply,
                    response,
                    guard,
                    directory,
                    cancel,
                })
            }
            Err(error) => {
                let _ = reply.send(Err(error));
            }
        }
    }
    pub(super) fn poll_file_export(&mut self, engine: &Engine, busy: bool) {
        let Some(mut job) = self.export.take() else {
            return;
        };
        if self.finished() {
            return;
        }
        if job.reply.is_closed() || job.cancel.is_cancelled() {
            // Typst compilation cannot be killed safely. Keep the occupied slot
            // until its worker ends so repeated cancellations cannot spawn unbounded work.
            let waiting = match &job.phase {
                Phase::Compiling(receive) => {
                    matches!(receive.try_recv(), Err(worker::TryRecvError::Empty))
                }
                Phase::Writing(receive) => {
                    matches!(receive.try_recv(), Err(worker::TryRecvError::Empty))
                }
            };
            if waiting {
                self.export = Some(job);
            }
            return;
        }
        match &job.phase {
            Phase::Compiling(receive) => match receive.try_recv() {
                Err(worker::TryRecvError::Empty) => self.export = Some(job),
                result => {
                    let compiled = result.map_err(failure).and_then(|r| r);
                    let validated = compiled.and_then(|compiled| {
                        if engine.target_guard() != job.guard {
                            return Err(CommandError::new(ErrorCode::Document, "stale_revision"));
                        }
                        if busy {
                            return Err(CommandError::new(
                                ErrorCode::Unavailable,
                                "mcp_attached_busy",
                            ));
                        }
                        if self.files.export_path(&job.directory)? != job.directory {
                            return Err(failure("Export destination changed"));
                        }
                        Ok(compiled)
                    });
                    let next = validated.and_then(|compiled| {
                        let files: FileGrant = self.files.clone();
                        let directory = job.directory.clone();
                        let stop = self.shutdown.clone();
                        let cancel = job.cancel.clone();
                        background("mcp-export-write", self.wake.clone(), move || {
                            if stop.is_cancelled() || cancel.is_cancelled() {
                                return Err(failure("Invitation ended before export write"));
                            }
                            if files.export_path(&directory)? != directory {
                                return Err(failure("Export destination changed"));
                            }
                            compiled.write()
                        })
                    });
                    match next {
                        Ok(receive) => {
                            job.phase = Phase::Writing(receive);
                            self.export = Some(job);
                        }
                        Err(error) => {
                            let _ = job.reply.send(Err(error));
                        }
                    }
                }
            },
            Phase::Writing(receive) => match receive.try_recv() {
                Err(worker::TryRecvError::Empty) => self.export = Some(job),
                result => {
                    let result = result.map_err(failure).and_then(|r| r).map(|value| {
                        self.file_events.push(FileEvent::Exported(value.clone()));
                        job.response.value["outcome"]["value"] = value;
                        // The outer reference remains a fresh reference to the published
                        // document; outcome.guard/value.guard identify the exported snapshot.
                        job.response.value["document"]["guard"] =
                            serde_json::json!(engine.target_guard());
                        let permissions = self.files.describe(engine);
                        job.response.value["permissions"]["write_enabled"] = serde_json::json!(
                            !permissions["current_file"].is_null()
                                || permissions["export_roots"]
                                    .as_array()
                                    .is_some_and(|roots| !roots.is_empty())
                        );
                        job.response.value["permissions"]["attached_files"] = permissions;
                        job.response
                    });
                    let _ = job.reply.send(result);
                }
            },
        }
    }
}
