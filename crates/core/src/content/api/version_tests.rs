use super::tests::{new, target};
use super::*;
use crate::content::project::Project;
fn create(history: &Value, name: &str) -> Value {
    json!({"history":history["history"],"action":{"kind":"create","details":{"name":name,"tags":["draft"],"creator":"Camille"}}})
}
#[test]
fn version_commands_save_atomically_without_editing_text_contexts_or_undo_history() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("article.typsto");
    let mut session = new("article");
    session.save_as(&path).unwrap();
    session
        .execute(
            "text.insert",
            json!({"target":target(&[0],0,0),"text":"Original"}),
        )
        .unwrap();
    let guard = session.guard().cloned();
    let context = session.query("context.get", json!({})).unwrap();
    let empty = session.query("versions.list", json!({})).unwrap();
    let first = session
        .execute("versions.change", create(&empty, "v1"))
        .unwrap();
    assert_eq!(session.guard().cloned(), guard);
    assert_eq!(session.query("context.get", json!({})).unwrap(), context);
    assert_eq!(first["versions"][0]["details"]["creator"], "Camille");
    assert_eq!(
        session.query("document.inspect", json!({})).unwrap()["dirty"],
        false
    );
    let id = first["versions"][0]["id"].clone();
    let read = session.query("versions.read", json!({"id":id})).unwrap();
    session
        .execute(
            "text.insert",
            json!({"target":target(&[0],0,0),"text":"Latest "}),
        )
        .unwrap();
    assert_eq!(
        session.query("versions.read", json!({"id":id})).unwrap(),
        read
    );
    assert!(
        session
            .execute("versions.change", create(&empty, "stale"))
            .is_err()
    );
    let before = std::fs::read(&path).unwrap();
    let mut details = first["versions"][0]["details"].clone();
    details["name"] = "renamed".into();
    let updated = session.execute("versions.change",json!({"history":first["history"],"action":{"kind":"update","id":id,"details":details}})).unwrap();
    assert_ne!(updated["history"], first["history"]);
    assert_ne!(before, std::fs::read(&path).unwrap());
    assert_eq!(
        session.query("versions.read", json!({"id":id})).unwrap(),
        read
    );
    details["tags"] = json!(["duplicate", "duplicate"]);
    let saved = std::fs::read(&path).unwrap();
    assert!(session.execute("versions.change",json!({"history":updated["history"],"action":{"kind":"update","id":id,"details":details}})).is_err());
    assert_eq!(saved, std::fs::read(&path).unwrap());
    let deleted = session
        .execute(
            "versions.change",
            json!({"history":updated["history"],"action":{"kind":"delete","id":id}}),
        )
        .unwrap();
    assert_eq!(deleted["versions"], json!([]));
    assert!(session.query("versions.read", json!({"id":id})).is_err());
    assert_eq!(
        Project::read(&path).unwrap().current().state(),
        *session.state().unwrap()
    );
}
#[test]
fn failed_version_save_keeps_the_index_and_file_while_read_only_sessions_cannot_mutate() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("letter.typsto");
    let mut session = new("courrier");
    let empty = session.query("versions.list", json!({})).unwrap();
    assert!(
        session
            .execute("versions.change", create(&empty, "unsaved"))
            .is_err()
    );
    assert_eq!(session.query("versions.list", json!({})).unwrap(), empty);
    session.save_as(&path).unwrap();
    let mut reader = Session::new();
    reader
        .execute("document.open", json!({"path":path,"read_only":true}))
        .unwrap();
    assert_eq!(reader.query("versions.list", json!({})).unwrap(), empty);
    assert!(
        reader
            .execute("versions.change", create(&empty, "forbidden"))
            .is_err()
    );
    std::fs::write(&path, b"external replacement").unwrap();
    assert!(
        session
            .execute("versions.change", create(&empty, "refused"))
            .is_err()
    );
    assert_eq!(session.query("versions.list", json!({})).unwrap(), empty);
    assert_eq!(std::fs::read(&path).unwrap(), b"external replacement");
}
