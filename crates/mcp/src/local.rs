//! Fixed MCP bridge to a native app invitation. No project is opened by the bridge.
use rmcp::{ErrorData, RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    sync::Arc,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invitation {
    address: String,
    token: String,
}
#[derive(Clone)]
struct Client {
    invitation: Arc<Invitation>,
}
impl Client {
    async fn request(&self, method: String, params: Value) -> Result<Value, ErrorData> {
        let invitation = self.invitation.clone();
        tokio::task::spawn_blocking(move || -> Result<Value, std::io::Error> {
            let address: SocketAddr = invitation.address.parse().map_err(std::io::Error::other)?;
            if !address.ip().is_loopback() || invitation.token.len() < 16 {
                return Err(std::io::Error::other("Invalid local invitation"));
            }
            let mut stream =
                TcpStream::connect_timeout(&address, std::time::Duration::from_secs(5))?;
            stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))?;
            stream.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;
            writeln!(
                stream,
                "{}",
                json!({"token":invitation.token,"method":method,"params":params})
            )?;
            let mut line = String::new();
            BufReader::new(stream.take(32 * 1024 * 1024 + 1)).read_line(&mut line)?;
            if line.len() > 32 * 1024 * 1024 {
                return Err(std::io::Error::other("Reply too large"));
            }
            serde_json::from_str(&line).map_err(std::io::Error::other)
        })
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }
}
impl ServerHandler for Client {
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
        .with_instructions(crate::instructions(true))
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: crate::tools(true),
            ..Default::default()
        })
    }
    fn get_tool(&self, name: &str) -> Option<Tool> {
        crate::tools(true).into_iter().find(|t| t.name == name)
    }
    async fn call_tool(
        &self,
        r: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if self.get_tool(&r.name).is_none() {
            return Err(ErrorData::invalid_params("Unknown tool", None));
        }
        let reply = self
            .request(r.name.into_owned(), json!(r.arguments.unwrap_or_default()))
            .await?;
        Ok(if let Some(error) = reply.get("error") {
            CallToolResult::structured_error(json!({"version":1,"ok":false,"error":error}))
        } else {
            CallToolResult::structured(reply["result"].clone())
        }
        .into())
    }
    async fn list_resources(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult {
            resources: crate::mcp::resources_list(),
            ..Default::default()
        })
    }
    async fn read_resource(
        &self,
        r: ReadResourceRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        let reply = self
            .request("resource.read".into(), json!({"uri":r.uri}))
            .await?;
        if let Some(e) = reply.get("error") {
            return Err(ErrorData::internal_error(
                "Cannot read resource",
                Some(e.clone()),
            ));
        }
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(
                serde_json::to_string_pretty(&reply["result"]).unwrap(),
                r.uri,
            )
            .with_mime_type("application/json"),
        ])
        .into())
    }
}
pub fn bridge(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 8192 {
        return Err("Invalid invitation file".into());
    }
    let invitation: Invitation = serde_json::from_slice(&std::fs::read(path)?)?;
    let address: SocketAddr = invitation.address.parse()?;
    if !address.ip().is_loopback() || invitation.token.len() < 16 {
        return Err("Invalid local invitation".into());
    }
    let client = Client {
        invitation: Arc::new(invitation),
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let server = client.serve(rmcp::transport::stdio()).await?;
            server.waiting().await?;
            Ok(())
        })
}
pub fn active_path() -> Result<PathBuf, std::io::Error> {
    #[cfg(not(target_os = "windows"))]
    let home =
        home::home_dir().ok_or_else(|| std::io::Error::other("Home directory unavailable"))?;
    #[cfg(target_os = "macos")]
    let base = home.join("Library/Application Support");
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("APPDATA unavailable"))?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    Ok(base.join("org.typstographe.web-editor-spike/invitation.json"))
}
pub fn bridge_active() -> Result<(), Box<dyn std::error::Error>> {
    bridge(&active_path()?)
}
