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
        "diagnostic_request_close" => Ok(serde_json::Value::Null),
        "work_list" => serde_json::to_value(
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .work_list()?,
        )
        .map_err(|e| e.to_string()),
        "work_delete" => {
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .work_delete(args["id"].as_str().ok_or("id missing")?)?;
            Ok(serde_json::Value::Null)
        }
        "work_restore" => {
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .work_restore(
                    peer,
                    args["id"].as_str().ok_or("id missing")?,
                    args["revision"].as_u64().ok_or("revision missing")?,
                )?;
            Ok(serde_json::Value::Null)
        }
        "work_abandon" => {
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .abandon_window_work(
                    peer,
                    args["generation"].as_str().ok_or("generation missing")?,
                )?;
            Ok(serde_json::Value::Null)
        }
        "work_import" => {
            let record = crate::recovery::work::read_work(std::path::Path::new(
                args["path"].as_str().ok_or("path missing")?,
            ))?;
            let mut s = bridge.session.lock().map_err(|e| e.to_string())?;
            s.lifecycle(args["revision"].as_u64().ok_or("revision missing")?)?;
            s.restore_work_record(peer, record)?;
            Ok(serde_json::Value::Null)
        }
        "transition_begin" => serde_json::to_value(
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .begin_transition(
                    peer,
                    args["generation"].as_str().ok_or("generation missing")?,
                )?,
        )
        .map_err(|e| e.to_string()),
        "transition_ack" => {
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .acknowledge_transition(
                    peer,
                    serde_json::from_value(args["ack"].clone()).map_err(|e| e.to_string())?,
                )?;
            Ok(serde_json::Value::Null)
        }
        "transition_status" => serde_json::to_value(
            bridge
                .session
                .lock()
                .map_err(|e| e.to_string())?
                .transition_status(args["id"].as_str().ok_or("id missing")?)?,
        )
        .map_err(|e| e.to_string()),
        "transition_end" => {
            let mut s = bridge.session.lock().map_err(|e| e.to_string())?;
            if s.transition
                .as_ref()
                .is_some_and(|t| Some(t.id.as_str()) == args["id"].as_str())
            {
                s.transition = None;
            }
            Ok(serde_json::Value::Null)
        }
        "close_document" => {
            crate::files::close_for_recipe(
                &mut *bridge.session.lock().map_err(|e| e.to_string())?,
                args["revision"].as_u64().ok_or("revision missing")?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "open_private_recovery" => {
            let generation = args["generation"].as_str().ok_or("generation missing")?;
            Ok(serde_json::json!(
                bridge
                    .session
                    .lock()
                    .map_err(|e| e.to_string())?
                    .open_private_recovery(peer, generation)?
            ))
        }
        "conserve_private" => {
            let input = serde_json::from_value(args["input"].clone()).map_err(|e| e.to_string())?;
            Ok(serde_json::json!(
                bridge
                    .session
                    .lock()
                    .map_err(|e| e.to_string())?
                    .conserve_private(peer, input)?
            ))
        }
        "diagnostic_private_recovery" => {
            let s = bridge.session.lock().map_err(|e| e.to_string())?;
            let path = crate::recovery::drafts::path(&s.root, &s.generation, peer)?;
            if path.exists() {
                serde_json::to_value(crate::recovery::drafts::read(&path)?)
                    .map_err(|e| e.to_string())
            } else {
                Ok(serde_json::Value::Null)
            }
        }

        "diagnostic_agent" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            let mut agent = bridge.agent.lock().map_err(|e| e.to_string())?;
            agent::dispatch(
                &mut session,
                &mut agent,
                args["method"].as_str().ok_or("Method absent")?,
                &args["params"],
            )
        }
        "diagnostic_agent_accept" => {
            let publication = agent::accept(
                &bridge.session,
                &bridge.agent,
                args["id"].as_str().ok_or("ID absent")?,
                peer,
                |candidate, assets| {
                    bridge.renderer.validate(
                        &crate::schema::source(&candidate.after, assets)?,
                        assets,
                        bridge
                            .session
                            .lock()
                            .map_err(|e| e.to_string())?
                            .render_context(&candidate.settings),
                    )
                },
            )?;
            serde_json::to_value(publication).map_err(|e| e.to_string())
        }
        "diagnostic_shared_state" => shared_diagnostic::run(
            args.clone(),
            &bridge.session.lock().map_err(|e| e.to_string())?.assets,
        ),
        "bibliography_index" | "bibliography_search" => {
            let files = serde_json::from_value(args["files"].clone()).map_err(|e| e.to_string())?;
            serde_json::to_value(if command == "bibliography_index" {
                bibliography::bibliography_index(files)?
            } else {
                bibliography::bibliography_search(
                    files,
                    args["query"].as_str().unwrap_or("").into(),
                )?
            })
            .map_err(|e| e.to_string())
        }
        "validate_bibliography" => {
            bibliography::validate_bibliography(
                serde_json::from_value(args["document"].clone()).map_err(|e| e.to_string())?,
                serde_json::from_value(args["metadata"].clone()).map_err(|e| e.to_string())?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "model_policy" => serde_json::to_value(bibliography::policy(
            &*bridge.session.lock().map_err(|e| e.to_string())?,
            args["template"].as_str().unwrap_or("").into(),
        )?)
        .map_err(|e| e.to_string()),
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
        "connection" | "set_synchronization" | "leave" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            let generation = args["generation"].as_str().unwrap_or("");
            if command == "connection" {
                session.connection(peer, generation, args["online"].as_bool() == Some(true))?;
            } else if command == "leave" {
                session.check_peer(peer, generation)?;
                session.leave(peer);
            } else {
                session.set_synchronization(
                    peer,
                    generation,
                    args["enabled"].as_bool() == Some(true),
                )?;
            }
            serde_json::to_value(session.synchronization()).map_err(|e| e.to_string())
        }
        "content_catalogue" => Ok(catalogue::content_catalogue()),
        "validate_model" => {
            let snapshot: Snapshot =
                serde_json::from_value(args["snapshot"].clone()).map_err(|e| e.to_string())?;
            let session = bridge.session.lock().map_err(|e| e.to_string())?;
            let source = session.validate(&snapshot)?;
            bridge.renderer.validate(
                &source,
                &session.assets,
                session.render_context(snapshot.settings.as_ref().unwrap_or(&session.settings)),
            )?;
            Ok(serde_json::Value::Null)
        }
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
        "new_document" => {
            let settings =
                serde_json::from_value(args["settings"].clone()).map_err(|e| e.to_string())?;
            let revision = args["revision"].as_u64().ok_or("Révision absente.")?;
            files::create(
                &mut *bridge.session.lock().map_err(|e| e.to_string())?,
                settings,
                revision,
                serde_json::from_value(
                    args.get("resources")
                        .cloned()
                        .unwrap_or(serde_json::json!([])),
                )
                .map_err(|e| e.to_string())?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "join" => {
            serde_json::to_value(bridge.session.lock().map_err(|e| e.to_string())?.join(peer))
                .map_err(|e| e.to_string())
        }
        "commit_structured_commands" => {
            let request =
                serde_json::from_value(args["request"].clone()).map_err(|e| e.to_string())?;
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            let publication = structured::commit(&mut session, peer, request)?;
            let mut value = serde_json::to_value(publication).map_err(|e| e.to_string())?;
            value["broadcast"] = serde_json::json!(session.should_broadcast(peer));
            Ok(value)
        }
        "prepare_structured_commands" => {
            let request =
                serde_json::from_value(args["request"].clone()).map_err(|e| e.to_string())?;
            serde_json::to_value(structured::prepare(
                &*bridge.session.lock().map_err(|e| e.to_string())?,
                request,
            )?)
            .map_err(|e| e.to_string())
        }
        "relay" => {
            let update: Vec<u8> =
                serde_json::from_value(args["update"].clone()).map_err(|e| e.to_string())?;
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            let receipt = session.relay_view(
                peer,
                &update,
                args["group"].as_u64(),
                args["capture"].as_bool().unwrap_or(true),
                serde_json::from_value(args["historyView"].clone()).map_err(|e| e.to_string())?,
            )?;
            let mut value = serde_json::to_value(receipt).map_err(|e| e.to_string())?;
            value["broadcast"] = serde_json::json!(session.should_broadcast(peer));
            Ok(value)
        }
        "editing_history" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            let direction =
                serde_json::from_value(args["direction"].clone()).map_err(|e| e.to_string())?;
            let mut value = serde_json::to_value(session.history(peer, direction)?)
                .map_err(|e| e.to_string())?;
            value["broadcast"] = serde_json::json!(session.should_broadcast(peer));
            Ok(value)
        }

        "editing_context" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            serde_json::to_value(session.ensure_owner()?.snapshot(peer)?).map_err(|e| e.to_string())
        }
        "set_editing_context" => {
            let request: owner::ContextRequest =
                serde_json::from_value(args["request"].clone()).map_err(|e| e.to_string())?;
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            session.check_peer(peer, &request.generation)?;
            serde_json::to_value(session.ensure_owner()?.set_context(peer, request)?)
                .map_err(|e| e.to_string())
        }
        "reset_editing_context" => {
            let mut session = bridge.session.lock().map_err(|e| e.to_string())?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            serde_json::to_value(session.ensure_owner()?.reset(peer)?).map_err(|e| e.to_string())
        }
        "awareness" => {
            let update = args["update"].as_array().ok_or("Présence absente")?;
            if update.len() > 64 * 1024 {
                return Err("Présence trop volumineuse.".into());
            }
            let session = bridge.session.lock().map_err(|e| e.to_string())?;
            Ok(serde_json::json!(session.accepts_awareness(
                peer,
                args["generation"].as_str().unwrap_or(""),
                args["version"].as_u64().unwrap_or(0)
            )?))
        }
        "prepare_rust_edit" => Ok(prepare_rust_edit()),
        "prepare_image" => serde_json::to_value(prepare_image(
            args["extension"]
                .as_str()
                .ok_or("Missing extension")?
                .into(),
            args["data"].as_str().ok_or("Missing image data")?.into(),
        )?)
        .map_err(|e| e.to_string()),
        "preview_frame" => {
            use base64::Engine as _;
            let snapshot: Snapshot =
                serde_json::from_value(args["snapshot"].clone()).map_err(|e| e.to_string())?;
            let encoding =
                serde_json::from_value(args["encoding"].clone()).map_err(|e| e.to_string())?;
            let session = bridge.session.lock().map_err(|e| e.to_string())?;
            let source = session.validate(&snapshot)?;
            let map =
                typstographe_core::content::source_mapped(&snapshot.document, &session.assets)?.1;
            let bytes = bridge.renderer.frame(
                source,
                session.assets.clone(),
                snapshot.revision,
                session.render_context(snapshot.settings.as_ref().unwrap_or(&session.settings)),
                map,
                encoding,
            )?;
            // STDIO is only the browser functional test driver, never the native benchmark.
            Ok(
                serde_json::json!({"binary": base64::engine::general_purpose::STANDARD.encode(bytes)}),
            )
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
                Some(
                    session.render_context(snapshot.settings.as_ref().unwrap_or(&session.settings)),
                ),
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
#[cfg_attr(coverage_nightly, coverage(off))]
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
