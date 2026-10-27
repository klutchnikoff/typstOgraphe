//! Probe the real archive API without adding Yrs to core or the application.
use base64::{Engine as _, engine::general_purpose::STANDARD};
use typstographe_core::{DocumentLease, content::archive::Archive, rendering::TypstWorld};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mut archive = Archive::read(std::path::Path::new(&args[1])).unwrap();
    let edited: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    archive.document = serde_json::from_value(edited["document"].clone()).unwrap();
    archive.metadata = serde_json::from_value(edited["metadata"].clone()).unwrap();
    archive.yjs = STANDARD.decode(edited["state"].as_str().unwrap()).unwrap();
    let destination = std::path::Path::new(&args[3]);
    archive
        .save(&mut DocumentLease::acquire(destination).unwrap())
        .unwrap();
    let restored = Archive::read(destination).unwrap();
    let mut world = TypstWorld::default();
    world.set_dependencies(&restored.dependencies);
    world.set_assets(&restored.assets);
    let document = world
        .compile_with_template(
            &restored.validate().unwrap(),
            &restored.metadata,
            &restored.template,
        )
        .unwrap();
    std::fs::write(
        &args[4],
        serde_json::to_vec(&serde_json::json!({
            "document": restored.document, "metadata": restored.metadata,
            "state": STANDARD.encode(&restored.yjs), "pages": document.pages().len(),
        }))
        .unwrap(),
    )
    .unwrap();
    println!(
        "archive saved, reopened and compiled offline: {} pages",
        document.pages().len()
    );
}
