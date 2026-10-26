//! Transport de recette STDIO, uniquement pour le serveur de test du navigateur.
//! Les chemins restent ceux de la session fixée au lancement.
use crate::*;
use std::io::{BufRead, Write};

pub fn run(bridge: Bridge) {
    let root = bridge
        .session
        .lock()
        .expect("Session de recette indisponible")
        .root
        .clone();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                eprintln!("{error}");
                continue;
            }
        };
        let result = dispatch(&bridge, &root, &request);
        let response = match result {
            Ok(value) => serde_json::json!({"id":request["id"],"result":value}),
            Err(error) => serde_json::json!({"id":request["id"],"error":error}),
        };
        println!("{response}");
        let _ = std::io::stdout().flush();
    }
}
fn dispatch(
    bridge: &Bridge,
    root: &std::path::Path,
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let args = &request["args"];
    let peer = request["peer"].as_str().ok_or("Participant absent.")?;
    let command = request["command"].as_str().ok_or("Commande absente.")?;
    match command {
        "sync_forward" | "sync_backward" | "sync_selection" => {
            let revision = args["revision"].as_u64().ok_or("Révision absente.")?;
            let session = bridge.session.lock().map_err(|e| e.to_string())?;
            if session.revision != revision {
                return Err("stale".into());
            }
            let cache = bridge.renderer.1.lock().map_err(|e| e.to_string())?;
            let sync = cache
                .as_ref()
                .filter(|sync| sync.revision == revision)
                .ok_or("stale")?;
            match command {
                "sync_forward" => Ok(serde_json::json!(
                    sync.forward(args["position"].as_u64().ok_or("Position absente.")? as usize)
                )),
                "sync_backward" => Ok(serde_json::json!(sync.backward(
                    serde_json::from_value(args["location"].clone()).map_err(|e| e.to_string())?
                ))),
                _ => Ok(serde_json::json!(sync.selection_highlights_exact(
                    args["from"].as_u64().ok_or("Début absent.")? as usize
                        ..args["to"].as_u64().ok_or("Fin absente.")? as usize
                ))),
            }
        }
        "connection" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            if args["online"].as_bool() == Some(true) {
                session.offline.remove(peer);
            } else {
                session.offline.insert(peer.into());
            }
            Ok(serde_json::Value::Null)
        }
        "content_catalogue" => Ok(catalogue::content_catalogue()),
        "count_text" => Ok(proofing::count_text(
            args["text"].as_str().unwrap_or("").into(),
            args["selection"].as_str().map(str::to_owned),
        )),
        "spelling_check" => tauri::async_runtime::block_on(proofing::spelling_check(
            serde_json::from_value(args["segments"].clone()).map_err(|e| e.to_string())?,
            serde_json::from_value(args["language"].clone()).map_err(|e| e.to_string())?,
            serde_json::from_value(args["ignored"].clone()).map_err(|e| e.to_string())?,
        )),
        "spelling_suggestions" => tauri::async_runtime::block_on(proofing::spelling_suggestions(
            serde_json::from_value(args["language"].clone()).map_err(|e| e.to_string())?,
            args["word"].as_str().unwrap_or("").into(),
        ))
        .map(|words| serde_json::json!(words)),
        "preferences_load" => serde_json::to_value(preferences::Preferences::load(
            &bridge.session.lock().map_err(|e| e.to_string())?.root,
        )?)
        .map_err(|e| e.to_string()),
        "preferences_save" => {
            let preferences: preferences::Preferences =
                serde_json::from_value(args["preferences"].clone()).map_err(|e| e.to_string())?;
            preferences.save(&bridge.session.lock().map_err(|e| e.to_string())?.root)?;
            Ok(serde_json::Value::Null)
        }
        "reset" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            // Each case is a sibling: never nest it under the preceding case.
            let root = root.join(format!(
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
            let map = if command == "preview" {
                Some(
                    typstographe_core::content::source_mapped(&snapshot.document, &session.assets)?
                        .1,
                )
            } else {
                None
            };
            let preview = bridge.renderer.render(
                source,
                session.assets.clone(),
                snapshot.revision,
                export.clone(),
                Some(snapshot.settings.as_ref().unwrap_or(&session.settings)),
                map,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resetting_cases_keeps_paths_bounded_and_preferences_writable() {
        let dir = tempfile::tempdir().unwrap();
        let bridge = Bridge {
            session: Arc::new(Mutex::new(Session::open(dir.path().into()).unwrap())),
            renderer: Arc::new(Renderer::default()),
            diagnostic: None,
            agent: Arc::new(Mutex::new(agent::AgentState::default())),
        };
        let request = serde_json::json!({"command":"reset", "peer":"main", "args":{}});
        for _ in 0..50 {
            dispatch(&bridge, dir.path(), &request).unwrap();
            let session = bridge.session.lock().unwrap();
            assert_eq!(session.root.parent(), Some(dir.path()));
            preferences::Preferences::default()
                .save(&session.root)
                .unwrap();
        }
    }
}
