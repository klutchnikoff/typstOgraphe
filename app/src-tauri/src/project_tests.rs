use crate::{
    project::*,
    rendering::Renderer,
    session::{Checkpoint, Session},
};
use typstographe_core::{
    content::{
        project::{SnapshotDetails, SnapshotReason},
        shared::SharedDocument,
    },
    document_types::{DocumentType, Metadata},
};

#[test]
fn native_preview_policy_export_and_recovery_use_pinned_resources() {
    let state = State::new(serde_json::from_value(serde_json::json!({"type":"doc","content":[{"type":"paragraph","content":[{"type":"text","text":"Courrier de test"}]}]})).unwrap(), Default::default());
    let mut current = archive(
        state,
        vec![],
        Assets::default(),
        typstographe_core::packages::for_template(&DocumentType::Courrier.default_model()),
    );
    current.metadata = Metadata::demo(DocumentType::Courrier);
    current.template = current
        .template
        .replace("capabilities/dates.typ", "capabilities/pinned-dates.typ");
    let mut dependencies = (*current.dependencies).clone();
    let dates = dependencies.remove("capabilities/dates.typ").unwrap();
    dependencies.insert("capabilities/pinned-dates.typ".into(), dates);
    let mut inventory: serde_json::Value =
        serde_json::from_slice(&dependencies["dependencies.json"]).unwrap();
    for capability in inventory["capabilities"].as_array_mut().unwrap() {
        if capability["id"] == "dates" {
            capability["entrypoint"] = "capabilities/pinned-dates.typ".into();
        }
    }
    dependencies.insert(
        "dependencies.json".into(),
        serde_json::to_vec(&inventory).unwrap().into(),
    );
    current.dependencies = std::sync::Arc::new(dependencies);
    current.yjs = SharedDocument::from_state(&current.state())
        .unwrap()
        .update();
    let owner = Project::new(current.clone())
        .unwrap()
        .prepare_version(SnapshotDetails::default(), SnapshotReason::Manual)
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    let checkpoint = Checkpoint {
        project: Context::new(owner.clone(), true)
            .recovery(current.clone())
            .unwrap(),
        schema: "typstographe-web-editor-v2".into(),
        document: current.document.clone(),
        source: current.validate().unwrap(),
        update: current.yjs.clone(),
        resources: vec![],
        settings: current.state().settings,
    };
    let restored = Session::from_checkpoint(root.path().join("session"), checkpoint).unwrap();
    crate::bibliography::policy(&restored, current.template.clone()).unwrap();
    let renderer = Renderer::default();
    let source = current.validate().unwrap();
    renderer
        .render(
            source.clone(),
            current.assets.clone(),
            0,
            None,
            Some(restored.render_context(&restored.settings)),
            None,
        )
        .unwrap();
    let export = root.path().join("export");
    renderer
        .render(
            source,
            current.assets.clone(),
            0,
            Some(export.clone()),
            Some(restored.render_context(&restored.settings)),
            None,
        )
        .unwrap();
    assert_eq!(
        std::fs::read(export.join("capabilities/pinned-dates.typ")).unwrap(),
        current.dependencies["capabilities/pinned-dates.typ"].as_ref()
    );
    assert_eq!(
        restored.project.get().unwrap().owner.versions(),
        owner.versions()
    );
}
