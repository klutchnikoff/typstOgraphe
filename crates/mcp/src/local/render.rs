//! Bounded background render reads; no access to the live engine on worker threads.
#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
#[path = "render_tests.rs"]
mod tests;
use super::*;
use crate::mcp::types::Reply;
use serde_json::{Map, Value};
use std::sync::mpsc as worker;
use tokio::sync::oneshot;
use typstographe_core::engine::{CommandError, CompiledRender, ErrorCode};

enum Phase {
    Compiling(worker::Receiver<Result<CompiledRender, CommandError>>),
    Rasterizing(worker::Receiver<Result<Vec<u8>, CommandError>>, Reply),
}
pub(super) struct PendingRead {
    phase: Phase,
    name: String,
    arguments: Map<String, Value>,
    guard: TargetGuard,
    reply: oneshot::Sender<Result<Reply, CommandError>>,
    cancel: CancellationToken,
}
pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "render.compile" | "render.page" | "render.locate" | "render.selection_boxes"
    )
}
fn unavailable() -> CommandError {
    CommandError::new(ErrorCode::Unavailable, "mcp_attached_busy")
}
impl LocalSession {
    pub(super) fn begin_render_read(
        &mut self,
        engine: &mut Engine,
        name: String,
        mut arguments: Map<String, Value>,
        reply: oneshot::Sender<Result<Reply, CommandError>>,
        cancel: CancellationToken,
    ) {
        let prepared = (|| {
            let host = Host::attached(engine, self.id.clone(), self.context.clone())?;
            let task = host.prepare_render_read(&name, &mut arguments)?;
            if self.render_read.is_some() && (task.is_some() || name == "render.page") {
                return Err(unavailable());
            }
            Ok(task)
        })();
        match prepared {
            Err(error) => {
                let _ = reply.send(Err(error));
            }
            Ok(Some(task)) => {
                match files::background("mcp-render-compile", self.wake.clone(), move || {
                    Ok(task.compile())
                }) {
                    Ok(receive) => {
                        self.render_read = Some(PendingRead {
                            phase: Phase::Compiling(receive),
                            name,
                            arguments,
                            guard: engine.target_guard(),
                            reply,
                            cancel,
                        })
                    }
                    Err(error) => {
                        let _ = reply.send(Err(error));
                    }
                }
            }
            Ok(None) => self.finish_render_read(engine, name, arguments, reply, cancel),
        }
    }
    fn finish_render_read(
        &mut self,
        engine: &mut Engine,
        name: String,
        arguments: Map<String, Value>,
        reply: oneshot::Sender<Result<Reply, CommandError>>,
        cancel: CancellationToken,
    ) {
        if name != "render.page" {
            let result = Host::attached_proposals(
                engine,
                self.id.clone(),
                self.context.clone(),
                &mut self.proposals,
                false,
            )
            .map(|host| host.with_files(self.files.clone()))
            .and_then(|mut host| host.dispatch(&name, arguments));
            let _ = reply.send(result);
            return;
        }
        let result = (|| {
            let host = Host::attached_proposals(
                engine,
                self.id.clone(),
                self.context.clone(),
                &mut self.proposals,
                false,
            )?
            .with_files(self.files.clone());
            let (task, response) = host.begin_page(arguments.clone())?;
            let receive =
                files::background("mcp-render-page", self.wake.clone(), move || task.execute())?;
            Ok((receive, response))
        })();
        match result {
            Ok((receive, response)) => {
                self.render_read = Some(PendingRead {
                    phase: Phase::Rasterizing(receive, response),
                    name,
                    arguments,
                    guard: engine.target_guard(),
                    reply,
                    cancel,
                })
            }
            Err(error) => {
                let _ = reply.send(Err(error));
            }
        }
    }
    pub(super) fn poll_render_read(&mut self, engine: &mut Engine) {
        let Some(job) = self.render_read.take() else {
            return;
        };
        if self.finished() || job.reply.is_closed() || job.cancel.is_cancelled() {
            // Typst cannot be forcibly stopped. Retain its slot until it finishes,
            // otherwise repeated cancellations could start unbounded work.
            let waiting = match &job.phase {
                Phase::Compiling(r) => matches!(r.try_recv(), Err(worker::TryRecvError::Empty)),
                Phase::Rasterizing(r, _) => {
                    matches!(r.try_recv(), Err(worker::TryRecvError::Empty))
                }
            };
            if waiting && !self.finished() {
                self.render_read = Some(job);
            }
            return;
        }
        self.poll_ready_render(engine, job);
    }
    fn poll_ready_render(&mut self, engine: &mut Engine, mut job: PendingRead) {
        let current = || {
            if engine.target_guard() == job.guard {
                Ok(())
            } else {
                Err(CommandError::new(ErrorCode::Document, "stale_revision"))
            }
        };
        let stopped = || {
            CommandError::detail(
                ErrorCode::Document,
                "render_compile",
                "Render worker stopped",
            )
        };
        match &mut job.phase {
            Phase::Compiling(receive) => {
                let result = match receive.try_recv() {
                    Err(worker::TryRecvError::Empty) => {
                        self.render_read = Some(job);
                        return;
                    }
                    Err(_) => Err(stopped()),
                    Ok(result) => result,
                };
                let result = current()
                    .and(result)
                    .and_then(|compiled| engine.complete_render(compiled));
                if job.name == "render.compile" {
                    let response = result.and_then(|compiled| {
                        let host = Host::attached_proposals(
                            engine,
                            self.id.clone(),
                            self.context.clone(),
                            &mut self.proposals,
                            false,
                        )?
                        .with_files(self.files.clone());
                        Ok(host.compiled_reply(compiled))
                    });
                    let _ = job.reply.send(response);
                } else {
                    match result.and_then(crate::calls::render_from_result) {
                        Ok(_) => self.finish_render_read(
                            engine,
                            job.name,
                            job.arguments,
                            job.reply,
                            job.cancel,
                        ),
                        Err(error) => {
                            let _ = job.reply.send(Err(error));
                        }
                    }
                }
            }
            Phase::Rasterizing(receive, response) => {
                let result = match receive.try_recv() {
                    Err(worker::TryRecvError::Empty) => {
                        self.render_read = Some(job);
                        return;
                    }
                    Err(_) => Err(stopped()),
                    Ok(result) => result,
                };
                let result = current().and(result).and_then(|png| {
                    let host = Host::attached_proposals(
                        engine,
                        self.id.clone(),
                        self.context.clone(),
                        &mut self.proposals,
                        false,
                    )?
                    .with_files(self.files.clone());
                    let response = Reply {
                        value: std::mem::take(&mut response.value),
                        png: Some(png),
                    };
                    Ok(host.refresh_read_reply(response))
                });
                let _ = job.reply.send(result);
            }
        }
    }
}
