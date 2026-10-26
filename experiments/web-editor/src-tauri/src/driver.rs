//! Transport de recette STDIO, uniquement pour le serveur de test du navigateur.
//! Les chemins restent ceux de la session fixée au lancement.
use crate::*;
use std::io::{BufRead, Write};

pub fn run(bridge: Bridge) {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("{error}");
                continue;
            }
        };
        let result = dispatch(&bridge, &request);
        let response = match result {
            Ok(value) => serde_json::json!({"id":request["id"],"result":value}),
            Err(error) => serde_json::json!({"id":request["id"],"error":error}),
        };
        println!("{response}");
        let _ = std::io::stdout().flush();
    }
}
fn dispatch(bridge: &Bridge, request: &serde_json::Value) -> Result<serde_json::Value, String> {
    let args = &request["args"];
    let peer = request["peer"].as_str().ok_or("Participant absent.")?;
    let command = request["command"].as_str().ok_or("Commande absente.")?;
    match command {
        "reset" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            let root = session.root.join(format!(
                "case-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            *session = Session::open(root)?;
            Ok(serde_json::Value::Null)
        }
        "join" => {
            serde_json::to_value(bridge.session.lock().map_err(|e| e.to_string())?.join(peer))
                .map_err(|e| e.to_string())
        }
        "relay" => {
            let update: Vec<u8> =
                serde_json::from_value(args["update"].clone()).map_err(|e| e.to_string())?;
            let revision = bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .relay(peer, &update)?;
            Ok(serde_json::json!(revision))
        }
        "awareness" => Ok(serde_json::Value::Null),
        "prepare_rust_edit" => Ok(prepare_rust_edit()),
        "import_image" => {
            let data: Vec<u8> =
                serde_json::from_value(args["data"].clone()).map_err(|e| e.to_string())?;
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            serde_json::to_value(session.import(
                args["extension"].as_str().ok_or("Extension absente.")?,
                &data,
            )?)
            .map_err(|e| e.to_string())
        }
        "preview" | "save" | "export_document" => {
            let snapshot: Snapshot =
                serde_json::from_value(args["snapshot"].clone()).map_err(|e| e.to_string())?;
            let session = bridge.session.lock().map_err(|e| e.to_string())?;
            let source = session.validate(&snapshot)?;
            let export = (command == "export_document").then(|| {
                session.root.join("exports").join(format!(
                    "browser-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ))
            });
            let preview = bridge.renderer.render(
                source,
                session.assets.clone(),
                snapshot.revision,
                export.clone(),
            )?;
            match command {
                "preview" => serde_json::to_value(preview).map_err(|e| e.to_string()),
                "save" => {
                    let update = serde_json::from_value(args["update"].clone())
                        .map_err(|e| e.to_string())?;
                    Ok(serde_json::json!(
                        session.save(&snapshot, update)?.display().to_string()
                    ))
                }
                _ => Ok(serde_json::json!(export.unwrap().display().to_string())),
            }
        }
        _ => Err("Commande de diagnostic inconnue.".into()),
    }
}
