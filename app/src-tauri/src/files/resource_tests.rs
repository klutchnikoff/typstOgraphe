use super::*;
use typstographe_core::content::commands::{Command, ImageResource, TreeRoot, TreeTarget};

fn execute(f: &Fixture, commands: &[Command]) {
    let mut s = f.session.lock().unwrap();
    let guard = s.guard();
    let publication = s
        .ensure_owner()
        .unwrap()
        .commands("human", &guard, commands)
        .unwrap();
    s.record(&publication);
}
fn resource(path: &str) -> ImageResource {
    ImageResource {
        path: path.into(),
        data: include_bytes!("../../../../fixtures/signature.svg").to_vec(),
    }
}
#[test]
fn images_and_signature_survive_save_reopen_removal_undo_and_redo_without_unused_assets() {
    use typstographe_core::content::editing::Direction;
    let f = Fixture::new();
    let image = resource("images/signature.svg");
    execute(&f, &[
        Command::AddResource { resource: image.clone() },
        Command::AddResource { resource: resource("images/unused.svg") },
        Command::InsertBlock { parent: TreeTarget { root: TreeRoot::Body, path: vec![] }, index: 1,
            node: serde_json::from_value(json!({"type":"figure","attrs":{"src":image.path,"alt":"Signature manuscrite"},"content":[{"type":"text","text":"Légende 😀"}]})).unwrap() },
        Command::SetSignature { path: Some(image.path.clone()) },
    ]);
    f.persist_draft();
    f.save(&f.path());
    let original = Archive::read(&f.path()).unwrap();
    assert_eq!(original.assets.len(), 1);
    assert_eq!(original.assets[&image.path].as_ref(), image.data);
    assert_eq!(
        original.metadata.signature_path(),
        Some(image.path.as_str())
    );
    execute(
        &f,
        &[
            Command::RemoveBlock {
                target: TreeTarget {
                    root: TreeRoot::Body,
                    path: vec![1],
                },
            },
            Command::SetSignature { path: None },
        ],
    );
    f.save(&f.path());
    assert!(Archive::read(&f.path()).unwrap().assets.is_empty());
    assert!(f.session.lock().unwrap().assets.contains_key(&image.path));
    f.session
        .lock()
        .unwrap()
        .history("human", Direction::Undo)
        .unwrap();
    f.save(&f.path());
    assert_eq!(Archive::read(&f.path()).unwrap().state(), original.state());
    f.session
        .lock()
        .unwrap()
        .history("human", Direction::Redo)
        .unwrap();
    f.save(&f.path());
    assert!(Archive::read(&f.path()).unwrap().assets.is_empty());
    f.session
        .lock()
        .unwrap()
        .history("human", Direction::Undo)
        .unwrap();
    f.save(&f.path());
    f.session.lock().unwrap().file_lease = None;
    let target = Mutex::new(Session::managed(f.directory.path().join("reopened")).unwrap());
    let result = open_project(&target, &f.path(), 0, |source, assets, settings| {
        Renderer::default().validate(source, assets, settings)
    })
    .unwrap();
    assert_eq!(result["resources"].as_array().unwrap().len(), 1);
    let decoded: crate::session::Resource =
        serde_json::from_value(result["resources"][0].clone()).unwrap();
    assert_eq!(decoded.decoded().unwrap(), image.data);
    let mut restored = target.lock().unwrap();
    assert_eq!(restored.ensure_owner().unwrap().state(), &original.state());
}
#[test]
fn article_reopening_preserves_original_bibliography_image_caption_and_citation() {
    use typstographe_core::document_types::{DocumentType, Metadata};
    let f = Fixture::new();
    let mut metadata = Metadata::demo(DocumentType::Article);
    let Metadata::Article(article) = &mut metadata else {
        unreachable!()
    };
    let contents = "@book{martin2026, author={Martin, Camille}, title={Étude des archives}, year={2026}}\n% Conserver aussi ce commentaire\n";
    article.references = vec![typstographe_core::bibliography::ReferenceFile {
        id: "sources".into(),
        name: "archives.bib".into(),
        contents: contents.into(),
    }];
    let state = State::new(serde_json::from_value(json!({"type":"doc","content":[
        {"type":"paragraph","content":[{"type":"text","text":"Une référence "},{"type":"citation","attrs":{"items":[{"file_id":"sources","key":"martin2026"}],"form":"normal"}}]},
        {"type":"figure","attrs":{"src":"images/signature.svg","alt":"Illustration"},"content":[{"type":"text","text":"Légende"}]},
        {"type":"paragraph"}
    ]})).unwrap(), Settings { metadata, template:DocumentType::Article.default_model(), model_id:DocumentType::Article.models().next().unwrap().model.id().into() });
    let mut assets = Assets::default();
    assets.insert(
        "images/signature.svg".into(),
        resource("images/signature.svg").data.into(),
    );
    f.session.lock().unwrap().replace(
        state.document.clone(),
        state.settings.clone(),
        assets,
        SharedDocument::from_state(&state).unwrap().update(),
    );
    f.session.lock().unwrap().ensure_owner().unwrap();
    f.persist_draft();
    f.save(&f.path());
    f.session.lock().unwrap().file_lease = None;
    let reopened = Mutex::new(Session::managed(f.directory.path().join("article-opened")).unwrap());
    open_project(&reopened, &f.path(), 0, |source, assets, settings| {
        Renderer::default().validate(source, assets, settings)
    })
    .unwrap();
    let mut reopened = reopened.lock().unwrap();
    assert_eq!(reopened.ensure_owner().unwrap().state(), &state);
    let Metadata::Article(article) = &reopened.settings.metadata else {
        unreachable!()
    };
    assert_eq!(article.references[0].contents, contents);
    assert_eq!(reopened.assets.len(), 1);
}
