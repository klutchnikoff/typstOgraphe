//! Transport MCP en aval du cœur : worker autonome ou demandes au propriétaire UI.
mod catalogue;
mod documentation;
mod engine_access;
pub(crate) mod file_grant;
mod host;
mod policy;
pub(crate) mod proposals;
mod resources;
mod roots;
pub(crate) mod types;
pub(crate) use host::Host;
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde_json::{Map, Value, json};
use tokio::sync::{mpsc, oneshot};
use typstographe_core::engine::CommandError;

const INSTRUCTIONS: &str = "Discover document_types.list, then document_types.describe and capabilities.describe. Models are interchangeable visual presentations of a type. Start with document.new or document.open; pass the outer document reference from the latest response to every documentary call, including replacement. Put literal prose in text.insert and use formatting tools; *bold* and _italic_ are literal text, not inline markup. Bold/italic persist: at an empty caret use on, insert text, then off for each style you want to stop. Off on a nonempty selection removes its formatting. New paragraphs/structures do not guarantee a reset. capabilities.describe exposes batch_examples for undoable on/insert/off composition; do not write Typst markup. Each response includes server permissions: read/write roots are independent, paths are relative to the server launch directory, parent components are refused and symlinks are resolved. No roots means no document file access. Writable open needs both read and write authorization and reserves a cooperative writer lock; if locked, open read_only=true to read the last saved snapshot, then save_as an authorized separate copy to edit. A globally read-only server allows only queries, read-only opening and closing. Edits remain in memory; saving is explicit, export is separate and never marks the document saved. document.batch edits atomically and can be undone once, with an optional final save; failure publishes no edits. Step guards are filled by the server. Steps may use params.selector on the evolving draft; file operations are forbidden inside steps. Reads, validation, spelling and inline PNG rendering never correct or save text. Refresh document.inspect after a stale-reference error. No recovery, network or subprocess tools; no GUI attachment in phase 5. Resources typstographe://guide, types, commands, document and models provide current serialized snapshots; read them again after edits, as subscriptions are not advertised. Capability examples contain params and bindings: replace listed null references with actual current results before calling. Effective MCP schemas are authoritative. No skill or repository access is required.";

pub(crate) fn instructions(attached: bool) -> String {
    if !attached {
        return INSTRUCTIONS.into();
    }
    "This connection is attached to the document open in typstOgraphe. Start with document.inspect. Reads see published content only. Submit editing steps with proposals.submit: the isolated lot is pending, NOT published, until the human explicitly accepts the entire lot in the application. No tool can approve. Use a unique request_id per lot; an identical retry returns the same status. One pending lot at a time. Return control to the human after submission, do not poll in a loop. proposals.get/cancel use the proposal ID without a current document revision. Any user edit makes the lot stale: read again and submit a new lot, never merge or retry edits automatically. Direct editing, open/new/close/save_as are refused. document.save may save only the explicitly authorized current file; document.export may write only below the explicitly authorized export root into a new or empty directory. Read permissions.attached_files in document.inspect; no permission is implied. These operations use published content, never a pending proposal. Export compiles in the background and revalidates the revision and permission before writing; stale or busy requests fail without delayed retry. Once writing starts, subsequent user edits do not change that exported snapshot; refresh document.inspect after export. Discover capabilities.describe, document_types.describe and commands.list for params/selectors and proposal support. Bold/italic persist within a lot: on, insert, off; use literal text, not Typst markup. No arbitrary read roots, local imports or preferences are accessible. File permissions belong to this invitation and do not follow Save As to another file. Supply the latest outer document reference to documentary calls. Closing/replacing the document, disconnection or revocation terminates this invitation and drops pending candidates. No skill or repository access is needed.".into()
}

pub(crate) enum Operation {
    Tool {
        name: String,
        arguments: Map<String, Value>,
    },
    Resource {
        uri: String,
    },
}
pub(crate) struct Request {
    pub(crate) operation: Operation,
    pub(crate) reply: oneshot::Sender<Result<types::Reply, CommandError>>,
    pub(crate) cancel: tokio_util::sync::CancellationToken,
}
#[derive(Clone)]
pub(crate) struct Server {
    sender: mpsc::Sender<Request>,
    attached: bool,
    wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
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
                    if !request.reply.is_closed() && !request.cancel.is_cancelled() {
                        let result = match request.operation {
                            Operation::Tool { name, arguments } => host.dispatch(&name, arguments),
                            Operation::Resource { uri } => host.resource(&uri),
                        };
                        let _ = request.reply.send(result);
                    }
                }
            })?;
        Ok(Self {
            sender,
            attached: false,
            wake: None,
        })
    }
    pub(crate) fn attached(
        sender: mpsc::Sender<Request>,
        wake: std::sync::Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            sender,
            attached: true,
            wake: Some(wake),
        }
    }
    async fn dispatch(
        &self,
        name: String,
        arguments: Map<String, Value>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<CallToolResult, ErrorData> {
        let result = self
            .request(Operation::Tool { name, arguments }, cancel)
            .await?;
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
        if let Some(wake) = &self.wake {
            wake();
        }
        response.await.map_err(|_| internal())
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
        .with_instructions(instructions(self.attached))
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
            tools: catalogue::for_session(self.attached).to_vec(),
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        catalogue::for_session(self.attached)
            .iter()
            .find(|t| t.name == name)
            .cloned()
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if self.get_tool(&request.name).is_none() {
            return Err(ErrorData::invalid_params(
                "Unknown or unavailable tool; discover tools/list",
                None,
            ));
        }
        let pending = self.dispatch(
            request.name.into_owned(),
            request.arguments.unwrap_or_default(),
            context.ct.clone(),
        );
        tokio::select! {
            biased;
            _ = context.ct.cancelled() => Err(ErrorData::internal_error("Request cancelled", None)),
            result = pending => Ok(result?.into()),
        }
    }
    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        if request.is_some_and(|r| r.cursor.is_some()) {
            return Err(ErrorData::invalid_params(
                "No resource pagination cursor is issued",
                None,
            ));
        }
        Ok(ListResourcesResult {
            resources: resources::list(),
            ..Default::default()
        })
    }
    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        if !resources::known(&request.uri) {
            return Err(ErrorData::resource_not_found(
                "Unknown typstographe resource",
                Some(json!({"uri":request.uri})),
            ));
        }
        let pending = self.request(
            Operation::Resource {
                uri: request.uri.clone(),
            },
            context.ct.clone(),
        );
        let reply = tokio::select! {
            biased;
            _ = context.ct.cancelled() => return Err(ErrorData::internal_error("Request cancelled", None)),
            result = pending => result?,
        }
            .map_err(|error| {
                ErrorData::internal_error(
                    typstographe_i18n::command_error(&error),
                    Some(json!({"code":error.code,"params":error.params})),
                )
            })?;
        let value = serde_json::to_string_pretty(&reply.value).map_err(|_| internal())?;
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod documentation_tests;
