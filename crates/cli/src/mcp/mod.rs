//! Transport MCP uniquement en aval du cœur. Le worker possède sa session.
mod catalogue;
mod host;
mod policy;
mod resources;
mod roots;
mod types;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};
use typstographe_core::engine::CommandError;

const INSTRUCTIONS: &str = "Discover document_types.list, then document_types.describe and capabilities.describe. Models are interchangeable visual presentations of a type. Start with document.new or document.open; pass the outer document reference from the latest response to every documentary call, including replacement. Put literal prose in text.insert and use formatting tools; do not write Typst markup. Each response includes server permissions: read/write roots are independent, paths are relative to the server launch directory, parent components are refused and symlinks are resolved. No roots means no document file access. Writable open needs both read and write authorization and reserves a cooperative writer lock; if locked, open read_only=true to read the last saved snapshot, then save_as an authorized separate copy to edit. A globally read-only server allows only queries, read-only opening and closing. Edits remain in memory; saving is explicit, export is separate and never marks the document saved. document.batch edits atomically and can be undone once, with an optional final save; failure publishes no edits. Step guards are filled by the server. Steps may use params.selector on the evolving draft; file operations are forbidden inside steps. Reads, validation, spelling and inline PNG rendering never correct or save text. Refresh document.inspect after a stale-reference error. No recovery, network or subprocess tools; no GUI attachment in phase 5. No skill or repository access is required.";

struct Request {
    name: String,
    arguments: Map<String, Value>,
    reply: oneshot::Sender<Result<types::Reply, CommandError>>,
}
#[derive(Clone)]
struct Server {
    sender: mpsc::Sender<Request>,
}
impl Server {
    fn new(policy: policy::Policy) -> Result<Self, std::io::Error> {
        let (sender, mut receiver) = mpsc::channel::<Request>(16);
        std::thread::Builder::new()
            .name("mcp-document".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut host = host::Host::with_policy(policy);
                while let Some(request) = receiver.blocking_recv() {
                    // Un appel annulé avant son exécution n’a aucun effet.
                    if !request.reply.is_closed() {
                        let result = host.dispatch(&request.name, request.arguments);
                        let _ = request.reply.send(result);
                    }
                }
            })?;
        Ok(Self { sender })
    }
    async fn dispatch(
        &self,
        name: String,
        arguments: Map<String, Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Request {
                name,
                arguments,
                reply,
            })
            .await
            .map_err(|_| internal())?;
        let result = response.await.map_err(|_| internal())?;
        Ok(match result {
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
            Err(error) => CallToolResult::structured_error(
                json!({"version":1,"ok":false,"error":{"code":error.code,"params":error.params,"message":typstographe_i18n::command_error(&error)}}),
            ),
        })
    }
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
        .with_instructions(INSTRUCTIONS)
    }
    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        if request.is_some_and(|r| r.cursor.is_some()) {
            return Err(ErrorData::invalid_params(
                "No pagination cursor is issued by this prototype",
                None,
            ));
        }
        Ok(ListToolsResult {
            tools: catalogue::tools().to_vec(),
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        catalogue::tools().iter().find(|t| t.name == name).cloned()
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if self.get_tool(&request.name).is_none() {
            return Err(ErrorData::invalid_params(
                "Unknown or unavailable tool; discover tools/list",
                None,
            ));
        }
        Ok(self
            .dispatch(
                request.name.into_owned(),
                request.arguments.unwrap_or_default(),
            )
            .await?
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
        request: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let value = resources::read(&request.uri)
            .ok_or_else(|| ErrorData::invalid_params("Unknown typstographe resource", None))?;
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(value, request.uri).with_mime_type("application/json"),
        ])
        .into())
    }
}
fn internal() -> ErrorData {
    ErrorData::internal_error("Document worker stopped", None)
}

pub fn serve(
    read_only: bool,
    read_roots: Vec<std::path::PathBuf>,
    write_roots: Vec<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let policy = policy::Policy::new(read_roots, write_roots, read_only)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let server = Server::new(policy)?.serve(rmcp::transport::stdio()).await?;
        server.waiting().await?;
        Ok(())
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod file_tests;
