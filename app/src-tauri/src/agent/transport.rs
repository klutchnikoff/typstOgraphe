//! Bounded authenticated native MCP connection, shared with loopback integration tests.
use super::{AgentState, authorize, dispatch, validate_request};
use crate::session::Session;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    sync::Mutex,
};

pub(super) fn connection(
    mut stream: TcpStream,
    session: &Mutex<Session>,
    agent: &Mutex<AgentState>,
    notify: impl FnOnce(),
) {
    // macOS peut transmettre O_NONBLOCK depuis le listener au socket accepté.
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(30)));
    let mut line = String::new();
    if BufReader::new((&stream).take(5 * 1024 * 1024 + 1))
        .read_line(&mut line)
        .is_err()
        || line.len() > 5 * 1024 * 1024
    {
        return;
    }
    let response = (|| {
        let request: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        authorize(
            &*agent.lock().map_err(|e| e.to_string())?,
            request["token"].as_str(),
        )?;
        // Schema compilation can be cold/expensive; do it before taking the live document lock.
        validate_request(request["method"].as_str().unwrap_or(""), &request["params"])?;
        let mut s = session.lock().map_err(|e| e.to_string())?;
        let mut a = agent.lock().map_err(|e| e.to_string())?;
        authorize(&a, request["token"].as_str())?; // Revocation may have happened meanwhile.
        let result = dispatch(
            &mut s,
            &mut a,
            request["method"].as_str().unwrap_or(""),
            &request["params"],
        );
        if request["method"] == "proposals.submit" {
            notify();
        }
        result
    })();
    let value = match response {
        Ok(value) => json!({"result":value}),
        Err(error) => {
            let value = serde_json::from_str::<Value>(&error)
                .unwrap_or_else(|_| json!({"code":"unavailable","message":error}));
            json!({"error":value})
        }
    };
    let _ = writeln!(stream, "{value}");
}
