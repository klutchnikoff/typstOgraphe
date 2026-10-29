//! Prepare a typed edit against a current client checkpoint. The existing relay owns publication.
use crate::session::Session;
use serde::Deserialize;
use typstographe_core::content::{
    commands::Command,
    contract::{Error, Guard},
    shared::{Publication, SharedDocument, SharedSession},
};
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub generation: String,
    pub revision: u64,
    pub update: Vec<u8>,
    pub commands: Vec<Command>,
}
pub fn prepare(session: &Session, request: Request) -> Result<Publication, String> {
    let execute = || {
        let expected = Guard {
            document: request.generation,
            revision: request.revision,
        };
        expected.check(&Guard {
            document: session.generation.clone(),
            revision: session.revision,
        })?;
        let confirmed = SharedDocument::from_updates(&session.updates)?;
        let supplied = SharedDocument::from_update(&request.update)?;
        if !confirmed.same_history(&supplied) {
            return Err(Error::InconsistentState {
                field: "checkpoint".into(),
            });
        }
        confirmed.verify(&supplied.state()?)?;
        let mut candidate =
            SharedSession::open(expected.clone(), &request.update, session.assets.clone())?;
        let prepared = candidate.prepare_commands(&expected, &request.commands)?;
        candidate.publish(prepared)
    };
    execute().map_err(|e| serde_json::to_string(&e).expect("typed error"))
}

/// Recheck and publish under the owning session lock, never as an unguarded client relay.
pub fn commit(session: &mut Session, peer: &str, request: Request) -> Result<Publication, String> {
    session.check_peer(peer, &request.generation)?;
    // Full checkpoint verification and final guarded execution share the owning lock.
    prepare(session, request.clone())?;
    let guard = session.guard();
    let publication = session
        .ensure_owner()?
        .commands(peer, &guard, &request.commands)?;
    session.record(&publication);
    Ok(publication)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typstographe_core::content::{
        commands::{TreeRoot, TreeTarget},
        contract::{Settings, State},
        shared::SharedDocument,
    };
    fn request(session: &Session) -> Request {
        let state = State::new(
            serde_json::from_value(
                serde_json::json!({"type":"doc","content":[{"type":"paragraph"}]}),
            )
            .unwrap(),
            Settings::default(),
        );
        Request {
            generation: session.generation.clone(),
            revision: session.revision,
            update: SharedDocument::from_state(&state).unwrap().update(),
            commands: vec![Command::InsertTable {
                parent: TreeTarget {
                    root: TreeRoot::Body,
                    path: vec![],
                },
                index: 0,
                rows: 2,
                columns: 2,
                header: true,
            }],
        }
    }
    #[test]
    fn preparation_is_guarded_and_does_not_publish_or_touch_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::fresh(dir.path().into());
        let initial = request(&session);
        session.updates.push(initial.update.clone());
        let prepared = prepare(&session, initial.clone()).unwrap();
        assert!(prepared.changed);
        assert_eq!(prepared.state.document.content[0].kind, "tableFigure");
        assert_eq!(session.revision, 0);
        assert_eq!(session.updates.len(), 1);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        let mut wrong = request(&session);
        wrong.generation = "another".into();
        assert!(
            prepare(&session, wrong)
                .unwrap_err()
                .contains("wrong_document")
        );
        let mut stale = request(&session);
        stale.revision += 1;
        assert!(
            prepare(&session, stale)
                .unwrap_err()
                .contains("stale_revision")
        );
        assert!(
            prepare(&session, request(&session))
                .unwrap_err()
                .contains("inconsistent_state")
        );
        let mut invalid = initial.clone();
        invalid.commands.push(Command::RemoveTableColumn {
            target: TreeTarget {
                root: TreeRoot::Body,
                path: vec![0],
            },
            index: 99,
        });
        assert!(
            prepare(&session, invalid)
                .unwrap_err()
                .contains("batch_failed")
        );
        assert_eq!(session.updates.len(), 1);
    }
    #[test]
    fn final_commit_rechecks_the_owner_and_rejects_a_prepared_competitor() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::fresh(dir.path().into());
        session.join("main");
        let request = request(&session);
        session.updates.push(request.update.clone());
        let prepared = prepare(&session, request.clone()).unwrap();
        assert!(prepared.changed);
        let committed = commit(&mut session, "main", request.clone()).unwrap();
        assert_eq!(committed.guard.revision, 1);
        let confirmed = session.updates.clone();
        assert!(
            commit(&mut session, "main", request)
                .unwrap_err()
                .contains("stale_revision")
        );
        assert_eq!(session.updates, confirmed);
    }
    #[test]
    fn prepared_images_stay_private_and_stale_commits_cannot_admit_them() {
        use typstographe_core::content::commands::ImageResource;
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::fresh(dir.path().into());
        session.join("main");
        let mut request = request(&session);
        session.updates.push(request.update.clone());
        let resource =
            ImageResource::prepare("svg", include_bytes!("../../../fixtures/parc.svg").to_vec())
                .unwrap();
        request.commands = vec![
            Command::AddResource {
                resource: resource.clone(),
            },
            Command::SetSignature {
                path: Some(resource.path.clone()),
            },
        ];
        assert_eq!(
            prepare(&session, request.clone()).unwrap().resources,
            vec![resource.clone()]
        );
        assert!(session.assets.is_empty());
        let mut competitor = request.clone();
        competitor.commands = vec![Command::SetField {
            target: typstographe_core::content::commands::FieldTarget::Option {
                name: typstographe_core::content::commands::OptionName::Paper,
            },
            value: serde_json::json!("a5"),
        }];
        commit(&mut session, "main", competitor).unwrap();
        assert!(
            commit(&mut session, "main", request.clone())
                .unwrap_err()
                .contains("stale_revision")
        );
        assert!(session.assets.is_empty());
        request.revision = session.revision;
        request.update = session.ensure_owner().unwrap().update_bytes();
        commit(&mut session, "main", request).unwrap();
        assert!(session.assets.contains_key(&resource.path));
        session
            .history("main", typstographe_core::content::editing::Direction::Undo)
            .unwrap();
        assert!(session.assets.contains_key(&resource.path));
        let state = SharedDocument::from_updates(&session.updates)
            .unwrap()
            .state()
            .unwrap();
        assert!(state.settings.metadata.signature_path().is_none());
    }
}
