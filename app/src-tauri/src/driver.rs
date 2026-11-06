//! Transport de recette STDIO, uniquement pour le serveur de test du navigateur.
//! Les chemins restent ceux de la session fixée au lancement.
use crate::error::CommandError;
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
) -> Result<serde_json::Value, CommandError> {
    let args = &request["args"];
    let peer = request["peer"].as_str().ok_or("Participant absent.")?;
    let command = request["command"].as_str().ok_or("Commande absente.")?;
    match command {
        "choose_file" if args["save"] == true => {
            let folder = bridge.session()?.root.clone();
            std::fs::create_dir_all(&folder)?;
            Ok(serde_json::json!(
                folder.join(format!("document-{}.typsto", uuid::Uuid::new_v4()))
            ))
        }

        "comparison_open" | "comparison_read" | "comparison_close" | "comparison_apply"
        | "comparison_preview" | "comparison_prepare" => {
            let mut s = bridge.session()?;
            let mut a = bridge.agent.lock()?;
            let id = args["id"].as_str().unwrap_or("");
            match command {
                "comparison_open" => crate::comparisons::open(
                    &mut s,
                    &mut a,
                    peer,
                    &serde_json::from_value(args["guard"].clone())?,
                    serde_json::from_value(args["origin"].clone())?,
                ),
                "comparison_prepare" => {
                    let choices: Vec<typstographe_core::content::comparison::SourceSide> =
                        serde_json::from_value(args["choices"].clone())?;
                    let ready =
                        crate::comparisons::staging::capture(&mut s, &a, peer, id, &choices)?
                            .prepare()?;
                    let archive = ready.archive();
                    bridge.renderer.validate(
                        &archive.validate().map_err(crate::project::error)?,
                        &archive.assets,
                        crate::rendering::RenderContext {
                            settings: &archive.state().settings,
                            dependencies: archive.dependencies.clone(),
                        },
                    )?;
                    crate::comparisons::staging::finish(&mut s, &a, peer, ready)
                }
                "comparison_read" => crate::comparisons::read(&s, &a, peer, id),
                "comparison_close" => {
                    crate::comparisons::close(&mut s, peer, id)?;
                    Ok(serde_json::Value::Null)
                }
                "comparison_preview" => {
                    crate::comparisons::read(&s, &a, peer, id)?;
                    let r = &s.comparisons[id];
                    let archive = if args["side"] == "before" {
                        r.plan.before().clone()
                    } else {
                        r.plan.after().clone()
                    };
                    drop(s);
                    drop(a);
                    serde_json::to_value(crate::versions::preview(archive)?)
                        .map_err(CommandError::from)
                }
                _ => {
                    let choices: Vec<typstographe_core::content::comparison::SourceSide> =
                        serde_json::from_value(args["choices"].clone())?;
                    let ready = crate::comparisons::prepare(&mut s, &a, peer, id, &choices)?;
                    let archive = ready.archive();
                    bridge.renderer.validate(
                        &archive.validate().map_err(crate::project::error)?,
                        &archive.assets,
                        crate::rendering::RenderContext {
                            settings: &archive.state().settings,
                            dependencies: archive.dependencies.clone(),
                        },
                    )?;
                    let (mut result, publication) =
                        crate::comparisons::apply(&mut s, &mut a, peer, id, ready)?;
                    result["publication"] = serde_json::to_value(publication)?;
                    Ok(result)
                }
            }
        }

        "versions_list" | "versions_read" | "versions_preview" | "versions_change" => {
            let guard = serde_json::from_value(args["guard"].clone())?;
            let mut session = bridge.session()?;
            match command {
                "versions_list" => crate::versions::list(&mut session, peer, &guard),
                "versions_read" => crate::versions::read(
                    &mut session,
                    peer,
                    &guard,
                    serde_json::from_value(args["id"].clone())?,
                ),
                "versions_change" => crate::versions::change(
                    &mut session,
                    peer,
                    &guard,
                    serde_json::from_value(args["change"].clone())?,
                    args["destination"].as_str().map(std::path::Path::new),
                ),
                _ => {
                    let archive = crate::versions::selected(
                        &mut session,
                        peer,
                        &guard,
                        serde_json::from_value(args["id"].clone())?,
                    )?;
                    drop(session);
                    serde_json::to_value(crate::versions::preview(archive)?)
                        .map_err(CommandError::from)
                }
            }
        }

        "diagnostic_request_close" => Ok(serde_json::Value::Null),
        "work_list" => {
            serde_json::to_value(bridge.session()?.work_list()?).map_err(CommandError::from)
        }
        "work_delete" => {
            bridge
                .session()?
                .work_delete(args["id"].as_str().ok_or("id missing")?)?;
            Ok(serde_json::Value::Null)
        }
        "work_restore" => {
            bridge.session()?.work_restore(
                peer,
                args["id"].as_str().ok_or("id missing")?,
                args["revision"].as_u64().ok_or("revision missing")?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "work_abandon" => {
            bridge.session()?.abandon_window_work(
                peer,
                args["generation"].as_str().ok_or("generation missing")?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "work_import" => {
            let record = crate::recovery::work::read_work(std::path::Path::new(
                args["path"].as_str().ok_or("path missing")?,
            ))?;
            let mut s = bridge.session()?;
            s.lifecycle(args["revision"].as_u64().ok_or("revision missing")?)?;
            s.restore_work_record(peer, record)?;
            Ok(serde_json::Value::Null)
        }
        "transition_begin" => serde_json::to_value(bridge.session()?.begin_transition(
            peer,
            args["generation"].as_str().ok_or("generation missing")?,
        )?)
        .map_err(CommandError::from),
        "transition_ack" => {
            bridge
                .session()?
                .acknowledge_transition(peer, serde_json::from_value(args["ack"].clone())?)?;
            Ok(serde_json::Value::Null)
        }
        "transition_status" => serde_json::to_value(
            bridge
                .session()?
                .transition_status(args["id"].as_str().ok_or("id missing")?)?,
        )
        .map_err(CommandError::from),
        "transition_end" => {
            let mut s = bridge.session()?;
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
                &mut *bridge.session()?,
                args["revision"].as_u64().ok_or("revision missing")?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "open_private_recovery" => {
            let generation = args["generation"].as_str().ok_or("generation missing")?;
            Ok(serde_json::json!(
                bridge.session()?.open_private_recovery(peer, generation)?
            ))
        }
        "conserve_private" => {
            let input = serde_json::from_value(args["input"].clone())?;
            Ok(serde_json::json!(
                bridge.session()?.conserve_private(peer, input)?
            ))
        }
        "diagnostic_private_recovery" => {
            let s = bridge.session()?;
            let path = crate::recovery::drafts::path(&s.root, &s.generation, peer)?;
            if path.exists() {
                serde_json::to_value(crate::recovery::drafts::read(&path)?)
                    .map_err(CommandError::from)
            } else {
                Ok(serde_json::Value::Null)
            }
        }

        "diagnostic_agent_authorize" => {
            let s = bridge.session()?;
            s.check_peer(peer, &s.generation)?;
            *bridge.agent.lock()? = agent::AgentState {
                token: Some("isolated-browser-recipe".into()),
                ..Default::default()
            };
            Ok(serde_json::Value::Null)
        }
        "diagnostic_agent" => {
            let mut session = bridge.session()?;
            let mut agent = bridge.agent.lock()?;
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
                        &typstographe_core::content::source_with_metadata(
                            &candidate.after,
                            assets,
                            &candidate.settings.metadata,
                        )?,
                        assets,
                        bridge.session()?.render_context(&candidate.settings),
                    )
                },
            )?;
            serde_json::to_value(publication).map_err(CommandError::from)
        }
        "diagnostic_shared_state" => {
            shared_diagnostic::run(args.clone(), &bridge.session()?.assets)
        }
        "bibliography_index" | "bibliography_search" => {
            let files = serde_json::from_value(args["files"].clone())?;
            serde_json::to_value(if command == "bibliography_index" {
                bibliography::bibliography_index(files)?
            } else {
                bibliography::bibliography_search(
                    files,
                    args["query"].as_str().unwrap_or("").into(),
                )?
            })
            .map_err(CommandError::from)
        }
        "bibliography_query" => bibliography::bibliography_query(
            serde_json::from_value(args["metadata"].clone())?,
            args["operation"].as_str().unwrap_or("").into(),
            serde_json::from_value(
                args.get("document")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            )?,
            args.get("params")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        ),
        "validate_bibliography" => {
            bibliography::validate_bibliography(
                serde_json::from_value(args["document"].clone())?,
                serde_json::from_value(args["metadata"].clone())?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "model_policy" => bibliography::model_policy(
            serde_json::from_value(args["metadata"].clone())?,
            args["manifest"].as_str().unwrap_or("").into(),
        ),
        "sync_forward" | "sync_backward" | "sync_selection" => {
            let revision = args["revision"].as_u64().ok_or("Révision absente.")?;
            let session = bridge.session()?;
            if session.revision != revision {
                return Err("stale".into());
            }
            let cache = bridge.renderer.1.lock()?;
            let sync = cache
                .as_ref()
                .filter(|sync| sync.revision == revision)
                .ok_or("stale")?;
            match command {
                "sync_forward" => Ok(serde_json::json!(
                    sync.forward(args["position"].as_u64().ok_or("Position absente.")? as usize)
                )),
                "sync_backward" => Ok(serde_json::json!(
                    sync.backward(serde_json::from_value(args["location"].clone())?)
                )),
                _ => Ok(serde_json::json!(sync.selection_highlights_exact(
                    args["from"].as_u64().ok_or("Début absent.")? as usize
                        ..args["to"].as_u64().ok_or("Fin absente.")? as usize
                ))),
            }
        }
        "connection" | "set_synchronization" | "leave" => {
            let mut session = bridge.session()?;
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
            serde_json::to_value(session.synchronization()).map_err(CommandError::from)
        }
        "content_catalogue" => Ok(catalogue::content_catalogue()),
        "model_thumbnail" => Ok(serde_json::json!(crate::model_thumbnails::model_thumbnail(
            args["modelId"].as_str().ok_or("modelId missing")?.into()
        )?)),
        "validate_model" => {
            let snapshot: Snapshot = serde_json::from_value(args["snapshot"].clone())?;
            let session = bridge.session()?;
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
            args["fieldLabel"].as_str().map(str::to_owned),
        )),
        "spelling_check" => tauri::async_runtime::block_on(proofing::spelling_check(
            serde_json::from_value(args["segments"].clone())?,
            serde_json::from_value(args["language"].clone())?,
            serde_json::from_value(args["ignored"].clone())?,
        )),
        "spelling_suggestions" => tauri::async_runtime::block_on(proofing::spelling_suggestions(
            serde_json::from_value(args["language"].clone())?,
            args["word"].as_str().unwrap_or("").into(),
        ))
        .map(|words| serde_json::json!(words)),
        "preferences_load" => {
            serde_json::to_value(preferences::Preferences::load(&bridge.session()?.root)?)
                .map_err(CommandError::from)
        }
        "preferences_save" => {
            let preferences: preferences::Preferences =
                serde_json::from_value(args["preferences"].clone())?;
            preferences.save(&bridge.session()?.root)?;
            Ok(serde_json::Value::Null)
        }
        "reset" => {
            let mut session = bridge.session()?;
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
        "open_file" => {
            let path =
                std::path::PathBuf::from(args["path"].as_str().ok_or("Missing project path")?)
                    .canonicalize()?;
            let session_root = bridge.session()?.root.canonicalize()?;
            if !path.starts_with(session_root) {
                return Err("Recipe projects must stay inside the isolated session".into());
            }
            files::open_for_recipe(
                &bridge.session,
                &path,
                args["revision"].as_u64().ok_or("Missing revision")?,
                |source, assets, context| bridge.renderer.validate(source, assets, context),
            )
        }
        "new_document" => {
            let settings = serde_json::from_value(args["settings"].clone())?;
            let revision = args["revision"].as_u64().ok_or("Révision absente.")?;
            files::create(
                &mut *bridge.session()?,
                settings,
                revision,
                serde_json::from_value(
                    args.get("resources")
                        .cloned()
                        .unwrap_or(serde_json::json!([])),
                )?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "join" => {
            bridge.renderer.2.lock()?.detach(peer);
            serde_json::to_value(bridge.session()?.join(peer)).map_err(CommandError::from)
        }
        "commit_structured_commands" => {
            let request = serde_json::from_value(args["request"].clone())?;
            let mut session = bridge.session()?;
            let publication = structured::commit(&mut session, peer, request)?;
            let mut value = serde_json::to_value(publication)?;
            value["broadcast"] = serde_json::json!(session.should_broadcast(peer));
            Ok(value)
        }
        "prepare_structured_commands" => {
            let request = serde_json::from_value(args["request"].clone())?;
            serde_json::to_value(structured::prepare(&*bridge.session()?, request)?)
                .map_err(CommandError::from)
        }
        "relay" => {
            let update: Vec<u8> = serde_json::from_value(args["update"].clone())?;
            let mut session = bridge.session()?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            let receipt = session.relay_view(
                peer,
                &update,
                args["group"].as_u64(),
                args["capture"].as_bool().unwrap_or(true),
                serde_json::from_value(args["historyView"].clone())?,
            )?;
            let mut value = serde_json::to_value(receipt)?;
            value["broadcast"] = serde_json::json!(session.should_broadcast(peer));
            Ok(value)
        }
        "editing_history" => {
            let mut session = bridge.session()?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            let direction = serde_json::from_value(args["direction"].clone())?;
            let mut value = serde_json::to_value(session.history(peer, direction)?)?;
            value["broadcast"] = serde_json::json!(session.should_broadcast(peer));
            Ok(value)
        }

        "editing_context" => {
            let mut session = bridge.session()?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            serde_json::to_value(session.ensure_owner()?.snapshot(peer)?)
                .map_err(CommandError::from)
        }
        "set_editing_context" => {
            let request: owner::ContextRequest = serde_json::from_value(args["request"].clone())?;
            let mut session = bridge.session()?;
            session.check_peer(peer, &request.generation)?;
            serde_json::to_value(session.ensure_owner()?.set_context(peer, request)?)
                .map_err(CommandError::from)
        }
        "reset_editing_context" => {
            let mut session = bridge.session()?;
            session.check_peer(peer, args["generation"].as_str().unwrap_or(""))?;
            serde_json::to_value(session.ensure_owner()?.reset(peer)?).map_err(CommandError::from)
        }
        "awareness" => {
            let update = args["update"].as_array().ok_or("Présence absente")?;
            if update.len() > 64 * 1024 {
                return Err("Présence trop volumineuse.".into());
            }
            let session = bridge.session()?;
            Ok(serde_json::json!(session.accepts_awareness(
                peer,
                args["generation"].as_str().unwrap_or(""),
                args["version"].as_u64().unwrap_or(0)
            )?))
        }
        "prepare_image" => serde_json::to_value(prepare_image(
            args["extension"]
                .as_str()
                .ok_or("Missing extension")?
                .into(),
            args["data"].as_str().ok_or("Missing image data")?.into(),
        )?)
        .map_err(CommandError::from),
        "preview_bind" => Ok(serde_json::json!(crate::preview_pipeline::bind(
            bridge,
            peer,
            args["generation"].as_str().ok_or("Missing generation")?
        )?)),
        "preview_invalidate" => {
            crate::preview_pipeline::invalidate(
                bridge,
                peer,
                &serde_json::from_value(args["scope"].clone())?,
            )?;
            Ok(serde_json::Value::Null)
        }
        "preview_sync" => crate::preview_pipeline::query(
            bridge,
            peer,
            &serde_json::from_value(args["frame"].clone())?,
            args["revision"].as_u64().ok_or("Missing revision")?,
            serde_json::from_value(args["query"].clone())?,
        ),
        "preview_delta" => {
            use base64::Engine as _;
            let bytes = crate::preview_pipeline::render(
                bridge,
                peer,
                serde_json::from_value(args["snapshot"].clone())?,
                serde_json::from_value(args["request"].clone())?,
            )?;
            Ok(
                serde_json::json!({"binary":base64::engine::general_purpose::STANDARD.encode(bytes)}),
            )
        }
        "preview_frame" => {
            use base64::Engine as _;
            let snapshot: Snapshot = serde_json::from_value(args["snapshot"].clone())?;
            let encoding = serde_json::from_value(args["encoding"].clone())?;
            let session = bridge.session()?;
            let source = session.validate(&snapshot)?;
            let map = typstographe_core::content::source_mapped_with_metadata(
                &snapshot.document,
                &session.assets,
                &snapshot
                    .settings
                    .as_ref()
                    .unwrap_or(&session.settings)
                    .metadata,
            )?
            .1;
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
            let snapshot: Snapshot = serde_json::from_value(args["snapshot"].clone())?;
            let session = bridge.session()?;
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
                    typstographe_core::content::source_mapped_with_metadata(
                        &snapshot.document,
                        &session.assets,
                        &snapshot
                            .settings
                            .as_ref()
                            .unwrap_or(&session.settings)
                            .metadata,
                    )?
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
                "preview" => serde_json::to_value(preview).map_err(CommandError::from),
                "save" => {
                    let update = serde_json::from_value(args["update"].clone())?;
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
