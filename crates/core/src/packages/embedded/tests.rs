use super::*;
use std::{collections::BTreeMap, sync::Arc};
pub(crate) fn historical() -> Dependencies {
    let inventory = serde_json::json!({"version":1,"capabilities":[],"packages":[{
        "name":"retired-helper","version":"0.1.0","root":"packages/retired-helper-0.1.0",
        "entrypoint":"main.typ","license":"MIT","archive":"https://example.org/retired.tar.gz",
        "sha256":"0".repeat(64),"dependencies":[]
    }]});
    Arc::new(BTreeMap::from([
        (
            INVENTORY.into(),
            serde_json::to_vec(&inventory).unwrap().into(),
        ),
        (
            "packages/retired-helper-0.1.0/main.typ".into(),
            b"#let greeting = [Pinned helper]".as_slice().into(),
        ),
        (
            "packages/retired-helper-0.1.0/LICENSE".into(),
            b"MIT fixture".as_slice().into(),
        ),
        (
            "packages/PROVENANCE.md".into(),
            b"Synthetic historical package for tests".as_slice().into(),
        ),
    ]))
}
const TEMPLATE: &str = "#import \"packages/retired-helper-0.1.0/main.typ\": greeting";
#[test]
fn pinned_inventory_is_independent_of_the_installed_catalogue() {
    let files = historical();
    validate(TEMPLATE, &files).unwrap();
    assert!(!crate::packages::valid_path(
        "packages/retired-helper-0.1.0/main.typ"
    ));
    for kind in crate::document_types::DocumentType::ALL {
        for model in kind.models() {
            validate(model.source, &crate::packages::for_template(model.source)).unwrap();
        }
    }
}
#[test]
fn paths_inventory_and_transitive_dependencies_are_checked() {
    for path in [
        "/packages/x",
        "packages/../x",
        "packages//x",
        "packages/x\\y",
        "capabilities/C:x",
        "packages/./x",
    ] {
        assert!(!valid_path(path), "{path}");
    }
    for missing in [
        INVENTORY,
        "packages/retired-helper-0.1.0/main.typ",
        "packages/retired-helper-0.1.0/LICENSE",
        "packages/PROVENANCE.md",
    ] {
        let mut files = historical().as_ref().clone();
        files.remove(missing);
        assert!(validate(TEMPLATE, &Arc::new(files)).is_err(), "{missing}");
    }
    for change in ["dependency", "duplicate", "root", "escape", "unknown-file"] {
        let mut files = historical().as_ref().clone();
        let mut inventory: serde_json::Value = serde_json::from_slice(&files[INVENTORY]).unwrap();
        match change {
            "dependency" => {
                inventory["packages"][0]["dependencies"] = serde_json::json!(["absent"])
            }
            "duplicate" => {
                let p = inventory["packages"][0].clone();
                inventory["packages"].as_array_mut().unwrap().push(p);
            }
            "root" => inventory["packages"][0]["root"] = "packages/wrong".into(),
            "escape" => inventory["packages"][0]["entrypoint"] = "../main.typ".into(),
            "unknown-file" => {
                files.insert(
                    "capabilities/unlisted.typ".into(),
                    b"unused".as_slice().into(),
                );
            }
            _ => unreachable!(),
        }
        files.insert(
            INVENTORY.into(),
            serde_json::to_vec(&inventory).unwrap().into(),
        );
        assert!(validate(TEMPLATE, &Arc::new(files)).is_err(), "{change}");
    }
    assert!(validate(TEMPLATE, &Arc::default()).is_err());
    validate("// no dependencies", &Arc::default()).unwrap();
}
