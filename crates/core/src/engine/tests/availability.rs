use super::*;
fn state(value: Availability) -> Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn parameter_collection_intents_share_target_checks_with_complete_commands() {
    for (source, range) in [
        ("Été\nNoël", 0..8),
        ("Été\n\nNoël", 0..9),
        ("- Un\n- Deux", 0..7),
        ("Un texte.", 2..2),
    ] {
        let e = engine(source);
        let target = explicit(&e, range);
        let before = e.query(Query::Inspect(Empty {})).unwrap();
        let stored = e.stored_document();
        assert_eq!(
            state(e.available_intent(CommandIntent::Link, target)),
            state(e.available(&Command::SetLink(LinkParams {
                target,
                url: "https://example.org".into()
            })))
        );
        let image = Command::InsertImage(InsertImageParams {
            target,
            image: ImageInput::Path {
                path: "absent-image.png".into(),
            },
            caption: String::new(),
            alt: String::new(),
        });
        assert_eq!(
            state(e.available_intent(CommandIntent::InsertImage, target)),
            state(e.available(&image))
        );
        assert_eq!(e.query(Query::Inspect(Empty {})).unwrap(), before);
        assert_eq!(e.stored_document(), stored);
        assert!(!std::path::Path::new("absent-image.png").exists());
    }
}

#[test]
fn intents_protect_document_scope_foreign_targets_and_export_requirements_without_io() {
    let e = engine("Un texte.");
    let other = engine("Autre.");
    let foreign = explicit(&other, 0..2);
    assert_eq!(
        state(e.available_intent(CommandIntent::Link, foreign))["reason"]["params"]["reason"],
        "foreign_document"
    );
    let empty = Engine::empty();
    for intent in [CommandIntent::New, CommandIntent::Open] {
        assert_eq!(
            state(empty.available_intent(intent, TextTarget::Current {}))["state"],
            "enabled"
        );
    }
    for intent in [
        CommandIntent::Save,
        CommandIntent::SaveAs,
        CommandIntent::Close,
        CommandIntent::Export,
        CommandIntent::Link,
        CommandIntent::InsertImage,
        CommandIntent::SetModel,
    ] {
        assert_eq!(
            state(empty.available_intent(intent, TextTarget::Current {}))["reason"]["params"]["reason"],
            "no_document"
        );
        assert!(
            Command::command_specs()
                .iter()
                .any(|spec| spec.id.as_str() == intent.id())
        );
    }
    let params = ExportParams {
        guard: e.target_guard(),
        directory: "absent-export-directory".into(),
        format: ExportFormat::Both,
        page_images: true,
    };
    assert_eq!(
        state(e.available_intent(CommandIntent::Export, TextTarget::Current {})),
        state(e.available(&Command::Export(params)))
    );
    assert!(!std::path::Path::new("absent-export-directory").exists());
    assert_eq!(
        state(e.available_intent(CommandIntent::Export, TextTarget::Current {}))["reason"]["params"]
            ["reason"],
        "export_required_fields"
    );
}
