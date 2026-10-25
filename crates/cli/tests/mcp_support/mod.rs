#![cfg_attr(coverage_nightly, coverage(off))]
use rmcp::{
    RoleClient, ServiceExt, model::*, service::RunningService, transport::TokioChildProcess,
};
use serde_json::Value;
use std::{path::Path, process::Stdio};

pub type Client = RunningService<RoleClient, ClientConfig>;
pub fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
pub async fn start(path: &Path, version: ProtocolVersion, args: &[&str]) -> Client {
    start_with_env(path, version, args, &[]).await
}
pub async fn start_with_env(
    path: &Path,
    version: ProtocolVersion,
    args: &[&str],
    environment: &[(&str, &Path)],
) -> Client {
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_typstographe-cli"));
    command.envs(environment.iter().copied());
    command
        .arg("mcp")
        .args(args)
        .current_dir(path)
        .stderr(Stdio::from(
            std::fs::File::create(path.join("stderr.log")).unwrap(),
        ));
    ClientConfig::default()
        .with_protocol_version(version)
        .serve(TokioChildProcess::new(command).unwrap())
        .await
        .unwrap()
}
pub async fn result(client: &Client, name: &str, args: Value, success: bool) -> CallToolResult {
    let result = client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    assert_eq!(
        result.is_error.unwrap_or(false),
        !success,
        "{name}: {result:?}"
    );
    result
}
pub async fn call(client: &Client, name: &str, args: Value) -> Value {
    result(client, name, args, true)
        .await
        .structured_content
        .unwrap()
}
