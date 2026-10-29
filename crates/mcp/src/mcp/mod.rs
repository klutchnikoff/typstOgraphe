//! A bounded serialized worker for the autonomous structured contract.
pub mod catalogue;
mod host;
mod resources;
#[cfg(test)]
mod root_tests;
pub(crate) mod roots;
#[cfg(test)]
mod tests;
pub mod types;
pub mod validation;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};
use typstographe_core::{content::api, engine::CommandError};
pub fn instructions(attached: bool) -> String {
    let common = "Discover document_types.list and commands.list, then document_types.describe and capabilities.describe. All operations use the same structured Rust contract as the editor. Document targets are zone paths in the current guard; text offsets are UTF-16, inline objects count as one. Never reuse a target after edits without reading again. Commands and queries expose actual parameter schemas. Use literal prose, never Typst markup: text.insert carries explicit marks (bold/italic/link); text.bold/italic format a nonempty typed range. For private future typing use context.set/context.style, context.insert, then switch styles off. Documentary batches contain only typed edits (not context operations); one final validation, one undo entry. Selector conveniences resolve exact text on the evolving draft, without regex or fuzzy search. Header collections use stable record IDs. Models are interchangeable visual presentations; metadata/options remain subject to the type/model contract. Read resources for the guide, types, commands, document and models. No skill or repository access is required.";
    format!(
        "{common} {}",
        if attached {
            "This invitation sees only published content in the application. Start with document.inspect. Edits require proposals.submit and whole-lot human acceptance; no tool can approve. Attached proposals currently edit body content only; metadata/resources/files remain read-only. One pending lot, unique idempotent request_id. Return control to the human after submission. Concurrent human edits invalidate a proposal; never auto-retry or merge it. Context navigation is private and cannot move human focus. IME/dialogues suspend proposals. Revocation or document replacement ends the invitation."
        } else {
            "This is an autonomous session. File access requires explicit independent read/write roots; paths are relative to the launch directory, parent components are refused and symlinks resolved. Writable opening locks before reading. Read-only opening can inspect the last saved copy under another writer's lock. Editing remains in memory until an explicit save; export is separate and never marks it saved. A batch may save only as its final action, and failure publishes/saves nothing. Pass the latest outer document reference to calls. Refresh document.inspect after a stale-reference error. No tools execute programs, access networks, modify preferences or recover arbitrary files."
        }
    )
}
struct Request {
    operation: Operation,
    reply: oneshot::Sender<Result<types::Reply, CommandError>>,
    cancel: tokio_util::sync::CancellationToken,
}
enum Operation {
    Tool(String, Map<String, Value>),
    Resource(String),
}
#[derive(Clone)]
struct Server {
    sender: mpsc::Sender<Request>,
}
impl Server {
    fn new(
        read: Vec<std::path::PathBuf>,
        write: Vec<std::path::PathBuf>,
        read_only: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut host = host::Host::new(read, write, read_only)?;
        let (sender, mut receiver) = mpsc::channel::<Request>(16);
        std::thread::Builder::new()
            .name("mcp-document".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                while let Some(r) = receiver.blocking_recv() {
                    if !r.reply.is_closed() && !r.cancel.is_cancelled() {
                        let value = match r.operation {
                            Operation::Tool(name, p) => host.dispatch(&name, p),
                            Operation::Resource(uri) => host.resource(&uri),
                        };
                        let _ = r.reply.send(value);
                    }
                }
            })?;
        Ok(Self { sender })
    }
    async fn request(
        &self,
        operation: Operation,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<Result<types::Reply, CommandError>, ErrorData> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request {
                operation,
                reply,
                cancel,
            })
            .await
            .map_err(|_| internal())?;
        response.await.map_err(|_| internal())
    }
}
fn internal() -> ErrorData {
    ErrorData::internal_error("Document worker stopped", None)
}
impl ServerHandler for Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new(
            "typstographe",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(instructions(false))
    }
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        if request.is_some_and(|r| r.cursor.is_some()) {
            return Err(ErrorData::invalid_params("No pagination cursor", None));
        }
        Ok(ListToolsResult {
            tools: catalogue::tools(false),
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        catalogue::tools(false).into_iter().find(|t| t.name == name)
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if self.get_tool(&request.name).is_none() {
            return Err(ErrorData::invalid_params(
                "Unknown tool; discover tools/list",
                None,
            ));
        }
        let pending = self.request(
            Operation::Tool(
                request.name.into_owned(),
                request.arguments.unwrap_or_default(),
            ),
            context.ct.clone(),
        );
        let reply = tokio::select! {biased; _=context.ct.cancelled()=>return Err(ErrorData::internal_error("Cancelled",None)), r=pending=>r?};
        Ok(match reply {
            Ok(reply) => {
                let mut result = CallToolResult::structured(reply.value);
                if let Some(png) = reply.png {
                    use base64::Engine;
                    result.content.push(ContentBlock::image(
                        base64::prelude::BASE64_STANDARD.encode(png),
                        "image/png",
                    ));
                }
                result
            }
            Err(error) => {
                let mut value = api::error_value(&error);
                value["message"] = json!(typstographe_i18n::command_error(&error));
                CallToolResult::structured_error(json!({"version":1,"ok":false,"error":value}))
            }
        }
        .into())
    }
    async fn list_resources(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: resources::list(),
            ..Default::default()
        })
    }
    async fn read_resource(
        &self,
        r: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        if !resources::known(&r.uri) {
            return Err(ErrorData::resource_not_found("Unknown resource", None));
        }
        let reply = self
            .request(Operation::Resource(r.uri.clone()), context.ct.clone())
            .await?
            .map_err(|e| {
                ErrorData::internal_error(
                    typstographe_i18n::command_error(&e),
                    Some(api::error_value(&e)),
                )
            })?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(serde_json::to_string_pretty(&reply.value).unwrap(), r.uri)
                .with_mime_type("application/json"),
        ])
        .into())
    }
}
pub fn serve(
    read_only: bool,
    read_roots: Vec<std::path::PathBuf>,
    write_roots: Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new(read_roots, write_roots, read_only)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let running = server.serve(rmcp::transport::stdio()).await?;
            running.waiting().await?;
            Ok(())
        })
}
pub(crate) fn resources_list() -> Vec<rmcp::model::Resource> {
    resources::list()
}
