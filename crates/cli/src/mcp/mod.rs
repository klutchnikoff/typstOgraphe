//! Transport MCP uniquement en aval du cœur. Le worker possède sa session.
mod catalogue;
mod host;
mod policy;
mod resources;
mod types;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};
use typstographe_core::engine::CommandError;

const INSTRUCTIONS: &str = "Discover document_types.list, then document_types.describe and capabilities.describe. Models are interchangeable visual presentations of a type. Start with document.new; pass the outer document reference from the latest response to every documentary call. Put literal prose in text.insert and use formatting tools; do not write Typst markup. Reads, validation and inline PNG rendering never correct or save text. This prototype keeps one document in memory: no opening, saving, export, recovery, model imports or path-based images/signatures. Embedded images are allowed. document.batch edits atomically and can be undone once; batch step guards are filled by the server. A step may use params.selector to resolve exact text or a block on the evolving draft. Refresh with document.inspect after a stale-reference error. Do not assume a type capability is authorized by this prototype. No skill or repository access is required.";

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
    fn new(read_only: bool) -> Result<Self, std::io::Error> {
        let (sender, mut receiver) = mpsc::channel::<Request>(16);
        std::thread::Builder::new()
            .name("mcp-document".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                let mut host = host::Host::new(read_only);
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

pub fn serve(read_only: bool) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let server = Server::new(read_only)?
            .serve(rmcp::transport::stdio())
            .await?;
        server.waiting().await?;
        Ok(())
    })
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;
