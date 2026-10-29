use super::{
    command_tests::{command, execute, fields, session},
    *,
};
use crate::{
    content::{
        commands::{Command, ImageResource},
        editing::{Direction, EditingSession},
    },
    document_types::DocumentType,
};
use serde_json::json;
use yrs::Map;
fn resource() -> ImageResource {
    ImageResource::prepare(
        "svg",
        include_bytes!("../../../../../fixtures/parc.svg").to_vec(),
    )
    .unwrap()
}
#[test]
fn resource_and_signature_are_atomic_immutable_and_retained_through_history() {
    let s = session(DocumentType::Courrier);
    let mut owner =
        EditingSession::open(s.guard().clone(), &s.update(), Assets::default()).unwrap();
    let context = owner.open_context(&owner.guard().clone()).unwrap();
    let image = resource();
    let batch = vec![
        Command::AddResource {
            resource: image.clone(),
        },
        Command::SetSignature {
            path: Some(image.path.clone()),
        },
    ];
    let result = owner
        .execute(&owner.guard().clone(), context, &batch)
        .unwrap();
    assert_eq!(result.resources, vec![image.clone()]);
    assert_eq!(
        owner
            .context(&owner.guard().clone(), context)
            .unwrap()
            .undo_steps,
        1
    );
    owner
        .history(&owner.guard().clone(), context, Direction::Undo)
        .unwrap();
    assert!(owner.state().settings.metadata.signature_path().is_none());
    assert!(owner.assets().contains_key(&image.path));
    owner
        .history(&owner.guard().clone(), context, Direction::Redo)
        .unwrap();
    assert_eq!(
        owner.state().settings.metadata.signature_path(),
        Some(image.path.as_str())
    );
    let before = owner.update();
    let mut changed = image.clone();
    changed.data.push(b' ');
    assert!(
        owner
            .execute(
                &owner.guard().clone(),
                context,
                &[Command::AddResource { resource: changed }]
            )
            .is_err()
    );
    assert_eq!(before, owner.update());
    let mut other = image.clone();
    other.path = "images/new.svg".into();
    assert!(
        owner
            .execute(
                &owner.guard().clone(),
                context,
                &[
                    Command::AddResource {
                        resource: other.clone()
                    },
                    Command::SetSignature {
                        path: Some("images/missing.svg".into())
                    }
                ]
            )
            .is_err()
    );
    assert!(!owner.assets().contains_key(&other.path));
    assert_eq!(before, owner.update());
}
#[test]
fn options_validate_and_language_creates_only_missing_primary_records_in_one_batch() {
    let mut s = session(DocumentType::Article);
    let before = s.state().document.clone();
    for (name, value) in [
        ("paper", json!("a5")),
        ("landscape", json!(true)),
        ("language", json!("en")),
        ("bullet_marker", json!("—")),
    ] {
        execute(
            &mut s,
            vec![command(
                "metadata.set",
                json!({"target":{"kind":"option","name":name},"value":value}),
            )],
        );
        assert_eq!(fields(&s)["options"][name], value);
    }
    assert_eq!(s.state().document, before);
    assert_eq!(fields(&s)["authors"][0]["name"], "Camille");
    let names = fields(&s)["localizations"].clone();
    assert!(
        names
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["language"] == "en")
    );
    let current = s.update();
    for (name, value) in [
        ("paper", json!("a0")),
        ("landscape", json!("true")),
        ("language", json!("zz")),
        ("bullet_marker", json!("xx")),
    ] {
        assert!(
            s.prepare_commands(
                s.guard(),
                &[command(
                    "metadata.set",
                    json!({"target":{"kind":"option","name":name},"value":value})
                )]
            )
            .is_err()
        );
        assert_eq!(s.update(), current);
    }
}
#[test]
fn models_validate_offline_without_rebuilding_collections_and_failure_publishes_nothing() {
    use yrs::types::SharedRef;
    let mut s = session(DocumentType::Article);
    let shared = &s.shared.doc;
    let collections = shared
        .get_or_insert_map("settings")
        .get(&shared.transact(), "collections")
        .unwrap();
    let yrs::Out::YMap(collections) = collections else {
        panic!()
    };
    let identity = collections.hook();
    let before = fields(&s);
    let source = format!("{}\n// project-owned model", s.state().settings.template);
    execute(
        &mut s,
        vec![Command::SetModel {
            model_id: "article.custom".into(),
            source: source.clone(),
        }],
    );
    assert_eq!(fields(&s), before);
    let out = s
        .shared
        .doc
        .get_or_insert_map("settings")
        .get(&s.shared.doc.transact(), "collections")
        .unwrap();
    let yrs::Out::YMap(map) = out else { panic!() };
    assert_eq!(map.hook(), identity);
    let before = s.update();
    let guard = s.guard().clone();
    assert!(
        s.prepare_commands(
            &guard,
            &[
                Command::AddResource {
                    resource: resource()
                },
                Command::SetModel {
                    model_id: "broken".into(),
                    source: "#let nothing = 1".into()
                }
            ]
        )
        .is_err()
    );
    assert_eq!(s.update(), before);
    assert!(s.assets().is_empty());
    assert_eq!(s.guard(), &guard);
    assert!(
        s.prepare_commands(
            &guard,
            &[Command::SetModel {
                model_id: "courrier.model.personal".into(),
                source: DocumentType::Courrier.default_model()
            }]
        )
        .is_err()
    );
}
